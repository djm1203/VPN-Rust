//! The VPN session engine.
//!
//! The engine ties a [`TunDevice`] to a [`Transport`]: it reads IP packets from
//! the TUN and sends them to the peer as QUIC datagrams, and writes datagrams
//! received from the peer back to the TUN. Authentication uses pinned
//! certificates (the server presents its identity; the client pins it).
//!
//! Scope is **point-to-point** (decision D-10): the server handles a single
//! peer. Keepalive is handled by QUIC (see [`crate::transport::quic`]); the
//! client reconnects with exponential backoff.
//!
//! The engine publishes live telemetry (connection state, byte/packet counters,
//! RTT, negotiated parameters, peer address) into a shared [`LiveStats`] handle
//! that the TUI reads on each render tick (milestone M4).

pub mod stats;

pub use stats::{ConnectionState, LiveStats, StatsSnapshot};

use std::fs;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use quinn::rustls::pki_types::CertificateDer;
use quinn::{Connection, Endpoint};
use tracing::{info, warn};

use crate::constants::{
    KEEPALIVE_INTERVAL_SECS, RECONNECT_INITIAL_DELAY_MS, RECONNECT_MAX_DELAY_MS,
};
use crate::crypto::{certificate_fingerprint, NodeIdentity};
use crate::net::device::{SystemTun, TunDevice};
use crate::transport::control::{client_handshake, server_handshake, SessionParams};
use crate::transport::quic;
use crate::transport::{QuicTransport, Transport};

/// Parameters for running the server side of the tunnel.
pub struct ServerParams {
    /// UDP address to bind the QUIC endpoint to.
    pub bind: SocketAddr,
    /// Name placed in the generated certificate's SAN (must match the client's
    /// `--server-name`).
    pub server_name: String,
    /// TUN interface name.
    pub tun_name: String,
    /// Server IP within the VPN subnet.
    pub tun_ip: Ipv4Addr,
    /// VPN subnet prefix length (CIDR).
    pub prefix: u8,
    /// Inner MTU.
    pub mtu: u16,
    /// Path to the server's certificate (DER); generated if missing.
    pub cert_path: PathBuf,
    /// Path to the server's private key (DER); generated if missing.
    pub key_path: PathBuf,
    /// Outbound interface for server NAT (auto-detected when `None`).
    pub nat_interface: Option<String>,
}

/// Parameters for running the client side of the tunnel.
pub struct ClientParams {
    /// Server UDP address to connect to.
    pub server_addr: SocketAddr,
    /// Server name for certificate validation (must match the server cert SAN).
    pub server_name: String,
    /// Path to the pinned server certificate (DER).
    pub server_cert_path: PathBuf,
    /// TUN interface name.
    pub tun_name: String,
    /// Client IP within the VPN subnet.
    pub tun_ip: Ipv4Addr,
    /// VPN subnet prefix length (CIDR).
    pub prefix: u8,
    /// Inner MTU.
    pub mtu: u16,
    /// Disable automatic reconnection.
    pub no_reconnect: bool,
    /// Maximum reconnect attempts (0 = unlimited).
    pub max_reconnects: u32,
}

/// Run the VPN server: listen for a peer and tunnel packets.
pub async fn run_server(params: ServerParams, stats: Arc<LiveStats>) -> Result<()> {
    // The certificate's SAN must match the client's `--server-name` (default
    // "localhost"). A configurable SAN for non-local hostnames is future work.
    let identity =
        NodeIdentity::load_or_generate(&params.cert_path, &params.key_path, &params.server_name)
            .context("failed to load or generate server identity")?;

    let endpoint =
        quic::server_endpoint(params.bind, identity.certificate(), identity.private_key())
            .context("failed to start QUIC server")?;
    stats.set_endpoint(params.bind);
    info!("QUIC server listening on {}", params.bind);
    info!(
        "server certificate at {} — pin this on the client",
        params.cert_path.display()
    );
    info!("server certificate fingerprint: {}", identity.fingerprint());

    let tun = Arc::new(
        SystemTun::create(&params.tun_name, params.tun_ip, params.prefix, params.mtu)
            .context("failed to create server TUN device")?,
    );
    info!(
        "TUN device up: {} ({}/{})",
        tun.name(),
        params.tun_ip,
        params.prefix
    );

    // Configure host networking (IP forwarding + NAT). Kept alive for the
    // lifetime of the server; reverted on drop.
    let subnet = subnet_cidr(params.tun_ip, params.prefix);
    let mut netcfg = crate::net::netcfg::platform_default();
    if let Err(e) = netcfg.setup_server(&subnet, params.nat_interface.as_deref()) {
        warn!("failed to configure server networking (continuing without NAT): {e:#}");
    }

    let offered = SessionParams {
        mtu: params.mtu,
        keepalive_secs: KEEPALIVE_INTERVAL_SECS as u16,
    };

    while let Some(incoming) = endpoint.accept().await {
        let tun = tun.clone();
        let stats = stats.clone();
        tokio::spawn(async move {
            match handle_server_connection(incoming, tun, offered, stats.clone()).await {
                Ok(()) => info!("peer session ended"),
                Err(e) => warn!("peer session error: {e:#}"),
            }
            // The single peer has gone; return to a listening/disconnected state.
            stats.set_state(ConnectionState::Disconnected);
            stats.set_peer(None);
        });
    }

    Ok(())
}

