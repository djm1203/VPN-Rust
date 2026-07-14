//! Cryptographic identity and peer authentication for VPN-Rust.
//!
//! Authentication uses **pinned keypairs** rather than a CA/PKI (decision D-13):
//! each node has a persistent self-signed certificate ([`NodeIdentity`]), and a
//! peer is trusted by pinning its certificate. For self-signed certificates,
//! trusting the exact peer certificate (via a single-entry root store, see
//! [`crate::transport::quic`]) is equivalent to pinning that peer's identity.

pub mod identity;

pub use identity::NodeIdentity;

use quinn::rustls::pki_types::CertificateDer;
use sha2::{Digest, Sha256};

/// Compute a human-readable SHA-256 fingerprint of a DER-encoded certificate,
/// formatted as `sha256:ab:cd:…` (lowercase hex, colon-separated).
///
/// Because the pinned peer is trusted by its exact certificate, this fingerprint
/// uniquely identifies the pinned identity and can be compared out-of-band
/// (trust-on-first-use) to confirm the operator pinned the right peer.
pub fn certificate_fingerprint(cert: &CertificateDer<'_>) -> String {
    let digest = Sha256::digest(cert.as_ref());
    let hex: Vec<String> = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("sha256:{}", hex.join(":"))
}
