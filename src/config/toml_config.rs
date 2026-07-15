//! TOML configuration file support.
//!
//! This module provides configuration file parsing using TOML format.

use serde::{Deserialize, Serialize};
use std::fs;
use std::net::Ipv4Addr;
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

        // A config loaded from disk is always validated so callers never act on
        // out-of-range or malformed values (see `Config::validate`).
        config.validate()?;

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

impl Config {
    /// Validates the parsed configuration, returning an actionable error on the
    /// first problem found.
    ///
    /// Validation is **fail-fast**: it returns the *first* [`ConfigError::Invalid`]
    /// encountered rather than aggregating all problems. This keeps the return
    /// type simple; fix the reported field and re-run to surface the next issue.
    ///
    /// The present-section rule is: only sections that are actually present
    /// (`Some`) are checked, so a client-only or server-only config validates
    /// just the section it defines.
    ///
    /// # Checks performed
    ///
    /// * `server.bind`, `server.server_ip`, `client.client_ip` parse as `Ipv4Addr`.
    /// * `server.subnet` is valid CIDR (`A.B.C.D/prefix`) with prefix in `0..=32`.
    /// * `server.port` / `client.port` are non-zero.
    /// * Required path/name strings (`cert`, `key`, `tun_name`, `server`,
    ///   `hostname`, and `nat_interface` when set) are non-empty.
    ///
    /// Note: there is no MTU field in the on-disk config (the inner MTU is an
    /// engine/CLI parameter), so no MTU range is checked here.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Invalid`] describing the field, offending value,
    /// and what is allowed.
    pub fn validate(&self) -> Result<()> {
        if let Some(server) = &self.server {
            server.validate()?;
        }
        if let Some(client) = &self.client {
            client.validate()?;
        }
        Ok(())
    }
}

impl ServerConfig {
    /// Validates the server section. See [`Config::validate`].
    fn validate(&self) -> Result<()> {
        validate_ipv4("server.bind", &self.bind)?;
        validate_ipv4("server.server_ip", &self.server_ip)?;
        validate_cidr("server.subnet", &self.subnet)?;
        validate_port("server.port", self.port)?;
        validate_non_empty("server.cert", &self.cert)?;
        validate_non_empty("server.key", &self.key)?;
        validate_non_empty("server.tun_name", &self.tun_name)?;
        if let Some(iface) = &self.nat_interface {
            validate_non_empty("server.nat_interface", iface)?;
        }
        Ok(())
    }
}

impl ClientConfig {
    /// Validates the client section. See [`Config::validate`].
    fn validate(&self) -> Result<()> {
        // `server`/`hostname` may be DNS names, not IPs, so only require non-empty.
        validate_non_empty("client.server", &self.server)?;
        validate_non_empty("client.hostname", &self.hostname)?;
        validate_ipv4("client.client_ip", &self.client_ip)?;
        validate_port("client.port", self.port)?;
        validate_non_empty("client.tun_name", &self.tun_name)?;
        Ok(())
    }
}

/// Checks that `value` parses as an IPv4 address.
fn validate_ipv4(field: &'static str, value: &str) -> Result<()> {
    if value.parse::<Ipv4Addr>().is_err() {
        return Err(ConfigError::Invalid {
            field,
            value: value.to_string(),
            reason: "expected a valid IPv4 address (e.g. 10.8.0.1)".to_string(),
        });
    }
    Ok(())
}

/// Checks that `value` is CIDR notation `A.B.C.D/prefix` with prefix in `0..=32`.
fn validate_cidr(field: &'static str, value: &str) -> Result<()> {
    let Some((network, prefix)) = value.split_once('/') else {
        return Err(ConfigError::Invalid {
            field,
            value: value.to_string(),
            reason: "expected CIDR notation 'A.B.C.D/prefix' (e.g. 10.8.0.0/24)".to_string(),
        });
    };

    if network.parse::<Ipv4Addr>().is_err() {
        return Err(ConfigError::Invalid {
            field,
            value: value.to_string(),
            reason: "network part must be a valid IPv4 address (e.g. 10.8.0.0/24)".to_string(),
        });
    }

    match prefix.parse::<u8>() {
        Ok(bits) if bits <= 32 => Ok(()),
        _ => Err(ConfigError::Invalid {
            field,
            value: value.to_string(),
            reason: "prefix length must be an integer in the range 0..=32".to_string(),
        }),
    }
}

