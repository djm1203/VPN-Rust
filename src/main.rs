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

use std::future::Future;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use vpn_rust::cli::{Cli, Commands};
use vpn_rust::crypto::NodeIdentity;
use vpn_rust::engine::{self, ClientParams, LiveStats, ServerParams};
use vpn_rust::tui::{self, LogBuffer};

/// Capacity of the in-memory log ring rendered by the TUI.
const TUI_LOG_CAPACITY: usize = 1000;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Whether the dashboard will own the terminal, and whether to run headless
    // with plain logging (for journald). These are mutually exclusive (clap).
    let (tui_enabled, daemon) = match &cli.command {
        Commands::Server(a) => (a.tui, a.daemon),
        Commands::Client(a) => (a.tui, a.daemon),
        Commands::Keygen(_) => (false, false),
    };

    // In TUI mode `tracing` output is diverted into an in-memory buffer the
    // dashboard renders — stdout is owned by the alternate screen. Otherwise it
    // goes to stdout (ANSI disabled under --daemon for clean journald capture).
    let log_buffer = if tui_enabled {
        let buffer = LogBuffer::new(TUI_LOG_CAPACITY);
        tracing_subscriber::registry()
            .with(env_filter(&cli))
            .with(buffer.layer())
            .init();
        Some(buffer)
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter(&cli))
            .with_ansi(!daemon)
            .init();
        None
    };

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
                nat_interface: args.nat_interface,
            };
            let stats = LiveStats::new(true);
            run_session(
                engine::run_server(params, stats.clone()),
                stats,
                log_buffer,
                true,
            )
            .await
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
            let stats = LiveStats::new(false);
            run_session(
                engine::run_client(params, stats.clone()),
                stats,
                log_buffer,
                false,
            )
            .await
        }
        Commands::Keygen(args) => {
            if !args.force && (args.cert.exists() || args.key.exists()) {
                anyhow::bail!(
                    "refusing to overwrite existing '{}' / '{}' (use --force)",
                    args.cert.display(),
                    args.key.display()
                );
            }
            let identity =
                NodeIdentity::generate(&args.server_name).context("failed to generate identity")?;
            identity
                .save(&args.cert, &args.key)
                .context("failed to save identity")?;
            info!(
                "generated identity: certificate '{}', key '{}'",
                args.cert.display(),
                args.key.display()
            );
            info!("fingerprint: {}", identity.fingerprint());
            Ok(())
        }
    }
}

/// Build the tracing filter from `RUST_LOG`, falling back to the CLI verbosity.
fn env_filter(cli: &Cli) -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(cli.log_level()))
}

/// Run one VPN session, optionally under the live dashboard.
///
/// Without `log_buffer` the engine future is simply awaited (headless). With it,
/// the engine runs on a background task while the blocking dashboard event loop
/// owns the foreground; quitting the dashboard aborts the engine and returns.
async fn run_session<F>(
    engine_fut: F,
    stats: Arc<LiveStats>,
    log_buffer: Option<LogBuffer>,
    is_server: bool,
) -> Result<()>
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    match log_buffer {
        None => engine_fut.await,
        Some(logs) => {
            let engine = tokio::spawn(engine_fut);
            let result = tui::run_dashboard(stats, logs, is_server).await;
            // The operator quit (or the dashboard errored): stop the engine.
            engine.abort();
            result
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
