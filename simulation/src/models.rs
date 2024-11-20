use std::net::IpAddr;

use ts_core::Protocol;

use serde::{de::Visitor, Deserialize, Serialize};

struct Manifest {
    config: Config,
    events: Vec<Events>,
}

struct Config {
    packet_loss: f32,
    latency: u32,
    bandwidth: u64,
    protocol: Protocol,
    target_address: Option<IpAddr>,
    port_range: Option<(u16, u16)>,
}

struct Events {
    time: u32,
    latency: u32,
    bandwidth: u64,
    packet_loss: f32,
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ConfigVisitor;

        impl<'de> Visitor<'de> for ConfigVisitor {
            type Value = Config;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Config")
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                while let Some(key) = map.next_key()? {
                    match key {
                        1 => {}
                    }
                }
                Ok(Config {})
            }
        }
        deserializer.deserialize_struct("Config");
    }
}
