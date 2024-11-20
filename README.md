# Traffic Shaper

A Rust library for traffic shaping on macOS using pf (Packet Filter) and dummynet. This library allows you to simulate various network conditions by controlling:

- Packet loss
- Latency
- Maximum bandwidth
- Target protocols (TCP/UDP)
- Target addresses
- Target ports

## Prerequisites

- macOS (the library uses macOS-specific networking tools)
- Administrative privileges (required for pfctl and dnctl commands)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
traffic_shaper = "0.1.0"
```

## Usage

```rust
use std::net::IpAddr;
use traffic_shaper::{TrafficConfig, TrafficShaper, Protocol};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a configuration
    let config = TrafficConfig::new(
        5.0,                // 5% packet loss
        100,               // 100ms latency
        1_000_000,         // 1 Mbps bandwidth
        Protocol::Both,    // Apply to both TCP and UDP
        None,              // Apply to all addresses
        Some((80, 8080)), // Apply to ports 80-8080
    )?;

    // Create and apply traffic shaping rules
    let shaper = TrafficShaper::new(config);
    shaper.apply().await?;

    // ... your application code ...

    // Clean up when done
    shaper.cleanup().await?;

    Ok(())
}
```

## Configuration Options

- `packet_loss`: Percentage of packets to drop (0.0 to 100.0)
- `latency`: Additional latency in milliseconds
- `max_bandwidth`: Maximum bandwidth in bits per second
- `protocol`: TCP, UDP, or both
- `target_address`: Optional IP address to target
- `target_ports`: Optional port range to target

## Error Handling

The library provides a custom error type `TrafficShapingError` that covers various error cases:

- Invalid configuration values
- Command execution failures
- System-level errors

## Notes

- This library requires root privileges to modify network settings
- Always remember to call `cleanup()` when you're done to restore normal network operation
- The library uses a single dummynet pipe (number 1) for all traffic shaping rules

## License

MIT License
