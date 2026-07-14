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
//! use std::net::Ipv4Addr;
//! use vpn_rust::net::device::{SystemTun, TunDevice};
//!
//! # fn example() -> anyhow::Result<()> {
//! // Create a cross-platform TUN device.
//! let tun = SystemTun::create("rustvpn0", Ipv4Addr::new(10, 8, 0, 1), 30, 1300)?;
//! println!("device {} up, mtu {}", tun.name(), tun.mtu());
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
