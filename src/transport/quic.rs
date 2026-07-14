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

use anyhow::{Context, Result};
use bytes::Bytes;
use quinn::rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use quinn::{ClientConfig, Connection, Endpoint, ServerConfig};

use super::Transport;

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
// Development / test endpoint helpers (self-signed; replaced by M3 pinning).
// ---------------------------------------------------------------------------

/// Build a QUIC `ServerConfig` with a throwaway self-signed certificate.
///
/// Returns the config together with the DER-encoded certificate so a client can
/// trust (later: pin) it.
pub fn dev_server_config() -> Result<(ServerConfig, CertificateDer<'static>)> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
        .context("failed to generate self-signed certificate")?;
    let cert_der: CertificateDer<'static> = cert.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());

    let config = ServerConfig::with_single_cert(vec![cert_der.clone()], key.into())
        .context("failed to build QUIC server config")?;
    Ok((config, cert_der))
}

/// Bind a QUIC server [`Endpoint`] on `addr` with a throwaway self-signed cert.
///
/// Returns the endpoint and the server certificate (for the client to trust).
pub fn dev_server_endpoint(addr: SocketAddr) -> Result<(Endpoint, CertificateDer<'static>)> {
    let (config, cert) = dev_server_config()?;
    let endpoint = Endpoint::server(config, addr).context("failed to bind QUIC server endpoint")?;
    Ok((endpoint, cert))
}

/// Build a QUIC `ClientConfig` that trusts a specific server certificate.
pub fn dev_client_config(server_cert: CertificateDer<'static>) -> Result<ClientConfig> {
    let mut roots = quinn::rustls::RootCertStore::empty();
    roots
        .add(server_cert)
        .context("failed to add server certificate to root store")?;
    let config = ClientConfig::with_root_certificates(Arc::new(roots))
        .context("failed to build QUIC client config")?;
    Ok(config)
}

/// Bind an ephemeral-port QUIC client [`Endpoint`] that trusts `server_cert`.
pub fn dev_client_endpoint(server_cert: CertificateDer<'static>) -> Result<Endpoint> {
    let mut endpoint = Endpoint::client((Ipv4Addr::LOCALHOST, 0).into())
        .context("failed to bind QUIC client endpoint")?;
    endpoint.set_default_client_config(dev_client_config(server_cert)?);
    Ok(endpoint)
}
