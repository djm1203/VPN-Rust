//! VPN Server binary.
//!
//! Accepts TLS connections from VPN clients and forwards packets
//! bidirectionally between clients and the TUN interface.
//!
//! # Usage
//!
//! ```bash
//! sudo RUST_LOG=info cargo run --bin server
//! ```

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use log::{debug, error, info, trace, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::{broadcast, Mutex, RwLock};
use tokio::time;
use tokio_rustls::server::TlsStream;
use vpn_rust::constants::{
    DEFAULT_SERVER_ADDR, DEFAULT_SERVER_PORT, KEEPALIVE_INTERVAL_SECS, KEEPALIVE_MARKER, VPN_SUBNET,
};
use vpn_rust::net::route;
use vpn_rust::net::tls::start_tls_server;
use vpn_rust::net::tun::TunInterface;

/// Client connection state
struct ClientConnection {
    writer: Arc<Mutex<WriteHalf<TlsStream<tokio::net::TcpStream>>>>,
    vpn_ip: std::net::Ipv4Addr,
}

/// Manages all connected clients
type ClientMap = Arc<RwLock<HashMap<SocketAddr, ClientConnection>>>;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting VPN server");

    // Create and configure TUN interface
    let tun = TunInterface::create_server().context("Failed to create TUN interface")?;
    tun.configure_server_ip()
        .context("Failed to configure server IP")?;

    info!("TUN device created: {}", tun.name);

    // Enable IP forwarding for packet routing
    route::enable_ip_forwarding().context("Failed to enable IP forwarding")?;

    // Set up NAT for outbound traffic
    let outbound_interface = route::get_default_interface().unwrap_or_else(|_| "eth0".to_string());
    info!("Using {} as outbound interface for NAT", outbound_interface);

    if let Err(e) = route::setup_nat(VPN_SUBNET, &outbound_interface) {
        warn!("Failed to set up NAT (may require iptables): {}", e);
    }

    let tun = Arc::new(tun);

    // Client tracking
    let clients: ClientMap = Arc::new(RwLock::new(HashMap::new()));

    // Broadcast channel for shutdown signaling
    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    // Start TLS server
    let addr = format!("{}:{}", DEFAULT_SERVER_ADDR, DEFAULT_SERVER_PORT);
    let server = start_tls_server(&addr)
        .await
        .context("Failed to start TLS server")?;

    info!("Server ready, waiting for connections...");

    // Spawn TUN reader task (reads from TUN, sends to clients)
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

        // Upgrade to TLS
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

        // Handle client in a separate task
        tokio::spawn(async move {
            if let Err(e) = handle_client(tls_stream, tun, clients.clone(), peer_addr).await {
                error!("Client {} error: {}", peer_addr, e);
            }

            // Remove client from map on disconnect
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

/// Task that reads packets from TUN and forwards to appropriate clients.
async fn tun_to_clients_task(tun: Arc<TunInterface>, clients: ClientMap) -> Result<()> {
    loop {
        // Read packet from TUN
        let packet = match tun.read_packet().await {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to read from TUN: {}", e);
                continue;
            }
        };

        // Parse destination IP from packet (IPv4 header)
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

        // Find client with matching VPN IP and get writer
        let target_client = {
            let clients_guard = clients.read().await;
            clients_guard
                .iter()
                .find(|(_, client)| client.vpn_ip == dest_ip)
                .map(|(addr, client)| (*addr, Arc::clone(&client.writer)))
        };

        // Send packet to target client (outside of lock)
        if let Some((peer_addr, writer)) = target_client {
            let mut writer_guard = writer.lock().await;
            if let Err(e) = send_packet(&mut writer_guard, &packet).await {
                error!("Failed to send packet to {}: {}", peer_addr, e);
            }
        }
    }
}

/// Handles a single client connection.
async fn handle_client(
    stream: TlsStream<tokio::net::TcpStream>,
    tun: Arc<TunInterface>,
    clients: ClientMap,
    peer_addr: SocketAddr,
) -> Result<()> {
    // Split stream for concurrent read/write
    let (read_half, write_half) = tokio::io::split(stream);
    let writer = Arc::new(Mutex::new(write_half));

    // Assign VPN IP to client (for now, hardcoded to 10.8.0.2)
    // In future, implement IP pool allocation
    let vpn_ip = std::net::Ipv4Addr::new(10, 8, 0, 2);

    // Register client
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

    // Spawn keepalive sender for this client
    let keepalive_writer = Arc::clone(&writer);
    let keepalive_task = tokio::spawn(async move {
        if let Err(e) = client_keepalive_task(keepalive_writer, peer_addr).await {
            debug!("Keepalive task for {} ended: {}", peer_addr, e);
        }
    });

    // Handle incoming packets from client
    let result = client_to_tun_task(read_half, tun, peer_addr).await;

    // Cancel keepalive task when client disconnects
    keepalive_task.abort();

    result
}

/// Task that sends periodic keepalive packets to a specific client.
async fn client_keepalive_task(
    writer: Arc<Mutex<WriteHalf<TlsStream<tokio::net::TcpStream>>>>,
    peer_addr: SocketAddr,
) -> Result<()> {
    let mut interval = time::interval(Duration::from_secs(KEEPALIVE_INTERVAL_SECS));

    loop {
        interval.tick().await;

        trace!("Sending keepalive to {}", peer_addr);

        let mut writer_guard = writer.lock().await;

        // Send keepalive (length = 0)
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

/// Task that reads packets from a client and writes them to TUN.
async fn client_to_tun_task(
    mut reader: ReadHalf<TlsStream<tokio::net::TcpStream>>,
    tun: Arc<TunInterface>,
    peer_addr: SocketAddr,
) -> Result<()> {
    loop {
        // Read length prefix (2 bytes, big-endian)
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

        // Check for keepalive packet (length = 0)
        if len == KEEPALIVE_MARKER as usize {
            trace!("Received keepalive from {}", peer_addr);
            continue;
        }

        debug!("Receiving {} bytes from {}", len, peer_addr);

        // Read packet data
        let mut buf = vec![0u8; len];
        reader
            .read_exact(&mut buf)
            .await
            .context("Failed to read packet data")?;

        // Write to TUN
        tun.write_packet(&buf)
            .await
            .context("Failed to write to TUN")?;

        debug!("Wrote {} bytes to TUN from {}", len, peer_addr);
    }
}

/// Sends a length-prefixed packet over the TLS stream.
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
