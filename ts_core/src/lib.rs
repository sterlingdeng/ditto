use serde::Deserialize;
use thiserror::Error;
use tracing::info;

mod commands;
use commands::{DnctlCommands, PfctlCommands};

mod rules;
use rules::RuleGenerator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
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

    pub src_ports: Option<PortRange>,
    pub dst_ports: Option<PortRange>,
}

#[derive(Debug, Clone)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

#[derive(Debug, Clone)]
pub struct ApplyConfig {
    /// Packet loss percentage (0.0 to 100.0)
    pub packet_loss: f32,
    /// Latency in milliseconds
    pub latency: u32,
    /// Maximum bandwidth in bits per second
    pub max_bandwidth: u64,
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
        src_ports: Option<PortRange>,
        dst_ports: Option<PortRange>,
    ) -> Result<Self, TrafficShapingError> {
        // Validate packet loss
        if !(0.0..=100.0).contains(&packet_loss) {
            return Err(TrafficShapingError::InvalidPacketLoss(packet_loss));
        }

        Ok(Self {
            packet_loss,
            latency,
            max_bandwidth,
            protocol,
            src_ports,
            dst_ports,
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
    pub fn enable(&self) -> Result<(), TrafficShapingError> {
        // Step 1: Enable PF if not already enabled
        PfctlCommands::enable()?;
        info!("pfctl enabled");

        // Step 2: Configure dummynet pipe with the specified configuration
        // The pipe will be created if it doesn't exist, or updated if it does
        DnctlCommands::configure_pipe(
            DEFAULT_PIPE_NUMBER,
            Some(self.config.max_bandwidth),
            Some(self.config.latency),
            Some(self.config.packet_loss / 100.0), // Convert percentage to ratio
        )?;
        info!("configured pipe");

        // Step 3: Generate and load PF rules only if the pipe didn't exist
        if !DnctlCommands::pipe_exists(DEFAULT_PIPE_NUMBER)? {
            let anchor_name = String::from("traffic_shaper");
            let anchor_rules = RuleGenerator::generate_anchor_rules(&anchor_name)?;
            PfctlCommands::load_rules(&anchor_rules, Some(&anchor_name))?;
            info!("loaded anchor rules");

            let rules = RuleGenerator::generate_pf_rules(&self.config, DEFAULT_PIPE_NUMBER)?;
            PfctlCommands::load_rules(&rules, None)?;
            info!("loaded pf rules");
        }

        Ok(())
    }

    pub fn apply(&self, config: ApplyConfig) -> Result<(), TrafficShapingError> {
        DnctlCommands::configure_pipe(
            DEFAULT_PIPE_NUMBER,
            Some(config.max_bandwidth),
            Some(config.latency),
            Some(config.packet_loss / 100.0),
        )?;
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
