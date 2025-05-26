///
///tls.rs
///
/// completes the TLS handshake over TCP
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{
    rustls::{
        Certificate, ClientConfig, OwnedTrustAnchor, PrivateKey, RootCertStore,
        ServerConfig, ServerName,
    },
    TlsAcceptor, TlsConnector,
};
use webpki_roots::TLS_SERVER_ROOTS;
use std::{fs::File, io::BufReader};

pub async fn connect_tls(
    domain: &str,
    addr: &str,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    let mut root_store = RootCertStore::empty();
    root_store.add_trust_anchors(TLS_SERVER_ROOTS.0.iter().map(|ta| {
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));

    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));
    let stream = TcpStream::connect(addr)
        .await
        .context("TCP connect failed")?;

    let server_name = ServerName::try_from(domain).context("Invalid DNS name")?;
    let tls_stream = connector
        .connect(server_name, stream)
        .await
        .context("TLS handshake failed")?;

    Ok(tls_stream)
}

pub async fn start_tls_server(addr: &str) -> Result<TcpListener> {
    let certs = {
        let mut reader = BufReader::new(File::open("certs/server.crt")?);
        rustls_pemfile::certs(&mut reader)?
            .into_iter()
            .map(Certificate)
            .collect()
    };

    let mut reader = BufReader::new(File::open("certs/server.key")?);
    let mut keys = rustls_pemfile::pkcs8_private_keys(&mut reader)?;
    let key = PrivateKey(keys.remove(0));

    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind(addr).await?;

    println!("TLS server listening on {}", addr);

    Ok(listener)
}
