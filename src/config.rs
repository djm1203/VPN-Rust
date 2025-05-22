///
///Config.rs
///
/// a parser for a .ovpn file with ip and ports which will hold certs later on
use anyhow::{Context, Result};
use std::fs;

#[derive(Debug)]
pub struct OVPNConfig {
    pub remote_addr: String,
    pub remote_port: u16,
}

impl OVPNConfig {
    pub fn from_file(path: &str) -> Result<Self> {
        let content =
            fs::read_to_string(path).with_context(|| format!("Failed to read {}", path))?;

        let mut remote_addr = String::new();
        let mut remote_port = 1194;

        for line in content.lines() {
            if line.starts_with("remote ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    remote_addr = parts[1].into();
                    remote_port = parts[2].parse().unwrap_or(1194);
                }
            }
        }

        Ok(Self {
            remote_addr,
            remote_port,
        })
    }
}
