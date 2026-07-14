//! Network modules for VPN communication.
//!
//! This module contains the core networking components:
//! - [`device`] - Cross-platform async TUN device abstraction (`TunDevice`)
//! - [`clients`] - Multi-client management and IP assignment
//! - [`route`] - IP routing and NAT configuration
//! - [`security`] - Security features (kill switch, DNS/IPv6 leak prevention)
//! - [`tls`] - TLS connection handling for secure tunnels
//! - [`tun`] - TUN interface management for packet capture/injection

pub mod device;

pub mod clients;
pub mod route;
pub mod security;
pub mod tls;
pub mod tun;
