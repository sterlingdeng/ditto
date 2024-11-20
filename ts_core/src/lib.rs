use std::net::IpAddr;
use thiserror::Error;

mod commands;
mod rules;

use commands::{DnctlCommands, PfctlCommands};
use rules::RuleGenerator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
    Both,
}

#[derive(Debug, Clone)]
pub struct TrafficConfig {
    /// Packet loss percentage (0.0 to 100.0)
    pub packet_loss: f32,
    /// Latency in milliseconds
    pub latency: u32,
    /// Maximum bandwidth in bits per second
    pub max_bandwidth: u64,
    /// Target protocol (TCP, UDP, or both)
    pub protocol: Protocol,
    /// Target IP address
    pub target_address: Option<IpAddr>,
    /// Target port range
    pub target_ports: Option<PortRange>,
}

#[derive(Debug, Clone)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

#[derive(Error, Debug)]
pub enum TrafficShapingError {
    #[error("Invalid packet loss percentage: {0}. Must be between 0 and 100")]
    InvalidPacketLoss(f32),
    #[error("Invalid port range: start ({start}) must be less than or equal to end ({end})")]
    InvalidPortRange { start: u16, end: u16 },
    #[error("Command execution failed: {0}")]
    CommandError(String),
    #[error("System error: {0}")]
    SystemError(#[from] std::io::Error),
}

impl TrafficConfig {
    /// Creates a new TrafficConfig with validation
    pub fn new(
        packet_loss: f32,
        latency: u32,
        max_bandwidth: u64,
        protocol: Protocol,
        target_address: Option<IpAddr>,
        target_ports: Option<(u16, u16)>,
    ) -> Result<Self, TrafficShapingError> {
        // Validate packet loss
        if !(0.0..=100.0).contains(&packet_loss) {
            return Err(TrafficShapingError::InvalidPacketLoss(packet_loss));
        }

        // Validate port range if provided
        let target_ports = if let Some((start, end)) = target_ports {
            if start > end {
                return Err(TrafficShapingError::InvalidPortRange { start, end });
            }
            Some(PortRange { start, end })
        } else {
            None
        };

        Ok(Self {
            packet_loss,
            latency,
            max_bandwidth,
            protocol,
            target_address,
            target_ports,
        })
    }
}

const DEFAULT_PIPE_NUMBER: u32 = 1;

/// Main traffic shaper struct that handles the configuration and execution
pub struct TrafficShaper {
    config: TrafficConfig,
}

impl TrafficShaper {
    pub fn new(config: TrafficConfig) -> Self {
        Self { config }
    }

    /// Applies the traffic shaping rules
    pub fn apply(&self) -> Result<(), TrafficShapingError> {
        // Step 1: Enable PF if not already enabled
        PfctlCommands::enable()?;

        // Step 2: Configure dummynet pipe with the specified configuration
        // The pipe will be created if it doesn't exist, or updated if it does
        DnctlCommands::configure_pipe(
            DEFAULT_PIPE_NUMBER,
            Some(self.config.max_bandwidth),
            Some(self.config.latency),
            Some(self.config.packet_loss / 100.0), // Convert percentage to ratio
        )?;

        // Step 3: Generate and load PF rules only if the pipe didn't exist
        if !DnctlCommands::pipe_exists(DEFAULT_PIPE_NUMBER)? {
            let rules = RuleGenerator::generate_pf_rules(&self.config, DEFAULT_PIPE_NUMBER)?;
            PfctlCommands::load_rules(&rules)?;
        }

        Ok(())
    }

    /// Removes traffic shaping rules and restores original configuration
    pub fn cleanup(&self) -> Result<(), TrafficShapingError> {
        // Clean up dummynet pipes
        DnctlCommands::flush_pipes()?;

        // Restore original PF rules
        PfctlCommands::restore_original_rules()?;

        // Disable PF if no other references exist
        PfctlCommands::disable()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traffic_config_validation() {
        // Test valid config
        let config =
            TrafficConfig::new(50.0, 100, 1_000_000, Protocol::Tcp, None, Some((80, 8080)));
        assert!(config.is_ok());

        // Test invalid packet loss
        let config =
            TrafficConfig::new(101.0, 100, 1_000_000, Protocol::Tcp, None, Some((80, 8080)));
        assert!(config.is_err());

        // Test invalid port range
        let config =
            TrafficConfig::new(50.0, 100, 1_000_000, Protocol::Tcp, None, Some((8080, 80)));
        assert!(config.is_err());
    }
}
