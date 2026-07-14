//! VPN-Rust unified binary.
//!
//! This is the main entry point for the VPN application, supporting both
//! server and client modes through subcommands.
//!
//! # Usage
//!
//! ```bash
//! # Run server
//! sudo vpn-rust server
//!
//! # Run client
//! sudo vpn-rust client --server 192.168.1.1
//!
//! # With config file
//! sudo vpn-rust -c config.toml server
//!
//! # Verbose output
//! sudo vpn-rust -v client
//! ```

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;

use vpn_rust::cli::{Cli, Commands};
use vpn_rust::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging based on verbosity (RUST_LOG overrides the CLI-derived default)
    let log_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(cli.log_level()));
    tracing_subscriber::fmt().with_env_filter(log_filter).init();

    info!("VPN-Rust v{}", env!("CARGO_PKG_VERSION"));

    // Load config file if specified
    let config = if let Some(config_path) = &cli.config {
        Some(Config::from_file(config_path).context("Failed to load configuration")?)
    } else {
        None
    };

    // Dispatch to appropriate subcommand
    match cli.command {
        Commands::Server(args) => server::run(args, config).await,
        Commands::Client(args) => client::run(args, config).await,
    }
}

/// Server module - handles server subcommand
mod server {
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::Duration;

    use anyhow::{Context, Result};
    use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
    use tokio::sync::{broadcast, Mutex, RwLock};
    use tokio::time;
    use tokio_rustls::server::TlsStream;
    use tracing::{debug, error, info, trace, warn};

    use vpn_rust::cli::ServerArgs;
    use vpn_rust::config::Config;
    use vpn_rust::constants::{KEEPALIVE_INTERVAL_SECS, KEEPALIVE_MARKER};
    use vpn_rust::net::route;
    use vpn_rust::net::tls::start_tls_server;
    use vpn_rust::net::tun::TunInterface;

    struct ClientConnection {
        writer: Arc<Mutex<WriteHalf<TlsStream<tokio::net::TcpStream>>>>,
        vpn_ip: std::net::Ipv4Addr,
    }

    type ClientMap = Arc<RwLock<HashMap<SocketAddr, ClientConnection>>>;

    pub async fn run(args: ServerArgs, config: Option<Config>) -> Result<()> {
        // Merge CLI args with config file (CLI takes precedence)
        let _server_config = config.and_then(|c| c.server).unwrap_or_default();

        let bind = args.bind;
        let port = args.port;
        let _cert_path = args.cert.to_string_lossy().to_string();
        let _key_path = args.key.to_string_lossy().to_string();
        let tun_name = args.tun_name;
        let subnet = args.subnet;
        let server_ip = args.server_ip;
        let enable_nat = args.enable_nat;
        let nat_interface = args.nat_interface;

        info!("Starting VPN server");
        info!("  Bind: {}:{}", bind, port);
        info!("  TUN: {} ({})", tun_name, server_ip);
        info!("  Subnet: {}", subnet);

        // Create and configure TUN interface
        let tun = TunInterface::create_server().context("Failed to create TUN interface")?;
        tun.configure_server_ip()
            .context("Failed to configure server IP")?;

        info!("TUN device created: {}", tun.name);

        // Enable IP forwarding for packet routing
        route::enable_ip_forwarding().context("Failed to enable IP forwarding")?;

        // Set up NAT for outbound traffic
        if enable_nat {
            let outbound_interface = nat_interface
                .or_else(|| route::get_default_interface().ok())
                .unwrap_or_else(|| "eth0".to_string());
            info!("Using {} as outbound interface for NAT", outbound_interface);

            if let Err(e) = route::setup_nat(&subnet, &outbound_interface) {
                warn!("Failed to set up NAT (may require iptables): {}", e);
            }
        }

        let tun = Arc::new(tun);
        let clients: ClientMap = Arc::new(RwLock::new(HashMap::new()));
        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        // Start TLS server
        let addr = format!("{}:{}", bind, port);
        let server = start_tls_server(&addr)
            .await
            .context("Failed to start TLS server")?;

        info!("Server ready, waiting for connections on {}", addr);

        // Spawn TUN reader task
        let _tun_reader = {
            let tun = Arc::clone(&tun);
            let clients = Arc::clone(&clients);
            let mut shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                tokio::select! {
                    result = tun_to_clients_task(tun, clients) => {
                        if let Err(e) = result {
                            error!("TUN reader task error: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("TUN reader task shutting down");
                    }
                }
            })
        };

        // Accept connections
        loop {
            let (tcp_stream, peer_addr) = server
                .listener
                .accept()
                .await
                .context("Failed to accept connection")?;

            info!("New TCP connection from {}", peer_addr);

            let tls_stream = match server.acceptor.accept(tcp_stream).await {
                Ok(stream) => stream,
                Err(e) => {
                    warn!("TLS handshake failed with {}: {}", peer_addr, e);
                    continue;
                }
            };

            info!("TLS connection established with {}", peer_addr);

            let tun = Arc::clone(&tun);
            let clients = Arc::clone(&clients);

            tokio::spawn(async move {
                if let Err(e) = handle_client(tls_stream, tun, clients.clone(), peer_addr).await {
                    error!("Client {} error: {}", peer_addr, e);
                }

                {
                    let mut clients_guard = clients.write().await;
                    clients_guard.remove(&peer_addr);
                    info!(
                        "Client {} disconnected, {} clients remaining",
                        peer_addr,
                        clients_guard.len()
                    );
                }
            });
        }
    }

