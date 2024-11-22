use std::fs;

use crate::{Protocol, TrafficConfig, TrafficShapingError};

pub(crate) struct RuleGenerator;

impl RuleGenerator {
    /// Generates PF rules while preserving existing rules from /etc/pf.conf
    pub fn generate_pf_rules(
        config: &TrafficConfig,
        pipe_num: u32,
    ) -> Result<String, TrafficShapingError> {
        let proto = match config.protocol {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
            Protocol::Both => "proto { tcp udp }",
        };

        // Build the rule based on configuration
        let mut rule = format!("dummynet in quick proto {} ", proto);

        if let Some(src_ports) = &config.src_ports {
            rule.push_str(&format!("from port {}:{} ", src_ports.start, src_ports.end));
        } else {
            rule.push_str(&format!("from any "));
        }

        if let Some(dst_ports) = &config.dst_ports {
            rule.push_str(&format!("to port {}:{} ", dst_ports.start, dst_ports.end));
        } else {
            rule.push_str(&format!("to any "));
        }

        // Add pipe number
        rule.push_str(&format!("pipe {}\n", pipe_num));

        // Add the rule for outbound traffic as well
        let out_rule = rule.replace("in", "out");

        let mut rules = String::new();

        rules.push_str(&rule);
        rules.push_str(&out_rule);

        Ok(rules)
    }

    pub fn generate_anchor_rules(name: &str) -> Result<String, TrafficShapingError> {
        // First read existing pf.conf
        let existing_rules =
            fs::read_to_string("/etc/pf.conf").map_err(|e| TrafficShapingError::SystemError(e))?;

        let mut rules = String::new();

        rules.push_str(&existing_rules);
        rules.push_str("\n\n# Traffic shaping rules added by traffic_shaper\n");
        // Add dummynet configuration
        rules.push_str(&format!("dummynet-anchor \"{}\"\n", name));
        rules.push_str(&format!("anchor \"{}\"\n\n", name));
        Ok(rules)
    }
}
