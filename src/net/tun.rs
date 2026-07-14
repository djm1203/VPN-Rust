//! TUN interface management for Linux.
//!
//! This module provides a wrapper around Linux TUN devices for capturing
//! and injecting IP packets. TUN devices operate at Layer 3 (IP level).
//!
//! # Requirements
//!
//! - Linux kernel with TUN/TAP support
//! - Root privileges or CAP_NET_ADMIN capability
//!
//! # Example
//!
//! ```no_run
//! use vpn_rust::net::tun::TunInterface;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut tun = TunInterface::create_client()?;
//! tun.configure_client_ip()?;
//!
//! loop {
//!     let packet = tun.read_packet().await?;
//!     // Process packet...
//!     tun.write_packet(&packet).await?;
//! }
//! # }
//! ```

use anyhow::{Context, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::process::Command;
use tokio::io::unix::AsyncFd;
use tracing::{debug, info, trace, warn};
use tun::{Configuration, Device as DeviceTrait};

use crate::constants::{
    CLIENT_IP, CLIENT_IP_CIDR, CLIENT_TUN_NAME, DEFAULT_MTU, INTERFACE_CLEANUP_DELAY_MS,
    PACKET_BUFFER_SIZE, SERVER_IP, SERVER_IP_CIDR, SERVER_TUN_NAME,
};

/// A wrapper around a Linux TUN device with async I/O support.
///
/// The TUN interface is automatically cleaned up when dropped.
pub struct TunInterface {
    /// The name of the TUN interface (e.g., "rustvpn0").
    pub name: String,
    /// The async file descriptor for packet I/O.
    inner: AsyncFd<File>,
}

impl TunInterface {
    /// Creates a new TUN interface for the VPN server.
    ///
    /// The interface is named "rustvpn0" and configured for Layer 3 (IP) operation.
    /// Any existing interface with the same name is cleaned up first.
    ///
    /// # Returns
    ///
    /// A new `TunInterface` ready for IP configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The TUN device cannot be created (permissions, kernel support)
    /// - The async file descriptor cannot be created
    pub fn create_server() -> Result<Self> {
        Self::create_interface(SERVER_TUN_NAME, "server")
    }

    /// Creates a new TUN interface for the VPN client.
    ///
    /// The interface is named "rustvpn1" and configured for Layer 3 (IP) operation.
    /// Any existing interface with the same name is cleaned up first.
    ///
    /// # Returns
    ///
    /// A new `TunInterface` ready for IP configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The TUN device cannot be created (permissions, kernel support)
    /// - The async file descriptor cannot be created
    pub fn create_client() -> Result<Self> {
        Self::create_interface(CLIENT_TUN_NAME, "client")
    }

    /// Internal helper to create a TUN interface with the given name.
    fn create_interface(interface_name: &str, role: &str) -> Result<Self> {
        info!("Creating {} TUN interface: {}", role, interface_name);

        Self::cleanup_existing_interface(interface_name)?;

        let mut config = Configuration::default();
        config
            .name(interface_name)
            .mtu(DEFAULT_MTU)
            .layer(tun::Layer::L3);

        let dev = tun::create(&config)
            .with_context(|| format!("Failed to create {} TUN interface", role))?;

        let name = dev
            .name()
            .context("Failed to get interface name")?
            .to_string();

        debug!("Created TUN interface: {}", name);

        // Convert to async file descriptor
        let raw_fd = dev.into_raw_fd();
        // SAFETY: We own the file descriptor from the tun device
        let file = unsafe { File::from_raw_fd(raw_fd) };
        let async_fd = AsyncFd::new(file).context("Failed to create async file descriptor")?;

        Ok(Self {
            name,
            inner: async_fd,
        })
    }

    /// Cleans up an existing interface if it exists.
    fn cleanup_existing_interface(interface_name: &str) -> Result<()> {
        let output = Command::new("ip")
            .args(["link", "show", interface_name])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                warn!("Cleaning up existing interface: {}", interface_name);

                let _ = Command::new("ip")
                    .args(["link", "set", interface_name, "down"])
                    .output();

                let _ = Command::new("ip")
                    .args(["link", "delete", interface_name])
                    .output();

                // Brief delay to ensure kernel cleanup
                std::thread::sleep(std::time::Duration::from_millis(INTERFACE_CLEANUP_DELAY_MS));
            }
        }

        Ok(())
    }

    /// Configures the server IP address on the interface.
    ///
    /// Sets the IP address to 10.8.0.1/30 and brings the interface up.
    /// Also adds a route to reach the client (10.8.0.2).
    ///
    /// # Errors
    ///
    /// Returns an error if the IP address cannot be assigned or the interface
    /// cannot be brought up.
    pub fn configure_server_ip(&self) -> Result<()> {
        self.configure_ip(SERVER_IP_CIDR, CLIENT_IP, "server")
    }

    /// Configures the client IP address on the interface.
    ///
    /// Sets the IP address to 10.8.0.2/30 and brings the interface up.
    /// Also adds a route to reach the server (10.8.0.1).
    ///
    /// # Errors
    ///
    /// Returns an error if the IP address cannot be assigned or the interface
    /// cannot be brought up.
    pub fn configure_client_ip(&self) -> Result<()> {
        self.configure_ip(CLIENT_IP_CIDR, SERVER_IP, "client")
    }

    /// Internal helper to configure IP address.
    fn configure_ip(&self, ip_cidr: &str, peer_ip: &str, role: &str) -> Result<()> {
        info!("Configuring {} IP {} on {}", role, ip_cidr, self.name);

        // Add IP address
        let add_result = Command::new("ip")
            .args(["addr", "add", ip_cidr, "dev", &self.name])
            .output()
            .context("Failed to execute ip addr add")?;

        if !add_result.status.success() {
            let error = String::from_utf8_lossy(&add_result.stderr);
            return Err(anyhow::anyhow!("Failed to add {} IP: {}", role, error));
        }

        // Bring interface up
        let up_result = Command::new("ip")
            .args(["link", "set", &self.name, "up"])
            .output()
            .context("Failed to execute ip link set up")?;

        if !up_result.status.success() {
            let error = String::from_utf8_lossy(&up_result.stderr);
            return Err(anyhow::anyhow!(
                "Failed to bring {} interface up: {}",
                role,
                error
            ));
        }

        // Add route to peer (ignore errors - route may already exist)
        let route_result = Command::new("ip")
            .args([
                "route",
                "add",
                &format!("{}/32", peer_ip),
                "dev",
                &self.name,
            ])
            .output();

        if let Ok(result) = route_result {
            if result.status.success() {
                debug!("Added route to {} via {}", peer_ip, self.name);
            }
        }

        info!("TUN interface {} configured with {}", self.name, ip_cidr);

        if tracing::enabled!(tracing::Level::DEBUG) {
            self.log_interface_config();
        }

        Ok(())
    }

    /// Logs the current interface configuration.
    fn log_interface_config(&self) {
        if let Ok(output) = Command::new("ip")
            .args(["addr", "show", &self.name])
            .output()
        {
            if output.status.success() {
                let config = String::from_utf8_lossy(&output.stdout);
                for line in config.lines() {
                    if line.trim().starts_with("inet") {
                        debug!("  {}", line.trim());
                    }
                }
            }
        }
    }

    /// Reads a packet from the TUN interface.
    ///
    /// This method waits asynchronously until a packet is available.
    ///
    /// # Returns
    ///
    /// The raw IP packet data.
    ///
    /// # Errors
    ///
    /// Returns an error if reading from the interface fails.
    pub async fn read_packet(&self) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; PACKET_BUFFER_SIZE];

        let guard = self
            .inner
            .readable()
            .await
            .context("Failed to wait for readable")?;

        let n = guard
            .get_inner()
            .read(&mut buf)
            .context("Failed to read from TUN")?;

        buf.truncate(n);

        trace!(
            "TUN read {} bytes: {:02x?}...",
            n,
            &buf[..std::cmp::min(20, n)]
        );

        Ok(buf)
    }

    /// Writes a packet to the TUN interface.
    ///
    /// # Arguments
    ///
    /// * `data` - The raw IP packet to write.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the interface fails.
    pub async fn write_packet(&self, data: &[u8]) -> Result<()> {
        let guard = self
            .inner
            .writable()
            .await
            .context("Failed to wait for writable")?;

        guard
            .get_inner()
            .write_all(data)
            .context("Failed to write to TUN")?;

        trace!(
            "TUN write {} bytes: {:02x?}...",
            data.len(),
            &data[..std::cmp::min(20, data.len())]
        );

        Ok(())
    }
}

impl Drop for TunInterface {
    fn drop(&mut self) {
        debug!("Cleaning up TUN interface: {}", self.name);

        let _ = Command::new("ip")
            .args(["link", "set", &self.name, "down"])
            .output();

        let _ = Command::new("ip")
            .args(["link", "delete", &self.name])
            .output();

        info!("TUN interface {} removed", self.name);
    }
}
