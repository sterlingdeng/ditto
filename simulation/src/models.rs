use std::net::IpAddr;
use std::time::Duration;

use ts_core::{ApplyConfig, PortRange, Protocol, TrafficConfig};

use serde::{de::Visitor, Deserialize};

#[derive(Deserialize, Clone)]
pub struct Manifest {
    pub config: Config,
    pub events: Vec<Events>,
}

#[derive(Deserialize, Clone)]
pub struct Config {
    pub packet_loss: f32,
    pub latency: u32,
    pub bandwidth: u64,
    pub protocol: Protocol,
    pub target_address: Option<IpAddr>,
    pub port_range: Option<(u16, u16)>,
}

impl Into<TrafficConfig> for Config {
    fn into(self) -> TrafficConfig {
        ts_core::TrafficConfig {
            packet_loss: self.packet_loss,
            latency: self.latency,
            max_bandwidth: self.bandwidth,
            protocol: self.protocol,
            target_address: self.target_address,
            target_ports: self.port_range.map(|(start, end)| PortRange { start, end }),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Events {
    pub time: Duration,
    pub latency: u32,
    pub bandwidth: u64,
    pub packet_loss: f32,
}

impl Into<ApplyConfig> for Events {
    fn into(self) -> ApplyConfig {
        ApplyConfig {
            packet_loss: self.packet_loss,
            latency: self.latency,
            max_bandwidth: self.bandwidth,
        }
    }
}