async fn handle_server_connection(
    incoming: quinn::Incoming,
    tun: Arc<SystemTun>,
    offered: SessionParams,
    stats: Arc<LiveStats>,
) -> Result<()> {
    stats.set_state(ConnectionState::Handshaking);
    let connection = incoming.await.context("QUIC handshake failed")?;
    let peer = connection.remote_address();
    stats.set_peer(Some(peer));
    info!("peer connected from {peer}");

    let params = server_handshake(&connection, offered).await?;
    stats.set_negotiated(params);
    info!("negotiated {params:?} with {peer}");

    let transport = Arc::new(QuicTransport::new(connection));
    stats.set_state(ConnectionState::Connected);
    pump(tun, transport, stats).await
}

/// Run the VPN client: connect to the server and tunnel packets, reconnecting
/// with exponential backoff on failure.
pub async fn run_client(params: ClientParams, stats: Arc<LiveStats>) -> Result<()> {
    let server_cert = load_certificate(&params.server_cert_path)
        .context("failed to load pinned server certificate")?;
    info!(
        "pinned server fingerprint: {}",
        certificate_fingerprint(&server_cert)
    );
    let endpoint = quic::client_endpoint(server_cert).context("failed to start QUIC client")?;
    stats.set_endpoint(params.server_addr);

    let tun = Arc::new(
        SystemTun::create(&params.tun_name, params.tun_ip, params.prefix, params.mtu)
            .context("failed to create client TUN device")?,
    );
    info!(
        "TUN device up: {} ({}/{})",
        tun.name(),
        params.tun_ip,
        params.prefix
    );

    // Route the VPN subnet through the tunnel. Reverted on drop.
    let subnet = subnet_cidr(params.tun_ip, params.prefix);
    let mut netcfg = crate::net::netcfg::platform_default();
    if let Err(e) = netcfg.setup_client(&subnet, tun.name()) {
        warn!("failed to configure client routing (continuing): {e:#}");
    }

    let initial_delay = Duration::from_millis(RECONNECT_INITIAL_DELAY_MS);
    let mut delay = initial_delay;
    let mut attempts = 0u32;

    loop {
        info!("connecting to {}", params.server_addr);
        stats.set_state(if attempts == 0 {
            ConnectionState::Connecting
        } else {
            ConnectionState::Reconnecting
        });
        match connect_once(&endpoint, &params, &stats).await {
            Ok(transport) => {
                // Connection established — reset the backoff before pumping.
                delay = initial_delay;
                attempts = 0;
                stats.set_reconnect_attempts(0);
                stats.set_peer(Some(transport.remote_address()));
                stats.set_state(ConnectionState::Connected);
                match pump(tun.clone(), transport, stats.clone()).await {
                    Ok(()) => {
                        info!("session ended cleanly");
                        stats.set_state(ConnectionState::Disconnected);
                        return Ok(());
                    }
                    Err(e) => warn!("session dropped: {e:#}"),
                }
            }
            Err(e) => warn!("connection failed: {e:#}"),
        }

        stats.set_peer(None);
        if params.no_reconnect {
            stats.set_state(ConnectionState::Disconnected);
            return Ok(());
        }
        attempts += 1;
        stats.set_reconnect_attempts(attempts);
        stats.set_state(ConnectionState::Reconnecting);
        if params.max_reconnects != 0 && attempts >= params.max_reconnects {
            stats.set_state(ConnectionState::Disconnected);
            bail!("giving up after {attempts} reconnect attempts");
        }

        warn!("reconnecting in {delay:?} (attempt {attempts})");
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(Duration::from_millis(RECONNECT_MAX_DELAY_MS));
    }
}

