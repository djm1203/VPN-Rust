//! Live session telemetry shared between the engine and the TUI.
//!
//! [`LiveStats`] is a lock-light, `Arc`-shared handle the engine writes to on
//! the hot path (cumulative byte/packet counters via relaxed atomics) and the
//! TUI reads from on each render tick. Rate/throughput history is *not* stored
//! here — the UI derives it by sampling the cumulative counters per tick, which
//! keeps the engine side to a few `fetch_add`s and avoids a second clock in the
//! data plane.
//!
//! The handful of non-atomic fields (peer address, negotiated parameters,
//! connection instant) change rarely — at most once per (re)connect — so they
//! live behind a `std::sync::Mutex` rather than the hot path.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::transport::control::SessionParams;

/// The connection lifecycle as surfaced to the dashboard.
///
/// Ordering follows the natural progression
/// `Disconnected → Connecting → Handshaking → Connected`, with `Reconnecting`
/// entered after a drop when reconnection is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectionState {
    Disconnected = 0,
    Connecting = 1,
    Handshaking = 2,
    Connected = 3,
    Reconnecting = 4,
}

impl ConnectionState {
    /// Human-readable label for the state.
    pub fn as_str(self) -> &'static str {
        match self {
            ConnectionState::Disconnected => "Disconnected",
            ConnectionState::Connecting => "Connecting",
            ConnectionState::Handshaking => "Handshaking",
            ConnectionState::Connected => "Connected",
            ConnectionState::Reconnecting => "Reconnecting",
        }
    }

    /// Whether the tunnel is carrying traffic.
    pub fn is_connected(self) -> bool {
        matches!(self, ConnectionState::Connected)
    }

    fn from_u8(v: u8) -> Self {
        match v {
            1 => ConnectionState::Connecting,
            2 => ConnectionState::Handshaking,
            3 => ConnectionState::Connected,
            4 => ConnectionState::Reconnecting,
            _ => ConnectionState::Disconnected,
        }
    }
}

/// A point-in-time, cheap-to-copy snapshot of [`LiveStats`] for rendering.
///
/// Reading the individual atomics is not globally consistent, but the fields
/// are independent gauges/counters where a one-tick skew is invisible to a
/// human, so a torn read has no practical effect.
#[derive(Debug, Clone)]
pub struct StatsSnapshot {
    pub state: ConnectionState,
    pub bytes_up: u64,
    pub bytes_down: u64,
    pub packets_up: u64,
    pub packets_down: u64,
    /// Smoothed path RTT, if the transport has reported one.
    pub rtt: Option<Duration>,
    /// How long the current session has been `Connected`.
    pub connected_for: Option<Duration>,
    /// Number of reconnection attempts since the last clean connect.
    pub reconnect_attempts: u32,
    /// Peer/socket address (remote peer for the client, last peer for the server).
    pub peer: Option<SocketAddr>,
    /// Parameters agreed during the control handshake.
    pub negotiated: Option<SessionParams>,
    /// True when running as the server.
    pub is_server: bool,
    /// Address the server bound / the server the client dials.
    pub endpoint: Option<SocketAddr>,
}

/// Shared, mostly-atomic live telemetry for a VPN session.
#[derive(Debug)]
pub struct LiveStats {
    state: AtomicU8,
    bytes_up: AtomicU64,
    bytes_down: AtomicU64,
    packets_up: AtomicU64,
    packets_down: AtomicU64,
    /// Last observed RTT in microseconds; `0` means "not yet known".
    rtt_micros: AtomicU64,
    reconnect_attempts: AtomicU32,
    is_server: bool,
    /// Instant the session last entered `Connected`.
    connected_at: Mutex<Option<Instant>>,
    peer: Mutex<Option<SocketAddr>>,
    negotiated: Mutex<Option<SessionParams>>,
    endpoint: Mutex<Option<SocketAddr>>,
}

