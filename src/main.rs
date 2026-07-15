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
use vpn_rust::config::{ClientConfig, Config, ServerConfig};
use vpn_rust::crypto::NodeIdentity;
use vpn_rust::engine::{self, ClientParams, LiveStats, ServerParams};
use vpn_rust::metrics;
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

    // Load and validate the optional config file up front so a bad file fails
    // fast with an actionable error before any network/TUN setup.
    let config = match &cli.config {
        Some(path) => Some(Config::from_file(path).context("failed to load configuration")?),
        None => None,
    };

    match cli.command {
        Commands::Server(args) => {
            let mut params = ServerParams {
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
            if let Some(sc) = config.as_ref().and_then(|c| c.server.as_ref()) {
                apply_server_config(&mut params, sc)?;
            }
            let stats = LiveStats::new(true);
            spawn_metrics(args.metrics_addr, &stats);
            run_session(
                engine::run_server(params, stats.clone()),
                stats,
                log_buffer,
                true,
            )
            .await
        }
        Commands::Client(args) => {
            let mut params = ClientParams {
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
            if let Some(cc) = config.as_ref().and_then(|c| c.client.as_ref()) {
                apply_client_config(&mut params, cc)?;
            }
            let stats = LiveStats::new(false);
            spawn_metrics(args.metrics_addr, &stats);
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

/// Spawn the Prometheus metrics endpoint if an address was requested.
///
/// It runs for the lifetime of the process on a background task; failures are
/// logged and do not stop the VPN session.
fn spawn_metrics(addr: Option<SocketAddr>, stats: &Arc<LiveStats>) {
    if let Some(addr) = addr {
        let stats = stats.clone();
        tokio::spawn(async move {
            if let Err(e) = metrics::serve(addr, stats).await {
                tracing::warn!("metrics endpoint stopped: {e:#}");
            }
        });
    }
}

/// Overlay the `[server]` config section onto CLI-derived parameters.
///
/// The config file supplies addressing and file paths; `--server-name` and
/// `--mtu` have no config equivalent and are left as given on the CLI. Values in
/// the file take precedence over the CLI defaults for the fields it covers.
fn apply_server_config(params: &mut ServerParams, cfg: &ServerConfig) -> Result<()> {
    params.bind = resolve_addr(&cfg.bind, cfg.port)?;
    params.tun_name = cfg.tun_name.clone();
    params.tun_ip = cfg
        .server_ip
        .parse()
        .with_context(|| format!("invalid config server.server_ip '{}'", cfg.server_ip))?;
    params.prefix = prefix_from_cidr(&cfg.subnet)?;
    params.cert_path = cfg.cert.clone().into();
    params.key_path = cfg.key.clone().into();
    if cfg.nat_interface.is_some() {
        params.nat_interface = cfg.nat_interface.clone();
    }
    Ok(())
}

/// Overlay the `[client]` config section onto CLI-derived parameters.
///
/// `--server-cert` and `--mtu` have no config equivalent and are left as given
/// on the CLI. Values in the file take precedence over the CLI defaults for the
/// fields it covers.
fn apply_client_config(params: &mut ClientParams, cfg: &ClientConfig) -> Result<()> {
    params.server_addr = resolve_addr(&cfg.server, cfg.port)?;
    params.server_name = cfg.hostname.clone();
    params.tun_name = cfg.tun_name.clone();
    params.tun_ip = cfg
        .client_ip
        .parse()
        .with_context(|| format!("invalid config client.client_ip '{}'", cfg.client_ip))?;
    params.no_reconnect = cfg.no_reconnect;
    params.max_reconnects = cfg.max_reconnects;
    Ok(())
}

/// Extract the prefix length from a validated CIDR string (`"10.8.0.0/24"` → 24).
fn prefix_from_cidr(cidr: &str) -> Result<u8> {
    cidr.split_once('/')
        .and_then(|(_, p)| p.parse::<u8>().ok())
        .filter(|p| *p <= 32)
        .with_context(|| format!("invalid config subnet CIDR '{cidr}'"))
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
