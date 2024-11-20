use std::fs;
use crate::{Protocol, TrafficConfig, TrafficShapingError};

pub(crate) struct RuleGenerator;

impl RuleGenerator {
    /// Generates PF rules while preserving existing rules from /etc/pf.conf
    pub fn generate_pf_rules(config: &TrafficConfig, pipe_num: u32) -> Result<String, TrafficShapingError> {
        // First read existing pf.conf
        let existing_rules = fs::read_to_string("/etc/pf.conf")
            .map_err(|e| TrafficShapingError::SystemError(e))?;

        let mut rules = String::new();
        
        // Add existing rules first
        rules.push_str(&existing_rules);
        rules.push_str("\n\n# Traffic shaping rules added by traffic_shaper\n");
        
        // Add dummynet configuration
        rules.push_str("dummynet-anchor \"traffic-shaper\"\n");
        rules.push_str("anchor \"traffic-shaper\"\n\n");

        let proto = match config.protocol {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
            Protocol::Both => "proto { tcp udp }",
        };

        // Build the rule based on configuration
        let mut rule = format!("dummynet in quick proto {} ", proto);

        // Add address if specified
        if let Some(addr) = &config.target_address {
            rule.push_str(&format!("to {} ", addr));
        }

        // Add ports if specified
        if let Some(ports) = &config.target_ports {
            rule.push_str(&format!("port {} >< {} ", ports.start, ports.end));
        }

        // Add pipe number
        rule.push_str(&format!("pipe {}\n", pipe_num));

        // Add the rule for outbound traffic as well
        let out_rule = rule.replace("in", "out");
        
        rules.push_str(&rule);
        rules.push_str(&out_rule);

        Ok(rules)
    }
}
