//! Cryptographic identity and peer authentication for VPN-Rust.
//!
//! Authentication uses **pinned keypairs** rather than a CA/PKI (decision D-13):
//! each node has a persistent self-signed certificate ([`NodeIdentity`]), and a
//! peer is trusted by pinning its certificate. For self-signed certificates,
//! trusting the exact peer certificate (via a single-entry root store, see
//! [`crate::transport::quic`]) is equivalent to pinning that peer's identity.

pub mod identity;

pub use identity::NodeIdentity;
