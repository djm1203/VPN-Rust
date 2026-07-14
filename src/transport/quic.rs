//! QUIC implementation of the [`Transport`](super::Transport) trait.
//!
//! Uses [`quinn`] for QUIC/UDP with TLS 1.3. Tunneled IP packets are carried as
//! unreliable QUIC datagrams.
//!
//! The `dev_*` helpers below generate a throwaway self-signed certificate and
//! are intended for the loopback integration test and early development. They
//! will be replaced in milestone M3 by pinned-keypair authentication
//! (SPKI fingerprint verification via `rcgen`), at which point the peer
//! certificate is pinned rather than trusted as a root.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use bytes::Bytes;
use quinn::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use quinn::{ClientConfig, Connection, Endpoint, ServerConfig, TransportConfig};

use super::Transport;
use crate::constants::{CONNECTION_TIMEOUT_SECS, KEEPALIVE_INTERVAL_SECS};

/// Shared QUIC transport tuning: application-level keepalive plus an idle
/// timeout so dead peers are detected (mirrors the legacy keepalive constants).
fn tuned_transport_config() -> Arc<TransportConfig> {
    let mut tc = TransportConfig::default();
    tc.keep_alive_interval(Some(Duration::from_secs(KEEPALIVE_INTERVAL_SECS)));
    let idle = Duration::from_secs(CONNECTION_TIMEOUT_SECS)
        .try_into()
        .expect("connection timeout fits in a QUIC idle timeout");
    tc.max_idle_timeout(Some(idle));
    Arc::new(tc)
}

/// A QUIC-based transport wrapping an established [`quinn::Connection`].
pub struct QuicTransport {
    connection: Connection,
}

impl QuicTransport {
    /// Wrap an established QUIC connection.
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    /// The peer's socket address.
    pub fn remote_address(&self) -> SocketAddr {
        self.connection.remote_address()
    }

    /// Access the underlying connection (e.g. for control streams).
    pub fn connection(&self) -> &Connection {
        &self.connection
    }
}

impl Transport for QuicTransport {
    async fn send_datagram(&self, datagram: Bytes) -> Result<()> {
        self.connection
            .send_datagram(datagram)
            .context("failed to send QUIC datagram")?;
        Ok(())
    }

    async fn recv_datagram(&self) -> Result<Bytes> {
        let datagram = self
            .connection
            .read_datagram()
            .await
            .context("failed to read QUIC datagram")?;
        Ok(datagram)
    }

    fn max_datagram_size(&self) -> Option<usize> {
        self.connection.max_datagram_size()
    }
}

// ---------------------------------------------------------------------------
// Endpoint builders
// ---------------------------------------------------------------------------

/// Build a QUIC `ServerConfig` from a certificate and private key.
pub fn server_config(
    cert: CertificateDer<'static>,
    key: PrivateKeyDer<'static>,
) -> Result<ServerConfig> {
    let mut config = ServerConfig::with_single_cert(vec![cert], key)
        .context("failed to build QUIC server config")?;
    config.transport_config(tuned_transport_config());
    Ok(config)
}

/// Bind a QUIC server [`Endpoint`] on `addr` with the given identity.
pub fn server_endpoint(
    addr: SocketAddr,
    cert: CertificateDer<'static>,
    key: PrivateKeyDer<'static>,
) -> Result<Endpoint> {
    let config = server_config(cert, key)?;
    Endpoint::server(config, addr).context("failed to bind QUIC server endpoint")
}

/// Build a QUIC `ClientConfig` that pins (trusts only) `server_cert`.
///
/// For a self-signed peer certificate this pins the peer's identity: the client
/// accepts only a server presenting exactly this certificate.
pub fn client_config(server_cert: CertificateDer<'static>) -> Result<ClientConfig> {
    let mut roots = quinn::rustls::RootCertStore::empty();
    roots
        .add(server_cert)
        .context("failed to pin server certificate")?;
    let mut config = ClientConfig::with_root_certificates(Arc::new(roots))
        .context("failed to build QUIC client config")?;
    config.transport_config(tuned_transport_config());
    Ok(config)
}

/// Bind an ephemeral-port QUIC client [`Endpoint`] that pins `server_cert`.
pub fn client_endpoint(server_cert: CertificateDer<'static>) -> Result<Endpoint> {
    let mut endpoint = Endpoint::client((Ipv4Addr::LOCALHOST, 0).into())
        .context("failed to bind QUIC client endpoint")?;
    endpoint.set_default_client_config(client_config(server_cert)?);
    Ok(endpoint)
}

/// Bind a QUIC server endpoint with a throwaway self-signed identity, returning
/// the endpoint and its certificate (for a client to pin). For tests and quick
/// local experiments.
pub fn dev_server_endpoint(addr: SocketAddr) -> Result<(Endpoint, CertificateDer<'static>)> {
    let identity = crate::crypto::NodeIdentity::generate("localhost")?;
    let cert = identity.certificate();
    let endpoint = server_endpoint(addr, identity.certificate(), identity.private_key())?;
    Ok((endpoint, cert))
}