/// Connect to the server and complete the control handshake, returning a ready
/// transport.
async fn connect_once(
    endpoint: &Endpoint,
    params: &ClientParams,
    stats: &Arc<LiveStats>,
) -> Result<Arc<QuicTransport>> {
    let connection: Connection = endpoint
        .connect(params.server_addr, &params.server_name)
        .context("invalid connection parameters")?
        .await
        .context("QUIC connection failed")?;
    info!("connected to {}", connection.remote_address());

    let requested = SessionParams {
        mtu: params.mtu,
        keepalive_secs: KEEPALIVE_INTERVAL_SECS as u16,
    };
    stats.set_state(ConnectionState::Handshaking);
    let negotiated = client_handshake(&connection, requested).await?;
    stats.set_negotiated(negotiated);
    info!("negotiated {negotiated:?}");

    Ok(Arc::new(QuicTransport::new(connection)))
}

/// Bidirectionally move packets between the TUN device and the QUIC transport
/// until either direction fails (e.g. the connection drops), updating
/// [`LiveStats`] with byte/packet counts and periodic RTT samples.
async fn pump(
    tun: Arc<SystemTun>,
    transport: Arc<QuicTransport>,
    stats: Arc<LiveStats>,
) -> Result<()> {
    // Buffer generously above the MTU to accommodate any TUN framing overhead.
    let buf_size = (tun.mtu() as usize).max(1500) + 64;

    let outbound = {
        let tun = tun.clone();
        let transport = transport.clone();
        let stats = stats.clone();
        async move {
            let mut buf = vec![0u8; buf_size];
            loop {
                let n = tun.recv_packet(&mut buf).await.context("TUN read failed")?;
                if n == 0 {
                    continue;
                }
                let datagram = Bytes::copy_from_slice(&buf[..n]);
                if let Err(e) = transport.send_datagram(datagram).await {
                    // Oversized datagram or a closed connection: drop and continue
                    // (the connection error surfaces via the inbound direction).
                    warn!("dropping outbound packet ({n} bytes): {e:#}");
                } else {
                    stats.record_sent(n);
                }
            }
            #[allow(unreachable_code)]
            anyhow::Ok(())
        }
    };

    let inbound = {
        let stats = stats.clone();
        let transport = transport.clone();
        async move {
            loop {
                let datagram = transport
                    .recv_datagram()
                    .await
                    .context("QUIC datagram read failed")?;
                stats.record_received(datagram.len());
                tun.send_packet(&datagram)
                    .await
                    .context("TUN write failed")?;
            }
            #[allow(unreachable_code)]
            anyhow::Ok(())
        }
    };

    // Periodically sample the connection RTT so the dashboard has a live gauge.
    // Never completes on its own; cancelled when a data direction returns.
    let sampler = {
        let transport = transport.clone();
        let stats = stats.clone();
        async move {
            let mut tick = tokio::time::interval(Duration::from_secs(1));
            loop {
                tick.tick().await;
                stats.set_rtt(transport.connection().rtt());
            }
            #[allow(unreachable_code)]
            anyhow::Ok(())
        }
    };

    tokio::select! {
        r = outbound => r,
        r = inbound => r,
        r = sampler => r,
    }
}

/// Compute the network address in CIDR form for `ip`/`prefix`
/// (e.g. `10.8.0.2/30` -> `10.8.0.0/30`).
fn subnet_cidr(ip: Ipv4Addr, prefix: u8) -> String {
    let p = (prefix.min(32)) as u32;
    let mask = if p == 0 { 0 } else { u32::MAX << (32 - p) };
    let network = Ipv4Addr::from(u32::from(ip) & mask);
    format!("{network}/{p}")
}

/// Load a DER-encoded certificate from disk for pinning.
fn load_certificate(path: &std::path::Path) -> Result<CertificateDer<'static>> {
    let der = fs::read(path)
        .with_context(|| format!("failed to read certificate '{}'", path.display()))?;
    Ok(CertificateDer::from(der))
}

#[cfg(test)]
mod tests {
    use super::subnet_cidr;
    use std::net::Ipv4Addr;

    #[test]
    fn subnet_cidr_masks_host_bits() {
        assert_eq!(subnet_cidr(Ipv4Addr::new(10, 8, 0, 2), 30), "10.8.0.0/30");
        assert_eq!(subnet_cidr(Ipv4Addr::new(10, 8, 0, 1), 24), "10.8.0.0/24");
        assert_eq!(
            subnet_cidr(Ipv4Addr::new(192, 168, 1, 55), 16),
            "192.168.0.0/16"
        );
        assert_eq!(subnet_cidr(Ipv4Addr::new(10, 0, 0, 5), 32), "10.0.0.5/32");
    }
}
