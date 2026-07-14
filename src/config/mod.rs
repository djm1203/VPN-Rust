//! Configuration management for VPN-Rust.
//!
//! This module provides configuration file parsing and management,
//! supporting both TOML configuration files and command-line arguments.

mod error;
mod ovpn;
mod toml_config;

pub use error::ConfigError;
pub use ovpn::OVPNConfig;
pub use toml_config::{ClientConfig, Config, ServerConfig};