    async fn tun_to_clients_task(tun: Arc<TunInterface>, clients: ClientMap) -> Result<()> {
        loop {
            let packet = match tun.read_packet().await {
                Ok(p) => p,
                Err(e) => {
                    error!("Failed to read from TUN: {}", e);
                    continue;
                }
            };

            if packet.len() < 20 {
                debug!("Packet too short to contain IP header");
                continue;
            }

            let version = (packet[0] >> 4) & 0x0F;
            if version != 4 {
                debug!("Non-IPv4 packet (version {}), skipping", version);
                continue;
            }

            let dest_ip = std::net::Ipv4Addr::new(packet[16], packet[17], packet[18], packet[19]);
            debug!(
                "TUN packet for destination: {}, {} bytes",
                dest_ip,
                packet.len()
            );

            let target_client = {
                let clients_guard = clients.read().await;
                clients_guard
                    .iter()
                    .find(|(_, client)| client.vpn_ip == dest_ip)
                    .map(|(addr, client)| (*addr, Arc::clone(&client.writer)))
            };

            if let Some((peer_addr, writer)) = target_client {
                let mut writer_guard = writer.lock().await;
                if let Err(e) = send_packet(&mut writer_guard, &packet).await {
                    error!("Failed to send packet to {}: {}", peer_addr, e);
                }
            }
        }
    }

    async fn handle_client(
        stream: TlsStream<tokio::net::TcpStream>,
        tun: Arc<TunInterface>,
        clients: ClientMap,
        peer_addr: SocketAddr,
    ) -> Result<()> {
        let (read_half, write_half) = tokio::io::split(stream);
        let writer = Arc::new(Mutex::new(write_half));

        let vpn_ip = std::net::Ipv4Addr::new(10, 8, 0, 2);

        {
            let mut clients_guard = clients.write().await;
            clients_guard.insert(
                peer_addr,
                ClientConnection {
                    writer: Arc::clone(&writer),
                    vpn_ip,
                },
            );
            info!(
                "Client {} registered with VPN IP {}, {} total clients",
                peer_addr,
                vpn_ip,
                clients_guard.len()
            );
        }

        let keepalive_writer = Arc::clone(&writer);
        let keepalive_task = tokio::spawn(async move {
            if let Err(e) = client_keepalive_task(keepalive_writer, peer_addr).await {
                debug!("Keepalive task for {} ended: {}", peer_addr, e);
            }
        });

        let result = client_to_tun_task(read_half, tun, peer_addr).await;
        keepalive_task.abort();
        result
    }

    async fn client_keepalive_task(
        writer: Arc<Mutex<WriteHalf<TlsStream<tokio::net::TcpStream>>>>,
        peer_addr: SocketAddr,
    ) -> Result<()> {
        let mut interval = time::interval(Duration::from_secs(KEEPALIVE_INTERVAL_SECS));

        loop {
            interval.tick().await;
            trace!("Sending keepalive to {}", peer_addr);

            let mut writer_guard = writer.lock().await;
            writer_guard
                .write_all(&KEEPALIVE_MARKER.to_be_bytes())
                .await
                .context("Failed to send keepalive")?;
            writer_guard
                .flush()
                .await
                .context("Failed to flush keepalive")?;
        }
    }

