//! # VPN-Rust
//!
//! A learning-focused VPN client and server implementation in Rust.
//!
//! This library provides the core components for building a VPN:
//! - TUN interface management for packet capture/injection
//! - Transport abstraction with a QUIC (`quinn`) implementation
//! - Configuration file parsing (OpenVPN and TOML formats)
//! - Command-line interface
//!
//! ## Example
//!
//! ```no_run
//! use vpn_rust::net::tun::TunInterface;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create a TUN interface
//! let mut tun = TunInterface::create_client()?;
//! tun.configure_client_ip()?;
//!
//! // Read packets from the interface
//! let packet = tun.read_packet().await?;
//! println!("Received {} bytes", packet.len());
//! # Ok(())
//! # }
//! ```

pub mod cli;
pub mod config;
pub mod constants;
pub mod crypto;
pub mod engine;
pub mod net;
pub mod transport;
pub mod tui;
