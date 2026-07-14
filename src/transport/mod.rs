//! Wire transport abstraction for the VPN data plane.
//!
//! The VPN engine exchanges opaque *datagrams* (encapsulated IP packets) with a
//! peer over a [`Transport`]. Datagrams are delivered best-effort, like UDP — an
//! upper layer (or the guest's own transport, e.g. TCP) is responsible for
//! reliability. This avoids reliability-over-reliability ("TCP meltdown").
//!
//! The production implementation is QUIC (see [`quic`]), which provides
//! encryption (TLS 1.3), congestion control, and connection migration while
//! carrying the tunneled packets as unreliable QUIC datagrams.
//!
//! This trait is the seam (decision D-16) that keeps the engine independent of
//! the concrete wire protocol, so it can be tested against a loopback QUIC pair
//! without a TUN device or root privileges.

pub mod quic;

use anyhow::Result;
use bytes::Bytes;

pub use quic::QuicTransport;

/// A bidirectional, best-effort datagram channel between two VPN peers.
#[allow(async_fn_in_trait)]
pub trait Transport {
    /// Send a single datagram to the peer.
    ///
    /// Fails if the datagram exceeds the peer's currently negotiated maximum
    /// size (see [`Transport::max_datagram_size`]) or the connection is closed.
    async fn send_datagram(&self, datagram: Bytes) -> Result<()>;

    /// Await and return the next datagram received from the peer.
    async fn recv_datagram(&self) -> Result<Bytes>;

    /// The maximum datagram payload the peer will currently accept, if known.
    ///
    /// Returns `None` if datagrams are not supported by the peer or the size is
    /// not yet known (e.g. before the handshake completes).
    fn max_datagram_size(&self) -> Option<usize>;
}
