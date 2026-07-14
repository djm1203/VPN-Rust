//! VPN-Rust unified binary.
//!
//! A personal, QUIC-based point-to-point VPN. Run `vpn-rust server` on the
//! Linux host and `vpn-rust client` on a peer (Linux/macOS/Windows). The server
//! generates a self-signed identity on first run; pin its certificate on the
//! client via `--server-cert`.
//!
//! # Examples
//!
//! ```text
//! # On the server (Linux, needs root/CAP_NET_ADMIN for the TUN device):
//! sudo vpn-rust server --bind 0.0.0.0 --port 4433
//!
//! # Copy certs/server-cert.der to the client, then:
//! sudo vpn-rust client --server vpn.example.com --server-cert server-cert.der
//! ```

use std::net::{SocketAddr, ToSocketAddrs};

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;

use vpn_rust::cli::{Cli, Commands};
use vpn_rust::engine::{self, ClientParams, ServerParams};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging based on verbosity (RUST_LOG overrides the default).
    let log_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(cli.log_level()));
    tracing_subscriber::fmt().with_env_filter(log_filter).init();

    info!("VPN-Rust v{}", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Commands::Server(args) => {
            let params = ServerParams {
                bind: resolve_addr(&args.bind, args.port)?,
                server_name: args.server_name,
                tun_name: args.tun_name,
                tun_ip: args.tun_ip.parse().context("invalid --tun-ip")?,
                prefix: args.prefix,
                mtu: args.mtu,
                cert_path: args.cert,
                key_path: args.key,
            };
            engine::run_server(params).await
        }
        Commands::Client(args) => {
            let params = ClientParams {
                server_addr: resolve_addr(&args.server, args.port)?,
                server_name: args.server_name,
                server_cert_path: args.server_cert,
                tun_name: args.tun_name,
                tun_ip: args.tun_ip.parse().context("invalid --tun-ip")?,
                prefix: args.prefix,
                mtu: args.mtu,
                no_reconnect: args.no_reconnect,
                max_reconnects: args.max_reconnects,
            };
            engine::run_client(params).await
        }
    }
}

/// Resolve `host` + `port` to a single socket address.
fn resolve_addr(host: &str, port: u16) -> Result<SocketAddr> {
    (host, port)
        .to_socket_addrs()
        .with_context(|| format!("failed to resolve address '{host}:{port}'"))?
        .next()
        .with_context(|| format!("no address found for '{host}:{port}'"))
}
