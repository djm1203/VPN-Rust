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
use crate::crypto::NodeIdentity;
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
pub async fn run_server(params: ServerParams) -> Result<()> {
    // The certificate's SAN must match the client's `--server-name` (default
    // "localhost"). A configurable SAN for non-local hostnames is future work.
    let identity =
        NodeIdentity::load_or_generate(&params.cert_path, &params.key_path, &params.server_name)
            .context("failed to load or generate server identity")?;

    let endpoint =
        quic::server_endpoint(params.bind, identity.certificate(), identity.private_key())
            .context("failed to start QUIC server")?;
    info!("QUIC server listening on {}", params.bind);
    info!(
        "server certificate at {} — pin this on the client",
        params.cert_path.display()
    );

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

    let offered = SessionParams {
        mtu: params.mtu,
        keepalive_secs: KEEPALIVE_INTERVAL_SECS as u16,
    };

    while let Some(incoming) = endpoint.accept().await {
        let tun = tun.clone();
        tokio::spawn(async move {
            match handle_server_connection(incoming, tun, offered).await {
                Ok(()) => info!("peer session ended"),
                Err(e) => warn!("peer session error: {e:#}"),
            }
        });
    }

    Ok(())
}

async fn handle_server_connection(
    incoming: quinn::Incoming,
    tun: Arc<SystemTun>,
    offered: SessionParams,
) -> Result<()> {
    let connection = incoming.await.context("QUIC handshake failed")?;
    let peer = connection.remote_address();
    info!("peer connected from {peer}");

    let params = server_handshake(&connection, offered).await?;
    info!("negotiated {params:?} with {peer}");

    let transport = Arc::new(QuicTransport::new(connection));
    pump(tun, transport).await
}

/// Run the VPN client: connect to the server and tunnel packets, reconnecting
/// with exponential backoff on failure.
pub async fn run_client(params: ClientParams) -> Result<()> {
    let server_cert = load_certificate(&params.server_cert_path)
        .context("failed to load pinned server certificate")?;
    let endpoint = quic::client_endpoint(server_cert).context("failed to start QUIC client")?;

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

    let initial_delay = Duration::from_millis(RECONNECT_INITIAL_DELAY_MS);
    let mut delay = initial_delay;
    let mut attempts = 0u32;

    loop {
        info!("connecting to {}", params.server_addr);
        match connect_once(&endpoint, &params).await {
            Ok(transport) => {
                // Connection established — reset the backoff before pumping.
                delay = initial_delay;
                attempts = 0;
                match pump(tun.clone(), transport).await {
                    Ok(()) => {
                        info!("session ended cleanly");
                        return Ok(());
                    }
                    Err(e) => warn!("session dropped: {e:#}"),
                }
            }
            Err(e) => warn!("connection failed: {e:#}"),
        }

        if params.no_reconnect {
            return Ok(());
        }
        attempts += 1;
        if params.max_reconnects != 0 && attempts >= params.max_reconnects {
            bail!("giving up after {attempts} reconnect attempts");
        }

        warn!("reconnecting in {delay:?} (attempt {attempts})");
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(Duration::from_millis(RECONNECT_MAX_DELAY_MS));
    }
}

/// Connect to the server and complete the control handshake, returning a ready
/// transport.
async fn connect_once(endpoint: &Endpoint, params: &ClientParams) -> Result<Arc<QuicTransport>> {
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
    let negotiated = client_handshake(&connection, requested).await?;
    info!("negotiated {negotiated:?}");

    Ok(Arc::new(QuicTransport::new(connection)))
}

/// Bidirectionally move packets between the TUN device and the QUIC transport
/// until either direction fails (e.g. the connection drops).
async fn pump(tun: Arc<SystemTun>, transport: Arc<QuicTransport>) -> Result<()> {
    // Buffer generously above the MTU to accommodate any TUN framing overhead.
    let buf_size = (tun.mtu() as usize).max(1500) + 64;

    let outbound = {
        let tun = tun.clone();
        let transport = transport.clone();
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
                }
            }
            #[allow(unreachable_code)]
            anyhow::Ok(())
        }
    };

    let inbound = async move {
        loop {
            let datagram = transport
                .recv_datagram()
                .await
                .context("QUIC datagram read failed")?;
            tun.send_packet(&datagram)
                .await
                .context("TUN write failed")?;
        }
        #[allow(unreachable_code)]
        anyhow::Ok(())
    };

    tokio::select! {
        r = outbound => r,
        r = inbound => r,
    }
}

/// Load a DER-encoded certificate from disk for pinning.
fn load_certificate(path: &std::path::Path) -> Result<CertificateDer<'static>> {
    let der = fs::read(path)
        .with_context(|| format!("failed to read certificate '{}'", path.display()))?;
    Ok(CertificateDer::from(der))
}
