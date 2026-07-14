//! Command-line interface definitions for VPN-Rust.
//!
//! This module provides the CLI argument parsing using clap's derive API.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// VPN-Rust: a personal, QUIC-based point-to-point VPN.
#[derive(Parser, Debug)]
#[command(name = "vpn-rust")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Configuration file path (reserved; not yet wired into the QUIC engine)
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
    /// Run the VPN server (listens for a single peer over QUIC/UDP)
    Server(ServerArgs),

    /// Run the VPN client (connects to a server over QUIC/UDP)
    Client(ClientArgs),

    /// Generate a self-signed node identity (certificate + private key)
    Keygen(KeygenArgs),
}

/// Server command arguments
#[derive(Parser, Debug)]
pub struct ServerArgs {
    /// UDP address to bind the QUIC server to
    #[arg(short, long, default_value = "0.0.0.0")]
    pub bind: String,

    /// UDP port to listen on
    #[arg(short, long, default_value = "4433")]
    pub port: u16,

    /// Name embedded in the generated certificate's SAN (clients must connect
    /// with a matching `--server-name`)
    #[arg(long, default_value = "localhost")]
    pub server_name: String,

    /// TUN interface name
    #[arg(long, default_value = "rustvpn0")]
    pub tun_name: String,

    /// Server IP address within the VPN subnet
    #[arg(long, default_value = "10.8.0.1")]
    pub tun_ip: String,

    /// VPN subnet prefix length (CIDR)
    #[arg(long, default_value = "30")]
    pub prefix: u8,

    /// Inner MTU for tunneled packets
    #[arg(long, default_value = "1300")]
    pub mtu: u16,

    /// Path to the server certificate (DER); generated if missing
    #[arg(long, default_value = "certs/server-cert.der")]
    pub cert: PathBuf,

    /// Path to the server private key (DER); generated if missing
    #[arg(long, default_value = "certs/server-key.der")]
    pub key: PathBuf,

    /// Outbound interface for server NAT (auto-detected if omitted)
    #[arg(long)]
    pub nat_interface: Option<String>,
}

/// Client command arguments
#[derive(Parser, Debug)]
pub struct ClientArgs {
    /// Server address to connect to
    #[arg(short, long, default_value = "127.0.0.1")]
    pub server: String,

    /// Server UDP port
    #[arg(short, long, default_value = "4433")]
    pub port: u16,

    /// Server name for certificate validation (must match the certificate SAN)
    #[arg(long, default_value = "localhost")]
    pub server_name: String,

    /// Path to the pinned server certificate (DER)
    #[arg(long, default_value = "certs/server-cert.der")]
    pub server_cert: PathBuf,

    /// TUN interface name
    #[arg(long, default_value = "rustvpn1")]
    pub tun_name: String,

    /// Client IP address within the VPN subnet
    #[arg(long, default_value = "10.8.0.2")]
    pub tun_ip: String,

    /// VPN subnet prefix length (CIDR)
    #[arg(long, default_value = "30")]
    pub prefix: u8,

    /// Inner MTU for tunneled packets
    #[arg(long, default_value = "1300")]
    pub mtu: u16,

    /// Disable automatic reconnection
    #[arg(long)]
    pub no_reconnect: bool,

    /// Maximum reconnection attempts (0 = unlimited)
    #[arg(long, default_value = "0")]
    pub max_reconnects: u32,
}

/// `keygen` command arguments
#[derive(Parser, Debug)]
pub struct KeygenArgs {
    /// Name to embed in the certificate SAN (clients connect with a matching
    /// `--server-name`)
    #[arg(long, default_value = "localhost")]
    pub server_name: String,

    /// Output path for the certificate (DER)
    #[arg(long, default_value = "certs/server-cert.der")]
    pub cert: PathBuf,

    /// Output path for the private key (DER)
    #[arg(long, default_value = "certs/server-key.der")]
    pub key: PathBuf,

    /// Overwrite existing certificate/key files
    #[arg(long)]
    pub force: bool,
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

    fn sample_server() -> Commands {
        Commands::Server(ServerArgs {
            bind: "0.0.0.0".to_string(),
            port: 4433,
            server_name: "localhost".to_string(),
            tun_name: "rustvpn0".to_string(),
            tun_ip: "10.8.0.1".to_string(),
            prefix: 30,
            mtu: 1300,
            cert: PathBuf::from("certs/server-cert.der"),
            key: PathBuf::from("certs/server-key.der"),
            nat_interface: None,
        })
    }

    #[test]
    fn test_log_level() {
        let cli = Cli {
            config: None,
            verbose: 0,
            quiet: false,
            command: sample_server(),
        };
        assert_eq!(cli.log_level(), "info");
    }

    #[test]
    fn test_verbose_log_level() {
        let cli = Cli {
            config: None,
            verbose: 2,
            quiet: false,
            command: sample_server(),
        };
        assert_eq!(cli.log_level(), "trace");
    }

    #[test]
    fn test_quiet_log_level() {
        let cli = Cli {
            config: None,
            verbose: 0,
            quiet: true,
            command: sample_server(),
        };
        assert_eq!(cli.log_level(), "error");
    }
}
