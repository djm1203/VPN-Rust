//! VPN Client binary.
//!
//! Connects to a VPN server over TLS and tunnels packets bidirectionally
//! between the local TUN interface and the server.
//!
//! # Usage
//!
//! ```bash
//! sudo RUST_LOG=info cargo run --bin client
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::Mutex;
use tokio::time;
use tokio_rustls::client::TlsStream;
use tracing::{debug, error, info, trace, warn};
use vpn_rust::constants::{
    CLIENT_TUN_NAME, CONNECTION_TIMEOUT_SECS, DEFAULT_SERVER_ADDR, DEFAULT_SERVER_PORT,
    KEEPALIVE_INTERVAL_SECS, KEEPALIVE_MARKER, RECONNECT_INITIAL_DELAY_MS, RECONNECT_MAX_DELAY_MS,
    VPN_SUBNET,
};
use vpn_rust::net::route;
use vpn_rust::net::tls::connect_tls;
use vpn_rust::net::tun::TunInterface;

/// Tracks the last activity timestamp for connection monitoring.
struct ConnectionState {
    last_activity: AtomicU64,
    start_time: Instant,
}

impl ConnectionState {
    fn new() -> Self {
        Self {
            last_activity: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    fn update_activity(&self) {
        let elapsed = self.start_time.elapsed().as_secs();
        self.last_activity.store(elapsed, Ordering::Relaxed);
    }

    fn seconds_since_activity(&self) -> u64 {
        let last = self.last_activity.load(Ordering::Relaxed);
        let now = self.start_time.elapsed().as_secs();
        now.saturating_sub(last)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let log_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(log_filter).init();

    info!("Starting VPN client");

    // Create and configure TUN interface
    let tun = TunInterface::create_client().context("Failed to create TUN interface")?;
    tun.configure_client_ip()
        .context("Failed to configure client IP")?;

    info!("TUN device created: {}", tun.name);

    // Add route to VPN subnet through TUN interface
    if let Err(e) = route::add_route(VPN_SUBNET, CLIENT_TUN_NAME) {
        warn!("Failed to add VPN route (may already exist): {}", e);
    }

    // Wrap TUN in Arc for sharing between tasks
    let tun = Arc::new(tun);

    // Reconnection loop with exponential backoff
    let mut reconnect_delay = RECONNECT_INITIAL_DELAY_MS;

    loop {
        match run_connection(Arc::clone(&tun)).await {
            Ok(()) => {
                info!("Connection closed gracefully");
                break;
            }
            Err(e) => {
                error!("Connection error: {}", e);

                info!(
                    "Reconnecting in {} ms (max {} ms)...",
                    reconnect_delay, RECONNECT_MAX_DELAY_MS
                );

                time::sleep(Duration::from_millis(reconnect_delay)).await;

                // Exponential backoff with cap
                reconnect_delay = (reconnect_delay * 2).min(RECONNECT_MAX_DELAY_MS);
            }
        }
    }

    // Clean up routes
    if let Err(e) = route::remove_route(VPN_SUBNET) {
        warn!("Failed to remove VPN route: {}", e);
    }

    Ok(())
}

/// Runs a single connection session to the server.
async fn run_connection(tun: Arc<TunInterface>) -> Result<()> {
    let domain = "localhost";
    let addr = format!("{}:{}", DEFAULT_SERVER_ADDR, DEFAULT_SERVER_PORT);

    info!("Connecting to server at {}", addr);

    let tls_stream = connect_tls(domain, &addr)
        .await
        .context("Failed to connect to server")?;

    info!("Connected to server, starting bidirectional packet tunnel");

    // Connection state for keepalive monitoring
    let conn_state = Arc::new(ConnectionState::new());
    conn_state.update_activity();

    // Split the TLS stream for concurrent read/write
    let (tls_read, tls_write) = tokio::io::split(tls_stream);
    let tls_write = Arc::new(Mutex::new(tls_write));

    // Spawn task: TUN -> TLS (outbound)
    let tun_to_tls = {
        let tun = Arc::clone(&tun);
        let tls_write = Arc::clone(&tls_write);
        tokio::spawn(async move {
            if let Err(e) = tun_to_tls_task(tun, tls_write).await {
                error!("TUN->TLS task error: {}", e);
            }
        })
    };

    // Spawn task: TLS -> TUN (inbound)
    let tls_to_tun = {
        let tun = Arc::clone(&tun);
        let conn_state = Arc::clone(&conn_state);
        tokio::spawn(async move {
            if let Err(e) = tls_to_tun_task(tun, tls_read, conn_state).await {
                error!("TLS->TUN task error: {}", e);
            }
        })
    };

    // Spawn task: Keepalive sender
    let keepalive_sender = {
        let tls_write = Arc::clone(&tls_write);
        tokio::spawn(async move {
            if let Err(e) = keepalive_task(tls_write).await {
                error!("Keepalive task error: {}", e);
            }
        })
    };

    // Spawn task: Connection monitor
    let connection_monitor = {
        let conn_state = Arc::clone(&conn_state);
        tokio::spawn(async move { connection_monitor_task(conn_state).await })
    };

    // Wait for any task to complete (indicates connection issue)
    tokio::select! {
        result = tun_to_tls => {
            warn!("TUN->TLS task ended: {:?}", result);
        }
        result = tls_to_tun => {
            warn!("TLS->TUN task ended: {:?}", result);
        }
        result = keepalive_sender => {
            warn!("Keepalive task ended: {:?}", result);
        }
        _ = connection_monitor => {
            warn!("Connection timeout - no activity for {} seconds", CONNECTION_TIMEOUT_SECS);
            return Err(anyhow::anyhow!("Connection timeout"));
        }
    }

    Err(anyhow::anyhow!("Connection lost"))
}

/// Task that reads packets from TUN and sends them to the server over TLS.
async fn tun_to_tls_task(
    tun: Arc<TunInterface>,
    tls_write: Arc<Mutex<WriteHalf<TlsStream<tokio::net::TcpStream>>>>,
) -> Result<()> {
    loop {
        // Read packet from TUN
        let packet = match tun.read_packet().await {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to read from TUN: {}", e);
                continue;
            }
        };

        debug!("Sending {} bytes to server", packet.len());

        // Send to server (length-prefixed)
        let mut writer = tls_write.lock().await;

        writer
            .write_all(&(packet.len() as u16).to_be_bytes())
            .await
            .context("Failed to send packet length")?;

        writer
            .write_all(&packet)
            .await
            .context("Failed to send packet data")?;

        writer.flush().await.context("Failed to flush TLS stream")?;
    }
}

/// Task that reads packets from the server over TLS and writes them to TUN.
async fn tls_to_tun_task(
    tun: Arc<TunInterface>,
    mut tls_read: ReadHalf<TlsStream<tokio::net::TcpStream>>,
    conn_state: Arc<ConnectionState>,
) -> Result<()> {
    loop {
        // Read length prefix (2 bytes, big-endian)
        let mut len_buf = [0u8; 2];
        match tls_read.read_exact(&mut len_buf).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                info!("Server closed connection");
                return Ok(());
            }
            Err(e) => {
                return Err(e).context("Failed to read packet length from server");
            }
        }

        // Update activity timestamp
        conn_state.update_activity();

        let len = u16::from_be_bytes(len_buf) as usize;

        // Check for keepalive packet (length = 0)
        if len == KEEPALIVE_MARKER as usize {
            trace!("Received keepalive from server");
            continue;
        }

        // Read packet data
        let mut buf = vec![0u8; len];
        tls_read
            .read_exact(&mut buf)
            .await
            .context("Failed to read packet data from server")?;

        debug!("Received {} bytes from server", len);

        // Write to TUN
        if let Err(e) = tun.write_packet(&buf).await {
            error!("Failed to write to TUN: {}", e);
            continue;
        }
    }
}

/// Task that sends periodic keepalive packets to the server.
async fn keepalive_task(
    tls_write: Arc<Mutex<WriteHalf<TlsStream<tokio::net::TcpStream>>>>,
) -> Result<()> {
    let mut interval = time::interval(Duration::from_secs(KEEPALIVE_INTERVAL_SECS));

    loop {
        interval.tick().await;

        trace!("Sending keepalive to server");

        let mut writer = tls_write.lock().await;

        // Send keepalive (length = 0)
        writer
            .write_all(&KEEPALIVE_MARKER.to_be_bytes())
            .await
            .context("Failed to send keepalive")?;

        writer.flush().await.context("Failed to flush keepalive")?;
    }
}

/// Task that monitors connection state and returns when timeout is detected.
async fn connection_monitor_task(conn_state: Arc<ConnectionState>) {
    let mut interval = time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        let inactive_secs = conn_state.seconds_since_activity();
        trace!("Connection inactive for {} seconds", inactive_secs);

        if inactive_secs >= CONNECTION_TIMEOUT_SECS {
            error!(
                "Connection timeout: no activity for {} seconds",
                inactive_secs
            );
            return;
        }
    }
}
