//! Cross-platform async TUN device abstraction.
//!
//! [`TunDevice`] is the packet-I/O seam (decision D-16): the engine reads and
//! writes raw IP packets through it without knowing the platform backend. The
//! [`SystemTun`] implementation is backed by [`tun_rs`], which provides async
//! TUN devices on Linux, macOS (`utun`) and Windows (`wintun`) — replacing the
//! previous Linux-only `from_raw_fd` + `AsyncFd` approach.

use std::io;
use std::net::Ipv4Addr;

use anyhow::{Context, Result};
use tun_rs::{AsyncDevice, DeviceBuilder};

/// A bidirectional stream of raw IP packets (Layer 3).
#[allow(async_fn_in_trait)]
pub trait TunDevice {
    /// Read a single IP packet into `buf`, returning its length in bytes.
    async fn recv_packet(&self, buf: &mut [u8]) -> io::Result<usize>;

    /// Write a single IP packet, returning the number of bytes written.
    async fn send_packet(&self, buf: &[u8]) -> io::Result<usize>;

    /// The interface name (may differ from the requested one, e.g. `utun3`).
    fn name(&self) -> &str;

    /// The configured MTU.
    fn mtu(&self) -> u16;
}

/// A system TUN device backed by [`tun_rs`].
pub struct SystemTun {
    device: AsyncDevice,
    name: String,
    mtu: u16,
}

impl SystemTun {
    /// Create and bring up a TUN device with the given name, IPv4 address, CIDR
    /// prefix length, and MTU.
    ///
    /// The requested `name` is best-effort: some platforms assign their own
    /// (e.g. macOS `utunN`), so the actual name is read back from the device.
    pub fn create(name: &str, ipv4: Ipv4Addr, prefix: u8, mtu: u16) -> Result<Self> {
        let device = DeviceBuilder::new()
            .name(name)
            .mtu(mtu)
            .ipv4(ipv4, prefix, None)
            .build_async()
            .with_context(|| format!("failed to create TUN device '{name}'"))?;

        let actual_name = device.name().unwrap_or_else(|_| name.to_string());

        Ok(Self {
            device,
            name: actual_name,
            mtu,
        })
    }
}

impl TunDevice for SystemTun {
    async fn recv_packet(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.device.recv(buf).await
    }

    async fn send_packet(&self, buf: &[u8]) -> io::Result<usize> {
        self.device.send(buf).await
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn mtu(&self) -> u16 {
        self.mtu
    }
}
