//! TLS connection handling for secure VPN tunnels.
//!
//! This module provides TLS client and server functionality using the `rustls`
//! library for memory-safe TLS implementation.
//!
//! ## Features
//!
//! - Standard TLS connections (server authentication only)
//! - Mutual TLS (mTLS) with client certificate authentication
//! - Custom CA certificate support for self-signed certificates

use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{
    rustls::{
        server::AllowAnyAuthenticatedClient, Certificate, ClientConfig, OwnedTrustAnchor,
        PrivateKey, RootCertStore, ServerConfig, ServerName,
    },
    TlsAcceptor, TlsConnector,
};
use webpki_roots::TLS_SERVER_ROOTS;

use crate::constants::{
    CA_CERT_PATH, CLIENT_CERT_PATH, CLIENT_KEY_PATH, SERVER_CERT_PATH, SERVER_KEY_PATH,
};

/// Establishes a TLS connection to a remote server.
///
/// # Arguments
///
/// * `domain` - The domain name for TLS verification (e.g., "localhost")
/// * `addr` - The socket address to connect to (e.g., "127.0.0.1:4433")
///
/// # Returns
///
/// A TLS-wrapped TCP stream ready for secure communication.
///
/// # Errors
///
/// Returns an error if:
/// - TCP connection fails
/// - TLS handshake fails
/// - Domain name is invalid
///
/// # Example
///
/// ```no_run
/// # async fn example() -> anyhow::Result<()> {
/// use vpn_rust::net::tls::connect_tls;
///
/// let stream = connect_tls("example.com", "example.com:443").await?;
/// // Use stream for secure communication
/// # Ok(())
/// # }
/// ```
pub async fn connect_tls(
    domain: &str,
    addr: &str,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    debug!("Connecting to {} ({})", domain, addr);

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
        .with_context(|| format!("TCP connect to {} failed", addr))?;

    let server_name = ServerName::try_from(domain)
        .map_err(|_| anyhow::anyhow!("Invalid DNS name: {}", domain))?;

    let tls_stream = connector
        .connect(server_name, stream)
        .await
        .with_context(|| format!("TLS handshake with {} failed", domain))?;

    info!("TLS connection established to {}", addr);
    Ok(tls_stream)
}

/// TLS server components for accepting secure connections.
pub struct TlsServer {
    /// The TCP listener for accepting connections.
    pub listener: TcpListener,
    /// The TLS acceptor for upgrading TCP connections to TLS.
    pub acceptor: TlsAcceptor,
}

/// Starts a TLS server on the specified address.
///
/// Loads the server certificate and private key from the paths specified
/// in the constants module, then binds a TCP listener.
///
/// # Arguments
///
/// * `addr` - The socket address to bind to (e.g., "0.0.0.0:4433")
///
/// # Returns
///
/// A `TlsServer` containing both the TCP listener and TLS acceptor.
///
/// # Errors
///
/// Returns an error if:
/// - Certificate file cannot be read or parsed
/// - Private key file cannot be read or parsed
/// - TCP listener cannot bind to the address
///
/// # Example
///
/// ```no_run
/// # async fn example() -> anyhow::Result<()> {
/// use vpn_rust::net::tls::start_tls_server;
///
/// let server = start_tls_server("127.0.0.1:4433").await?;
///
/// loop {
///     let (tcp_stream, addr) = server.listener.accept().await?;
///     let tls_stream = server.acceptor.accept(tcp_stream).await?;
///     // Handle connection...
/// }
/// # Ok(())
/// # }
/// ```
pub async fn start_tls_server(addr: &str) -> Result<TlsServer> {
    info!("Starting TLS server on {}", addr);

    // Load certificate chain
    let certs = load_certs(SERVER_CERT_PATH)
        .with_context(|| format!("Failed to load certificate from {}", SERVER_CERT_PATH))?;
    debug!("Loaded {} certificate(s)", certs.len());

    // Load private key
    let key = load_private_key(SERVER_KEY_PATH)
        .with_context(|| format!("Failed to load private key from {}", SERVER_KEY_PATH))?;
    debug!("Loaded private key");

    // Build server config
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .context("Failed to build TLS server configuration")?;

    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind to {}", addr))?;

    info!("TLS server listening on {}", addr);

    Ok(TlsServer { listener, acceptor })
}

/// Loads certificates from a PEM file.
fn load_certs(path: &str) -> Result<Vec<Certificate>> {
    let file = File::open(path).with_context(|| format!("Cannot open {}", path))?;
    let mut reader = BufReader::new(file);

    let certs = rustls_pemfile::certs(&mut reader)
        .with_context(|| format!("Failed to parse certificates from {}", path))?
        .into_iter()
        .map(Certificate)
        .collect();

    Ok(certs)
}