    async fn client_to_tun_task(
        mut reader: ReadHalf<TlsStream<tokio::net::TcpStream>>,
        tun: Arc<TunInterface>,
        peer_addr: SocketAddr,
    ) -> Result<()> {
        loop {
            let mut len_buf = [0u8; 2];
            match reader.read_exact(&mut len_buf).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    debug!("Client {} closed connection", peer_addr);
                    return Ok(());
                }
                Err(e) => {
                    return Err(e).context("Failed to read packet length");
                }
            }

            let len = u16::from_be_bytes(len_buf) as usize;

            if len == KEEPALIVE_MARKER as usize {
                trace!("Received keepalive from {}", peer_addr);
                continue;
            }

            debug!("Receiving {} bytes from {}", len, peer_addr);

            let mut buf = vec![0u8; len];
            reader
                .read_exact(&mut buf)
                .await
                .context("Failed to read packet data")?;

            tun.write_packet(&buf)
                .await
                .context("Failed to write to TUN")?;

            debug!("Wrote {} bytes to TUN from {}", len, peer_addr);
        }
    }

    async fn send_packet(
        writer: &mut WriteHalf<TlsStream<tokio::net::TcpStream>>,
        packet: &[u8],
    ) -> Result<()> {
        writer
            .write_all(&(packet.len() as u16).to_be_bytes())
            .await
            .context("Failed to write packet length")?;
        writer
            .write_all(packet)
            .await
            .context("Failed to write packet data")?;
        writer.flush().await.context("Failed to flush stream")?;
        debug!("Sent {} bytes to client", packet.len());
        Ok(())
    }
}

/// Client module - handles client subcommand
mod client {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use anyhow::{Context, Result};
    use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
    use tokio::sync::Mutex;
    use tokio::time;
    use tokio_rustls::client::TlsStream;
    use tracing::{debug, error, info, trace, warn};

    use vpn_rust::cli::ClientArgs;
    use vpn_rust::config::Config;
    use vpn_rust::constants::{
        CONNECTION_TIMEOUT_SECS, KEEPALIVE_INTERVAL_SECS, KEEPALIVE_MARKER,
        RECONNECT_INITIAL_DELAY_MS, RECONNECT_MAX_DELAY_MS,
    };
    use vpn_rust::net::route;
    use vpn_rust::net::tls::connect_tls;
    use vpn_rust::net::tun::TunInterface;

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

    pub async fn run(args: ClientArgs, config: Option<Config>) -> Result<()> {
        let _client_config = config.and_then(|c| c.client).unwrap_or_default();

        let server = args.server;
        let port = args.port;
        let hostname = args.hostname;
        let tun_name = args.tun_name;
        let client_ip = args.client_ip;
        let no_reconnect = args.no_reconnect;
        let max_reconnects = args.max_reconnects;

        info!("Starting VPN client");
        info!("  Server: {}:{}", server, port);
        info!("  TUN: {} ({})", tun_name, client_ip);

        // Create and configure TUN interface
        let tun = TunInterface::create_client().context("Failed to create TUN interface")?;
        tun.configure_client_ip()
            .context("Failed to configure client IP")?;

        info!("TUN device created: {}", tun.name);

        // Add route to VPN subnet
        let subnet = "10.8.0.0/24";
        if let Err(e) = route::add_route(subnet, &tun_name) {
            warn!("Failed to add VPN route (may already exist): {}", e);
        }

        let tun = Arc::new(tun);

        // Reconnection loop
        let mut reconnect_delay = RECONNECT_INITIAL_DELAY_MS;
        let mut reconnect_count = 0u32;

        loop {
            let addr = format!("{}:{}", server, port);

            match run_connection(Arc::clone(&tun), &hostname, &addr).await {
                Ok(()) => {
                    info!("Connection closed gracefully");
                    break;
                }
                Err(e) => {
                    error!("Connection error: {}", e);

                    if no_reconnect {
                        break;
                    }

                    reconnect_count += 1;
                    if max_reconnects > 0 && reconnect_count >= max_reconnects {
                        error!("Maximum reconnection attempts ({}) reached", max_reconnects);
                        break;
                    }

                    info!(
                        "Reconnecting in {} ms (attempt {})...",
                        reconnect_delay, reconnect_count
                    );

                    time::sleep(Duration::from_millis(reconnect_delay)).await;
                    reconnect_delay = (reconnect_delay * 2).min(RECONNECT_MAX_DELAY_MS);
                }
            }
        }

        // Clean up routes
        if let Err(e) = route::remove_route(subnet) {
            warn!("Failed to remove VPN route: {}", e);
        }

        Ok(())
    }

