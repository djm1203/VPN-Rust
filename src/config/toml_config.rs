//! TOML configuration file support.
//!
//! This module provides configuration file parsing using TOML format.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::debug;

use super::error::{ConfigError, Result};

/// Root configuration structure.
///
/// # Example TOML
///
/// ```toml
/// [server]
/// bind = "0.0.0.0"
/// port = 4433
/// cert = "certs/server.crt"
/// key = "certs/server.key"
///
/// [client]
/// server = "vpn.example.com"
/// port = 4433
/// hostname = "vpn.example.com"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Server configuration section.
    #[serde(default)]
    pub server: Option<ServerConfig>,

    /// Client configuration section.
    #[serde(default)]
    pub client: Option<ClientConfig>,
}

/// Server-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Address to bind the server to.
    #[serde(default = "default_bind")]
    pub bind: String,

    /// Port to listen on.
    #[serde(default = "default_port")]
    pub port: u16,

    /// Path to TLS certificate file.
    #[serde(default = "default_cert")]
    pub cert: String,

    /// Path to TLS private key file.
    #[serde(default = "default_key")]
    pub key: String,

    /// TUN interface name.
    #[serde(default = "default_server_tun")]
    pub tun_name: String,

    /// VPN subnet in CIDR notation.
    #[serde(default = "default_subnet")]
    pub subnet: String,

    /// Server IP address within VPN subnet.
    #[serde(default = "default_server_ip")]
    pub server_ip: String,

    /// Enable NAT for client internet access.
    #[serde(default = "default_true")]
    pub enable_nat: bool,

    /// Outbound interface for NAT.
    #[serde(default)]
    pub nat_interface: Option<String>,
}

/// Client-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Server address to connect to.
    #[serde(default = "default_localhost")]
    pub server: String,

    /// Server port.
    #[serde(default = "default_port")]
    pub port: u16,

    /// Server hostname for TLS verification.
    #[serde(default = "default_localhost")]
    pub hostname: String,

    /// TUN interface name.
    #[serde(default = "default_client_tun")]
    pub tun_name: String,

    /// Client IP address within VPN subnet.
    #[serde(default = "default_client_ip")]
    pub client_ip: String,

    /// Disable automatic reconnection.
    #[serde(default)]
    pub no_reconnect: bool,

    /// Maximum reconnection attempts (0 = unlimited).
    #[serde(default)]
    pub max_reconnects: u32,
}

// Default value functions
fn default_bind() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    4433
}

fn default_cert() -> String {
    "certs/server.crt".to_string()
}

fn default_key() -> String {
    "certs/server.key".to_string()
}

fn default_server_tun() -> String {
    "rustvpn0".to_string()
}

fn default_client_tun() -> String {
    "rustvpn1".to_string()
}

fn default_subnet() -> String {
    "10.8.0.0/24".to_string()
}

fn default_server_ip() -> String {
    "10.8.0.1".to_string()
}

fn default_client_ip() -> String {
    "10.8.0.2".to_string()
}

fn default_localhost() -> String {
    "localhost".to_string()
}

fn default_true() -> bool {
    true
}

impl Config {
    /// Loads configuration from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML configuration file.
    ///
    /// # Returns
    ///
    /// A parsed `Config` structure.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        debug!("Loading configuration from: {}", path.display());

        let content = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        let config: Config = toml::from_str(&content).map_err(|source| ConfigError::TomlFile {
            path: path.to_path_buf(),
            source: Box::new(source),
        })?;

        debug!("Parsed configuration: {:?}", config);
        Ok(config)
    }

    /// Parses configuration from a TOML string.
    ///
    /// # Arguments
    ///
    /// * `content` - The TOML configuration content.
    ///
    /// # Returns
    ///
    /// A parsed `Config` structure.
    pub fn parse(content: &str) -> Result<Self> {
        let config: Config = toml::from_str(content)?;

        debug!("Parsed configuration: {:?}", config);
        Ok(config)
    }

    /// Creates a default server configuration.
    pub fn default_server() -> ServerConfig {
        ServerConfig {
            bind: default_bind(),
            port: default_port(),
            cert: default_cert(),
            key: default_key(),
            tun_name: default_server_tun(),
            subnet: default_subnet(),
            server_ip: default_server_ip(),
            enable_nat: true,
            nat_interface: None,
        }
    }

    /// Creates a default client configuration.
    pub fn default_client() -> ClientConfig {
        ClientConfig {
            server: default_localhost(),
            port: default_port(),
            hostname: default_localhost(),
            tun_name: default_client_tun(),
            client_ip: default_client_ip(),
            no_reconnect: false,
            max_reconnects: 0,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Config::default_server()
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Config::default_client()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_server_config() {
        let content = r#"
            [server]
            bind = "192.168.1.1"
            port = 8443
            cert = "/etc/vpn/server.crt"
            key = "/etc/vpn/server.key"
        "#;

        let config = Config::parse(content).unwrap();
        let server = config.server.unwrap();

        assert_eq!(server.bind, "192.168.1.1");
        assert_eq!(server.port, 8443);
        assert_eq!(server.cert, "/etc/vpn/server.crt");
        assert_eq!(server.key, "/etc/vpn/server.key");
    }

    #[test]
    fn test_parse_client_config() {
        let content = r#"
            [client]
            server = "vpn.example.com"
            port = 443
            hostname = "vpn.example.com"
            no_reconnect = true
        "#;

        let config = Config::parse(content).unwrap();
        let client = config.client.unwrap();

        assert_eq!(client.server, "vpn.example.com");
        assert_eq!(client.port, 443);
        assert_eq!(client.hostname, "vpn.example.com");
        assert!(client.no_reconnect);
    }

    #[test]
    fn test_default_values() {
        let content = r#"
            [server]
            bind = "10.0.0.1"
        "#;

        let config = Config::parse(content).unwrap();
        let server = config.server.unwrap();

        // Specified value
        assert_eq!(server.bind, "10.0.0.1");
        // Default values
        assert_eq!(server.port, 4433);
        assert_eq!(server.cert, "certs/server.crt");
    }

    #[test]
    fn test_empty_config() {
        let content = "";
        let config = Config::parse(content).unwrap();

        assert!(config.server.is_none());
        assert!(config.client.is_none());
    }
}