/// Loads a private key from a PEM file.
///
/// Tries PKCS#8 format first, then falls back to RSA format.
fn load_private_key(path: &str) -> Result<PrivateKey> {
    let file = File::open(path).with_context(|| format!("Cannot open {}", path))?;
    let mut reader = BufReader::new(file);

    // Try PKCS#8 format first
    let keys = rustls_pemfile::pkcs8_private_keys(&mut reader)
        .with_context(|| format!("Failed to parse private key from {}", path))?;

    if let Some(key) = keys.into_iter().next() {
        return Ok(PrivateKey(key));
    }

    // Try RSA format as fallback
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let keys = rustls_pemfile::rsa_private_keys(&mut reader)?;

    keys.into_iter()
        .next()
        .map(PrivateKey)
        .ok_or_else(|| anyhow::anyhow!("No private key found in {}", path))
}

// =============================================================================
// Mutual TLS (mTLS) Support
// =============================================================================

/// TLS configuration options for client connections.
#[derive(Debug, Clone)]
pub struct ClientTlsConfig {
    /// Path to CA certificate for server verification.
    pub ca_cert_path: Option<String>,
    /// Path to client certificate for mTLS.
    pub client_cert_path: Option<String>,
    /// Path to client private key for mTLS.
    pub client_key_path: Option<String>,
    /// Whether to use system root certificates.
    pub use_system_roots: bool,
}

impl Default for ClientTlsConfig {
    fn default() -> Self {
        Self {
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
            use_system_roots: true,
        }
    }
}

impl ClientTlsConfig {
    /// Create a new client TLS config with mTLS enabled.
    pub fn with_mtls() -> Self {
        Self {
            ca_cert_path: Some(CA_CERT_PATH.to_string()),
            client_cert_path: Some(CLIENT_CERT_PATH.to_string()),
            client_key_path: Some(CLIENT_KEY_PATH.to_string()),
            use_system_roots: false,
        }
    }

    /// Create a config with custom certificate paths.
    pub fn custom(
        ca_cert: Option<&str>,
        client_cert: Option<&str>,
        client_key: Option<&str>,
    ) -> Self {
        Self {
            ca_cert_path: ca_cert.map(String::from),
            client_cert_path: client_cert.map(String::from),
            client_key_path: client_key.map(String::from),
            use_system_roots: ca_cert.is_none(),
        }
    }
}

/// TLS configuration options for server.
#[derive(Debug, Clone)]
pub struct ServerTlsConfig {
    /// Path to server certificate.
    pub cert_path: String,
    /// Path to server private key.
    pub key_path: String,
    /// Path to CA certificate for client verification (enables mTLS).
    pub ca_cert_path: Option<String>,
    /// Whether to require client certificates.
    pub require_client_auth: bool,
}

impl Default for ServerTlsConfig {
    fn default() -> Self {
        Self {
            cert_path: SERVER_CERT_PATH.to_string(),
            key_path: SERVER_KEY_PATH.to_string(),
            ca_cert_path: None,
            require_client_auth: false,
        }
    }
}

impl ServerTlsConfig {
    /// Create a server config with mTLS enabled.
    pub fn with_mtls() -> Self {
        Self {
            cert_path: SERVER_CERT_PATH.to_string(),
            key_path: SERVER_KEY_PATH.to_string(),
            ca_cert_path: Some(CA_CERT_PATH.to_string()),
            require_client_auth: true,
        }
    }

    /// Create a server config with custom paths.
    pub fn custom(cert_path: &str, key_path: &str, ca_cert_path: Option<&str>) -> Self {
        Self {
            cert_path: cert_path.to_string(),
            key_path: key_path.to_string(),
            ca_cert_path: ca_cert_path.map(String::from),
            require_client_auth: ca_cert_path.is_some(),
        }
    }
}

