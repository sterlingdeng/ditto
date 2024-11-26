use std::time::Duration;

use serde::Deserialize;
use serde_with::{serde_as, DurationSeconds};
use ts_core::{ApplyConfig, Output, PortRange, Protocol, TrafficConfig};

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
    pub src_ports: Option<(u16, u16)>,
    pub dst_ports: Option<(u16, u16)>,
    pub report_output: Option<Output>,
}

impl Into<TrafficConfig> for Config {
    fn into(self) -> TrafficConfig {
        ts_core::TrafficConfig {
            packet_loss: self.packet_loss,
            latency: self.latency,
            max_bandwidth: self.bandwidth,
            protocol: self.protocol,
            src_ports: self.src_ports.map(|(start, end)| PortRange { start, end }),
            dst_ports: self.dst_ports.map(|(start, end)| PortRange { start, end }),
            report_output: self.report_output.map_or(Output::None, |v| v),
        }
    }
}

#[serde_as]
#[derive(Deserialize, Clone, Debug)]
pub struct Events {
    #[serde_as(as = "DurationSeconds<u64>")]
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