    async fn run_connection(tun: Arc<TunInterface>, hostname: &str, addr: &str) -> Result<()> {
        info!("Connecting to server at {}", addr);

        let tls_stream = connect_tls(hostname, addr)
            .await
            .context("Failed to connect to server")?;

        info!("Connected to server, starting bidirectional packet tunnel");

        let conn_state = Arc::new(ConnectionState::new());
        conn_state.update_activity();

        let (tls_read, tls_write) = tokio::io::split(tls_stream);
        let tls_write = Arc::new(Mutex::new(tls_write));

        let tun_to_tls = {
            let tun = Arc::clone(&tun);
            let tls_write = Arc::clone(&tls_write);
            tokio::spawn(async move {
                if let Err(e) = tun_to_tls_task(tun, tls_write).await {
                    error!("TUN->TLS task error: {}", e);
                }
            })
        };

        let tls_to_tun = {
            let tun = Arc::clone(&tun);
            let conn_state = Arc::clone(&conn_state);
            tokio::spawn(async move {
                if let Err(e) = tls_to_tun_task(tun, tls_read, conn_state).await {
                    error!("TLS->TUN task error: {}", e);
                }
            })
        };

        let keepalive_sender = {
            let tls_write = Arc::clone(&tls_write);
            tokio::spawn(async move {
                if let Err(e) = keepalive_task(tls_write).await {
                    error!("Keepalive task error: {}", e);
                }
            })
        };

        let connection_monitor = {
            let conn_state = Arc::clone(&conn_state);
            tokio::spawn(async move { connection_monitor_task(conn_state).await })
        };

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

    async fn tun_to_tls_task(
        tun: Arc<TunInterface>,
        tls_write: Arc<Mutex<WriteHalf<TlsStream<tokio::net::TcpStream>>>>,
    ) -> Result<()> {
        loop {
            let packet = match tun.read_packet().await {
                Ok(p) => p,
                Err(e) => {
                    error!("Failed to read from TUN: {}", e);
                    continue;
                }
            };

            debug!("Sending {} bytes to server", packet.len());

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

    async fn tls_to_tun_task(
        tun: Arc<TunInterface>,
        mut tls_read: ReadHalf<TlsStream<tokio::net::TcpStream>>,
        conn_state: Arc<ConnectionState>,
    ) -> Result<()> {
        loop {
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

            conn_state.update_activity();

            let len = u16::from_be_bytes(len_buf) as usize;

            if len == KEEPALIVE_MARKER as usize {
                trace!("Received keepalive from server");
                continue;
            }

            let mut buf = vec![0u8; len];
            tls_read
                .read_exact(&mut buf)
                .await
                .context("Failed to read packet data from server")?;

            debug!("Received {} bytes from server", len);

            if let Err(e) = tun.write_packet(&buf).await {
                error!("Failed to write to TUN: {}", e);
                continue;
            }
        }
    }

    async fn keepalive_task(
        tls_write: Arc<Mutex<WriteHalf<TlsStream<tokio::net::TcpStream>>>>,
    ) -> Result<()> {
        let mut interval = time::interval(Duration::from_secs(KEEPALIVE_INTERVAL_SECS));

        loop {
            interval.tick().await;
            trace!("Sending keepalive to server");

            let mut writer = tls_write.lock().await;
            writer
                .write_all(&KEEPALIVE_MARKER.to_be_bytes())
                .await
                .context("Failed to send keepalive")?;
            writer.flush().await.context("Failed to flush keepalive")?;
        }
    }

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
}