/// Establishes a TLS connection with configurable authentication.
///
/// This function supports:
/// - Standard TLS with system root certificates
/// - Custom CA certificate for self-signed server certificates
/// - Mutual TLS (mTLS) with client certificate authentication
///
/// # Arguments
///
/// * `domain` - The domain name for TLS verification
/// * `addr` - The socket address to connect to
/// * `config` - TLS configuration options
///
/// # Returns
///
/// A TLS-wrapped TCP stream ready for secure communication.
pub async fn connect_tls_with_config(
    domain: &str,
    addr: &str,
    config: &ClientTlsConfig,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    debug!("Connecting to {} ({}) with custom config", domain, addr);

    let mut root_store = RootCertStore::empty();

    // Add custom CA certificate if provided
    if let Some(ca_path) = &config.ca_cert_path {
        let ca_certs = load_certs(ca_path)
            .with_context(|| format!("Failed to load CA certificate from {}", ca_path))?;

        for cert in ca_certs {
            root_store
                .add(&cert)
                .with_context(|| "Failed to add CA certificate to root store")?;
        }
        debug!("Added custom CA certificate from {}", ca_path);
    }

    // Add system root certificates if enabled
    if config.use_system_roots {
        root_store.add_trust_anchors(TLS_SERVER_ROOTS.0.iter().map(|ta| {
            OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));
        debug!("Added system root certificates");
    }

    // Build client config
    let tls_config = if let (Some(cert_path), Some(key_path)) =
        (&config.client_cert_path, &config.client_key_path)
    {
        // mTLS: Load client certificate and key
        let client_certs = load_certs(cert_path)
            .with_context(|| format!("Failed to load client certificate from {}", cert_path))?;
        let client_key = load_private_key(key_path)
            .with_context(|| format!("Failed to load client key from {}", key_path))?;

        info!("Using client certificate authentication (mTLS)");

        ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_client_auth_cert(client_certs, client_key)
            .context("Failed to configure client certificate")?
    } else {
        // Standard TLS without client auth
        ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth()
    };

    let connector = TlsConnector::from(Arc::new(tls_config));
    let stream = TcpStream::connect(addr)
        .await
        .with_context(|| format!("TCP connect to {} failed", addr))?;

    let server_name = ServerName::try_from(domain)
        .map_err(|_| anyhow::anyhow!("Invalid DNS name: {}", domain))?;

    let tls_stream = connector
        .connect(server_name, stream)
        .await
        .with_context(|| format!("TLS handshake with {} failed", domain))?;

    info!("TLS connection established to {}", addr);
    Ok(tls_stream)
}

/// Starts a TLS server with configurable client authentication.
///
/// This function supports:
/// - Standard TLS (no client authentication)
/// - Mutual TLS (mTLS) requiring client certificates
///
/// # Arguments
///
/// * `addr` - The socket address to bind to
/// * `config` - Server TLS configuration
///
/// # Returns
///
/// A `TlsServer` containing both the TCP listener and TLS acceptor.
pub async fn start_tls_server_with_config(
    addr: &str,
    config: &ServerTlsConfig,
) -> Result<TlsServer> {
    info!(
        "Starting TLS server on {} (mTLS: {})",
        addr, config.require_client_auth
    );

    // Load server certificate chain
    let certs = load_certs(&config.cert_path)
        .with_context(|| format!("Failed to load certificate from {}", config.cert_path))?;
    debug!("Loaded {} server certificate(s)", certs.len());

    // Load server private key
    let key = load_private_key(&config.key_path)
        .with_context(|| format!("Failed to load private key from {}", config.key_path))?;
    debug!("Loaded server private key");

    // Build server config with or without client auth
    let server_config = if config.require_client_auth {
        // mTLS: Require client certificates
        let ca_path = config
            .ca_cert_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("CA certificate required for client authentication"))?;

        let mut client_root_store = RootCertStore::empty();
        let ca_certs = load_certs(ca_path)
            .with_context(|| format!("Failed to load CA certificate from {}", ca_path))?;

        for cert in ca_certs {
            client_root_store
                .add(&cert)
                .with_context(|| "Failed to add CA certificate for client verification")?;
        }

        let client_verifier = AllowAnyAuthenticatedClient::new(client_root_store);
        info!("Client certificate authentication enabled");

        ServerConfig::builder()
            .with_safe_defaults()
            .with_client_cert_verifier(Arc::new(client_verifier))
            .with_single_cert(certs, key)
            .context("Failed to build mTLS server configuration")?
    } else {
        // Standard TLS without client auth
        ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .context("Failed to build TLS server configuration")?
    };

    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind to {}", addr))?;

    info!("TLS server listening on {}", addr);

    Ok(TlsServer { listener, acceptor })
}

/// Extract client certificate information from a TLS connection.
///
/// Returns the Common Name (CN) from the client certificate if available.
pub fn get_client_cert_cn(
    tls_stream: &tokio_rustls::server::TlsStream<TcpStream>,
) -> Option<String> {
    let (_, server_conn) = tls_stream.get_ref();

    server_conn
        .peer_certificates()
        .and_then(|certs| certs.first())
        .and_then(|cert| {
            // Parse the certificate to extract CN
            match x509_parser::parse_x509_certificate(&cert.0) {
                Ok((_, parsed_cert)) => {
                    // Get the subject and find the CN
                    for rdn in parsed_cert.subject().iter() {
                        for attr in rdn.iter() {
                            if attr.attr_type() == &x509_parser::oid_registry::OID_X509_COMMON_NAME
                            {
                                if let Ok(cn) = attr.attr_value().as_str() {
                                    return Some(cn.to_string());
                                }
                            }
                        }
                    }
                    None
                }
                Err(e) => {
                    warn!("Failed to parse client certificate: {}", e);
                    None
                }
            }
        })
}