/// Checks that `port` is non-zero.
fn validate_port(field: &'static str, port: u16) -> Result<()> {
    if port == 0 {
        return Err(ConfigError::Invalid {
            field,
            value: port.to_string(),
            reason: "port must be in the range 1..=65535 (0 is not a valid port)".to_string(),
        });
    }
    Ok(())
}

/// Checks that a required string (path or name) is not empty/whitespace.
fn validate_non_empty(field: &'static str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(ConfigError::Invalid {
            field,
            value: value.to_string(),
            reason: "must not be empty".to_string(),
        });
    }
    Ok(())
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

    // --- validation tests ---

    /// A config built entirely from defaults (valid IPs, subnet, ports, paths)
    /// must pass validation, as must an empty config with no sections.
    #[test]
    fn test_validate_valid_config() {
        let config = Config {
            server: Some(ServerConfig::default()),
            client: Some(ClientConfig::default()),
        };
        assert!(config.validate().is_ok());

        // No sections present => nothing to validate.
        assert!(Config::default().validate().is_ok());
    }

    #[test]
    fn test_validate_bad_server_ip() {
        let server = ServerConfig {
            server_ip: "not-an-ip".to_string(),
            ..ServerConfig::default()
        };
        let config = Config {
            server: Some(server),
            client: None,
        };

        let err = config.validate().unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Invalid {
                field: "server.server_ip",
                ..
            }
        ));
        let msg = err.to_string();
        assert!(msg.contains("server.server_ip"), "message: {msg}");
        assert!(
            msg.contains("not-an-ip"),
            "message should echo value: {msg}"
        );
        assert!(
            msg.contains("IPv4"),
            "message should say what's allowed: {msg}"
        );
    }

    #[test]
    fn test_validate_bad_bind_address() {
        let server = ServerConfig {
            bind: "999.999.0.1".to_string(),
            ..ServerConfig::default()
        };
        let config = Config {
            server: Some(server),
            client: None,
        };

        let err = config.validate().unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Invalid {
                field: "server.bind",
                ..
            }
        ));
        assert!(err.to_string().contains("server.bind"));
    }

    #[test]
    fn test_validate_subnet_prefix_out_of_range() {
        let server = ServerConfig {
            subnet: "10.8.0.0/33".to_string(),
            ..ServerConfig::default()
        };
        let config = Config {
            server: Some(server),
            client: None,
        };

        let err = config.validate().unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Invalid {
                field: "server.subnet",
                ..
            }
        ));
        let msg = err.to_string();
        assert!(msg.contains("server.subnet"), "message: {msg}");
        assert!(
            msg.contains("0..=32"),
            "message should state the bound: {msg}"
        );
    }

    #[test]
    fn test_validate_subnet_not_cidr() {
        let server = ServerConfig {
            subnet: "10.8.0.0".to_string(), // missing "/prefix"
            ..ServerConfig::default()
        };
        let config = Config {
            server: Some(server),
            client: None,
        };

        let err = config.validate().unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Invalid {
                field: "server.subnet",
                ..
            }
        ));
        assert!(err.to_string().contains("CIDR"));
    }

    #[test]
    fn test_validate_zero_port() {
        let server = ServerConfig {
            port: 0,
            ..ServerConfig::default()
        };
        let config = Config {
            server: Some(server),
            client: None,
        };

        let err = config.validate().unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Invalid {
                field: "server.port",
                ..
            }
        ));
        let msg = err.to_string();
        assert!(msg.contains("server.port"), "message: {msg}");
        assert!(
            msg.contains("1..=65535"),
            "message should state the range: {msg}"
        );
    }

    #[test]
    fn test_validate_empty_cert_path() {
        let server = ServerConfig {
            cert: String::new(),
            ..ServerConfig::default()
        };
        let config = Config {
            server: Some(server),
            client: None,
        };

        let err = config.validate().unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Invalid {
                field: "server.cert",
                ..
            }
        ));
        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn test_validate_bad_client_ip() {
        let client = ClientConfig {
            client_ip: "10.8.0.999".to_string(),
            ..ClientConfig::default()
        };
        let config = Config {
            server: None,
            client: Some(client),
        };

        let err = config.validate().unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Invalid {
                field: "client.client_ip",
                ..
            }
        ));
        assert!(err.to_string().contains("client.client_ip"));
    }

    #[test]
    fn test_validate_empty_client_hostname() {
        let client = ClientConfig {
            hostname: "   ".to_string(), // whitespace-only
            ..ClientConfig::default()
        };
        let config = Config {
            server: None,
            client: Some(client),
        };

        let err = config.validate().unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Invalid {
                field: "client.hostname",
                ..
            }
        ));
    }
}
