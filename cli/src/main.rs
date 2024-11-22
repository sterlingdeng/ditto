use std::fs;
use std::process;

use clap::{Parser, Subcommand};
use serde_json::from_str;
use simulation::models::Manifest;
use simulation::Simulation;
use tracing::{error, info};
use ts_core::{PortRange, Protocol, TrafficConfig, TrafficShaper};

#[derive(Parser)]
#[command(name = "traffic-shaper")]
#[command(about = "A CLI tool for traffic shaping on macOS", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start traffic shaping with the specified configuration
    Start {
        /// Packet loss percentage (0.0 to 100.0)
        #[arg(long, value_parser = validate_percentage)]
        packet_loss: f32,

        /// Additional latency in milliseconds
        #[arg(long)]
        latency: u32,

        /// Maximum bandwidth in bits per second
        #[arg(long)]
        bandwidth: u64,

        /// Target protocol (tcp, udp, or both)
        #[arg(long, value_parser = parse_protocol)]
        protocol: Protocol,

        /// Optional target port range (format: start-end, e.g., 80-8080)
        #[arg(long, value_parser = parse_port_range)]
        src_ports: Option<(u16, u16)>,

        /// Optional target port range (format: start-end, e.g., 80-8080)
        #[arg(long, value_parser = parse_port_range)]
        dst_ports: Option<(u16, u16)>,
    },
    /// Stop traffic shaping and restore original configuration
    Stop,

    Simulation {
        #[arg(long)]
        manifest_path: String,
    },
}

fn validate_percentage(s: &str) -> Result<f32, String> {
    let value: f32 = s.parse().map_err(|_| "Invalid percentage value")?;
    if !(0.0..=100.0).contains(&value) {
        return Err("Percentage must be between 0 and 100".to_string());
    }
    Ok(value)
}

fn parse_protocol(s: &str) -> Result<Protocol, String> {
    match s.to_lowercase().as_str() {
        "tcp" => Ok(Protocol::Tcp),
        "udp" => Ok(Protocol::Udp),
        "both" => Ok(Protocol::Both),
        _ => Err("Protocol must be one of: tcp, udp, both".to_string()),
    }
}

fn parse_port_range(s: &str) -> Result<(u16, u16), String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return Err("Port range must be in format: start-end".to_string());
    }

    let start: u16 = parts[0].parse().map_err(|_| "Invalid start port number")?;
    let end: u16 = parts[1].parse().map_err(|_| "Invalid end port number")?;

    if start > end {
        return Err("Start port must be less than or equal to end port".to_string());
    }

    Ok((start, end))
}

fn check_root_access() -> bool {
    // Try to access a root-only file
    fs::metadata("/etc/pf.conf").is_ok()
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    )
    .unwrap();

    // Parse command line arguments
    let cli = Cli::parse();

    // Check if we have root access
    if !check_root_access() {
        error!("This program must be run with root privileges");
        process::exit(1);
    }

    match cli.command {
        Commands::Start {
            packet_loss,
            latency,
            bandwidth,
            protocol,
            src_ports,
            dst_ports,
        } => {
            info!("Starting traffic shaping...");

            // Create traffic shaping configuration
            let config = match TrafficConfig::new(
                packet_loss,
                latency,
                bandwidth,
                protocol,
                src_ports.map(|(start, end)| PortRange { start, end }),
                dst_ports.map(|(start, end)| PortRange { start, end }),
            ) {
                Ok(config) => config,
                Err(e) => {
                    error!("Failed to create configuration: {}", e);
                    process::exit(1);
                }
            };

            // Apply traffic shaping
            let shaper = TrafficShaper::new(config);
            if let Err(e) = shaper.enable() {
                error!("Failed to apply traffic shaping: {}", e);
                process::exit(1);
            }

            info!("Traffic shaping started successfully");
        }
        Commands::Stop => {
            info!("Stopping traffic shaping...");

            // Create a dummy config just to use the cleanup functionality
            let config = match TrafficConfig::new(0.0, 0, 0, Protocol::Both, None, None) {
                Ok(config) => config,
                Err(e) => {
                    error!("Failed to create configuration: {}", e);
                    process::exit(1);
                }
            };

            let shaper = TrafficShaper::new(config);
            if let Err(e) = shaper.cleanup() {
                error!("Failed to stop traffic shaping: {}", e);
                process::exit(1);
            }

            info!("Traffic shaping stopped successfully");
        }
        Commands::Simulation { manifest_path } => {
            use std::fs;
            use std::time::Instant;
            let contents = fs::read_to_string(manifest_path).expect("failed to open manifest path");
            let manifest: Manifest = from_str(contents.as_str()).unwrap();
            let mut simulation = Simulation::new(manifest, Instant::now());

            let join = tokio::spawn(async move { simulation.start().await });
            if let Err(e) = join.await {
                eprintln!("error after simulation: {}", e);
            }
        }
    }
}
