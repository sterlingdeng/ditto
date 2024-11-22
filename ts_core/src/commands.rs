use std::io::Write;
use std::process::Command;

use tempfile::NamedTempFile;
use tracing::info;

use crate::TrafficShapingError;

pub(crate) struct PfctlCommands;
pub(crate) struct DnctlCommands;

// pfctl - packet filter control
impl PfctlCommands {
    /// Loads PF rules from a file
    pub fn load_rules(rules: &str, anchor_name: Option<&str>) -> Result<(), TrafficShapingError> {
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(rules.as_bytes())?;

        let mut pfctl = Command::new("pfctl");
        if let Some(anchor_name) = anchor_name {
            pfctl.arg("-a").arg(anchor_name);
        }
        let output = pfctl.arg("-f").arg(temp_file.path()).output()?;

        if !output.status.success() {
            return Err(TrafficShapingError::CommandError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Restores original PF rules from /etc/pf.conf
    pub fn restore_original_rules() -> Result<(), TrafficShapingError> {
        let output = Command::new("pfctl")
            .arg("-f")
            .arg("/etc/pf.conf")
            .output()?;

        if !output.status.success() {
            return Err(TrafficShapingError::CommandError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Enables PF with reference counting
    pub fn enable() -> Result<(), TrafficShapingError> {
        let _ = Command::new("pfctl").arg("-e").output()?;
        Ok(())
    }

    /// Disables PF
    pub fn disable() -> Result<(), TrafficShapingError> {
        let output = Command::new("pfctl").arg("-d").output()?;

        if !output.status.success() {
            return Err(TrafficShapingError::CommandError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }
}

// dnctl - dummynet control
impl DnctlCommands {
    /// Checks if a pipe exists
    pub fn pipe_exists(pipe_num: u32) -> Result<bool, TrafficShapingError> {
        let output = Command::new("dnctl").arg("show").output()?;

        if !output.status.success() {
            return Err(TrafficShapingError::CommandError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.contains(&format!("pipe {} ", pipe_num)))
    }

    /// Creates or updates a pipe with specified configuration
    pub fn configure_pipe(
        pipe_num: u32,
        bandwidth: Option<u64>,
        delay: Option<u32>,
        plr: Option<f32>,
    ) -> Result<(), TrafficShapingError> {
        let mut cmd = Command::new("dnctl");
        cmd.arg("pipe").arg(pipe_num.to_string()).arg("config");

        if let Some(bw) = bandwidth {
            cmd.arg("bw").arg(format!("{}bit/s", bw));
        }

        if let Some(d) = delay {
            cmd.arg("delay").arg(format!("{}ms", d));
        }

        if let Some(p) = plr {
            cmd.arg("plr").arg(p.to_string());
        }

        let output = cmd.output()?;

        if !output.status.success() {
            return Err(TrafficShapingError::CommandError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Flushes all pipes
    pub fn flush_pipes() -> Result<(), TrafficShapingError> {
        let output = Command::new("dnctl").arg("-q").arg("flush").output()?;

        if !output.status.success() {
            return Err(TrafficShapingError::CommandError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }
}