impl LiveStats {
    /// Create a fresh handle in the `Disconnected` state.
    pub fn new(is_server: bool) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            state: AtomicU8::new(ConnectionState::Disconnected as u8),
            bytes_up: AtomicU64::new(0),
            bytes_down: AtomicU64::new(0),
            packets_up: AtomicU64::new(0),
            packets_down: AtomicU64::new(0),
            rtt_micros: AtomicU64::new(0),
            reconnect_attempts: AtomicU32::new(0),
            is_server,
            connected_at: Mutex::new(None),
            peer: Mutex::new(None),
            negotiated: Mutex::new(None),
            endpoint: Mutex::new(None),
        })
    }

    // --- hot path (engine writers) ---------------------------------------

    /// Record `n` bytes sent to the peer (one datagram).
    pub fn record_sent(&self, n: usize) {
        self.bytes_up.fetch_add(n as u64, Ordering::Relaxed);
        self.packets_up.fetch_add(1, Ordering::Relaxed);
    }

    /// Record `n` bytes received from the peer (one datagram).
    pub fn record_received(&self, n: usize) {
        self.bytes_down.fetch_add(n as u64, Ordering::Relaxed);
        self.packets_down.fetch_add(1, Ordering::Relaxed);
    }

    // --- lifecycle (engine writers) --------------------------------------

    /// Transition to a new connection state, stamping the connect instant when
    /// entering `Connected` and clearing it otherwise.
    pub fn set_state(&self, state: ConnectionState) {
        self.state.store(state as u8, Ordering::Relaxed);
        let mut at = self.connected_at.lock().unwrap();
        match state {
            ConnectionState::Connected => {
                if at.is_none() {
                    *at = Some(Instant::now());
                }
            }
            _ => *at = None,
        }
    }

    /// Store the smoothed RTT most recently reported by the transport.
    pub fn set_rtt(&self, rtt: Duration) {
        let micros = rtt.as_micros().min(u64::MAX as u128) as u64;
        self.rtt_micros.store(micros, Ordering::Relaxed);
    }

    /// Set the current peer/socket address.
    pub fn set_peer(&self, peer: Option<SocketAddr>) {
        *self.peer.lock().unwrap() = peer;
    }

    /// Set the negotiated session parameters.
    pub fn set_negotiated(&self, params: SessionParams) {
        *self.negotiated.lock().unwrap() = Some(params);
    }

    /// Set the local endpoint (server bind address or the dialed server).
    pub fn set_endpoint(&self, endpoint: SocketAddr) {
        *self.endpoint.lock().unwrap() = Some(endpoint);
    }

    /// Set the running reconnect-attempt counter.
    pub fn set_reconnect_attempts(&self, attempts: u32) {
        self.reconnect_attempts.store(attempts, Ordering::Relaxed);
    }

    // --- readers (TUI) ----------------------------------------------------

    /// The current connection state.
    pub fn state(&self) -> ConnectionState {
        ConnectionState::from_u8(self.state.load(Ordering::Relaxed))
    }

    /// Take a consistent-enough snapshot for one render frame.
    pub fn snapshot(&self) -> StatsSnapshot {
        let rtt_micros = self.rtt_micros.load(Ordering::Relaxed);
        let connected_for = self.connected_at.lock().unwrap().map(|at| at.elapsed());
        StatsSnapshot {
            state: self.state(),
            bytes_up: self.bytes_up.load(Ordering::Relaxed),
            bytes_down: self.bytes_down.load(Ordering::Relaxed),
            packets_up: self.packets_up.load(Ordering::Relaxed),
            packets_down: self.packets_down.load(Ordering::Relaxed),
            rtt: (rtt_micros > 0).then(|| Duration::from_micros(rtt_micros)),
            connected_for,
            reconnect_attempts: self.reconnect_attempts.load(Ordering::Relaxed),
            peer: *self.peer.lock().unwrap(),
            negotiated: *self.negotiated.lock().unwrap(),
            is_server: self.is_server,
            endpoint: *self.endpoint.lock().unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_accumulate() {
        let s = LiveStats::new(false);
        s.record_sent(100);
        s.record_sent(40);
        s.record_received(10);
        let snap = s.snapshot();
        assert_eq!(snap.bytes_up, 140);
        assert_eq!(snap.packets_up, 2);
        assert_eq!(snap.bytes_down, 10);
        assert_eq!(snap.packets_down, 1);
    }

    #[test]
    fn state_transitions_stamp_connected_instant() {
        let s = LiveStats::new(true);
        assert_eq!(s.state(), ConnectionState::Disconnected);
        assert!(s.snapshot().connected_for.is_none());

        s.set_state(ConnectionState::Connecting);
        assert!(s.snapshot().connected_for.is_none());

        s.set_state(ConnectionState::Connected);
        assert!(s.snapshot().connected_for.is_some());
        assert!(s.snapshot().is_server);

        s.set_state(ConnectionState::Reconnecting);
        assert!(s.snapshot().connected_for.is_none());
    }

    #[test]
    fn rtt_zero_reads_as_unknown() {
        let s = LiveStats::new(false);
        assert!(s.snapshot().rtt.is_none());
        s.set_rtt(Duration::from_micros(1500));
        assert_eq!(s.snapshot().rtt, Some(Duration::from_micros(1500)));
    }

    #[test]
    fn state_roundtrips_through_u8() {
        for st in [
            ConnectionState::Disconnected,
            ConnectionState::Connecting,
            ConnectionState::Handshaking,
            ConnectionState::Connected,
            ConnectionState::Reconnecting,
        ] {
            assert_eq!(ConnectionState::from_u8(st as u8), st);
        }
    }
}
