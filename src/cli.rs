//! Command-line interface definitions for VPN-Rust.
//!
//! This module provides the CLI argument parsing using clap's derive API.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// VPN-Rust: A simple VPN client and server implementation in Rust.
#[derive(Parser, Debug)]
#[command(name = "vpn-rust")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Enable verbose output (can be repeated for more verbosity)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress all output except errors
    #[arg(short, long)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run the VPN server
    Server(ServerArgs),

    /// Run the VPN client
    Client(ClientArgs),
}

/// Server command arguments
#[derive(Parser, Debug)]
pub struct ServerArgs {
    /// Address to bind the server to
    #[arg(short, long, default_value = "0.0.0.0")]
    pub bind: String,

    /// Port to listen on
    #[arg(short, long, default_value = "4433")]
    pub port: u16,

    /// Path to TLS certificate file
    #[arg(long, default_value = "certs/server.crt")]
    pub cert: PathBuf,

    /// Path to TLS private key file
    #[arg(long, default_value = "certs/server.key")]
    pub key: PathBuf,

    /// TUN interface name
    #[arg(long, default_value = "rustvpn0")]
    pub tun_name: String,

    /// VPN subnet in CIDR notation
    #[arg(long, default_value = "10.8.0.0/24")]
    pub subnet: String,

    /// Server IP address within VPN subnet
    #[arg(long, default_value = "10.8.0.1")]
    pub server_ip: String,

    /// Enable NAT for client internet access
    #[arg(long, default_value = "true")]
    pub enable_nat: bool,

    /// Outbound interface for NAT (auto-detected if not specified)
    #[arg(long)]
    pub nat_interface: Option<String>,
}

/// Client command arguments
#[derive(Parser, Debug)]
pub struct ClientArgs {
    /// Server address to connect to
    #[arg(short, long, default_value = "127.0.0.1")]
    pub server: String,

    /// Server port
    #[arg(short, long, default_value = "4433")]
    pub port: u16,

    /// Server hostname for TLS verification
    #[arg(long, default_value = "localhost")]
    pub hostname: String,

    /// TUN interface name
    #[arg(long, default_value = "rustvpn1")]
    pub tun_name: String,

    /// Client IP address within VPN subnet
    #[arg(long, default_value = "10.8.0.2")]
    pub client_ip: String,

    /// Disable automatic reconnection
    #[arg(long)]
    pub no_reconnect: bool,

    /// Maximum reconnection attempts (0 = unlimited)
    #[arg(long, default_value = "0")]
    pub max_reconnects: u32,
}

impl Cli {
    /// Returns the log level based on verbosity flags.
    pub fn log_level(&self) -> &'static str {
        if self.quiet {
            "error"
        } else {
            match self.verbose {
                0 => "info",
                1 => "debug",
                _ => "trace",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }

    #[test]
    fn test_log_level() {
        let cli = Cli {
            config: None,
            verbose: 0,
            quiet: false,
            command: Commands::Server(ServerArgs {
                bind: "0.0.0.0".to_string(),
                port: 4433,
                cert: PathBuf::from("certs/server.crt"),
                key: PathBuf::from("certs/server.key"),
                tun_name: "rustvpn0".to_string(),
                subnet: "10.8.0.0/24".to_string(),
                server_ip: "10.8.0.1".to_string(),
                enable_nat: true,
                nat_interface: None,
            }),
        };
        assert_eq!(cli.log_level(), "info");
    }

    #[test]
    fn test_verbose_log_level() {
        let cli = Cli {
            config: None,
            verbose: 2,
            quiet: false,
            command: Commands::Server(ServerArgs {
                bind: "0.0.0.0".to_string(),
                port: 4433,
                cert: PathBuf::from("certs/server.crt"),
                key: PathBuf::from("certs/server.key"),
                tun_name: "rustvpn0".to_string(),
                subnet: "10.8.0.0/24".to_string(),
                server_ip: "10.8.0.1".to_string(),
                enable_nat: true,
                nat_interface: None,
            }),
        };
        assert_eq!(cli.log_level(), "trace");
    }
}
