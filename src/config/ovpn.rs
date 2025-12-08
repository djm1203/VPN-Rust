//! OpenVPN configuration file parsing.
//!
//! This module provides parsers for OpenVPN-compatible configuration files.
//! Currently supports basic connection parameters; certificate parsing is planned.

use anyhow::{Context, Result};
use log::debug;
use std::fs;

use crate::constants::DEFAULT_OVPN_PORT;

/// Configuration parsed from an OpenVPN (.ovpn) configuration file.
///
/// # Example
///
/// ```no_run
/// use vpn_rust::config::OVPNConfig;
///
/// let config = OVPNConfig::from_file("client.ovpn")?;
/// println!("Server: {}:{}", config.remote_addr, config.remote_port);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct OVPNConfig {
    /// The remote server address (hostname or IP).
    pub remote_addr: String,
    /// The remote server port.
    pub remote_port: u16,
}

impl OVPNConfig {
    /// Parses an OpenVPN configuration file.
    ///
    /// Currently extracts the `remote` directive to determine the server
    /// address and port.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the .ovpn configuration file.
    ///
    /// # Returns
    ///
    /// A parsed `OVPNConfig` with the server connection details.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The file contains no valid `remote` directive
    ///
    /// # Example File Format
    ///
    /// ```text
    /// client
    /// dev tun
    /// proto udp
    /// remote vpn.example.com 1194
    /// ```
    pub fn from_file(path: &str) -> Result<Self> {
        debug!("Loading configuration from: {}", path);

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;

        Self::parse(&content).with_context(|| format!("Failed to parse config file: {}", path))
    }

    /// Parses configuration from a string.
    ///
    /// # Arguments
    ///
    /// * `content` - The configuration file content as a string.
    ///
    /// # Returns
    ///
    /// A parsed `OVPNConfig` with the server connection details.
    ///
    /// # Errors
    ///
    /// Returns an error if no valid `remote` directive is found.
    pub fn parse(content: &str) -> Result<Self> {
        let mut remote_addr = String::new();
        let mut remote_port = DEFAULT_OVPN_PORT;

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            if line.starts_with("remote ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    remote_addr = parts[1].to_string();
                    if parts.len() >= 3 {
                        remote_port = parts[2].parse().unwrap_or(DEFAULT_OVPN_PORT);
                    }
                    debug!("Found remote: {}:{}", remote_addr, remote_port);
                }
            }
        }

        if remote_addr.is_empty() {
            return Err(anyhow::anyhow!(
                "No 'remote' directive found in configuration"
            ));
        }

        Ok(Self {
            remote_addr,
            remote_port,
        })
    }

    /// Returns the full server address as "host:port".
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.remote_addr, self.remote_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_config() {
        let content = r#"
            client
            dev tun
            remote vpn.example.com 1194
        "#;

        let config = OVPNConfig::parse(content).unwrap();
        assert_eq!(config.remote_addr, "vpn.example.com");
        assert_eq!(config.remote_port, 1194);
    }

    #[test]
    fn test_parse_config_default_port() {
        let content = "remote vpn.example.com";

        let config = OVPNConfig::parse(content).unwrap();
        assert_eq!(config.remote_addr, "vpn.example.com");
        assert_eq!(config.remote_port, DEFAULT_OVPN_PORT);
    }

    #[test]
    fn test_parse_config_with_comments() {
        let content = r#"
            # This is a comment
            ; This is also a comment
            remote server.local 443
        "#;

        let config = OVPNConfig::parse(content).unwrap();
        assert_eq!(config.remote_addr, "server.local");
        assert_eq!(config.remote_port, 443);
    }

    #[test]
    fn test_parse_config_no_remote() {
        let content = r#"
            client
            dev tun
        "#;

        let result = OVPNConfig::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_server_address() {
        let config = OVPNConfig {
            remote_addr: "test.com".to_string(),
            remote_port: 8080,
        };

        assert_eq!(config.server_address(), "test.com:8080");
    }
}
