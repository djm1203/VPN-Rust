//! Versioned control handshake over a reliable QUIC stream.
//!
//! When a connection is established, the peers perform a handshake on a
//! dedicated bidirectional QUIC stream *before* any datagrams flow:
//!
//! 1. The client opens a control stream and sends a [`ClientHello`] carrying the
//!    [`PROTOCOL_VERSION`] and its requested [`SessionParams`].
//! 2. The server replies with a [`ServerHello`] — either `Accepted` with the
//!    negotiated parameters, or `Rejected` with a reason (e.g. version mismatch).
//!
//! Parameters are negotiated conservatively (the smaller inner MTU, the smaller
//! keepalive interval) so both peers can honour them. Messages are encoded with
//! `postcard` and length-prefixed (`u32` big-endian) on the stream.
//!
//! The data plane (tunneled IP packets) is carried separately as QUIC datagrams
//! (see [`super::quic`]); this control stream is used only for signaling.

use anyhow::{bail, Context, Result};
use quinn::{Connection, RecvStream, SendStream};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::constants::{DEFAULT_MTU, KEEPALIVE_INTERVAL_SECS};

/// The control/wire protocol version. Peers must agree exactly (pre-1.0).
pub const PROTOCOL_VERSION: u16 = 1;

/// Maximum accepted size of a single encoded control message, in bytes.
const MAX_CONTROL_MSG_LEN: usize = 64 * 1024;

/// Session parameters negotiated during the handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionParams {
    /// Inner MTU for tunneled IP packets, in bytes.
    pub mtu: u16,
    /// Keepalive interval, in seconds.
    pub keepalive_secs: u16,
}

impl Default for SessionParams {
    fn default() -> Self {
        Self {
            mtu: DEFAULT_MTU as u16,
            keepalive_secs: KEEPALIVE_INTERVAL_SECS as u16,
        }
    }
}

impl SessionParams {
    /// Negotiate a common parameter set: the smaller MTU and keepalive interval
    /// so both peers can honour the result. Keepalive is clamped to at least 1s.
    pub fn negotiate(&self, other: &SessionParams) -> SessionParams {
        SessionParams {
            mtu: self.mtu.min(other.mtu),
            keepalive_secs: self.keepalive_secs.min(other.keepalive_secs).max(1),
        }
    }
}

/// Client → server opening message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientHello {
    /// The client's protocol version.
    pub version: u16,
    /// The parameters the client would like to use.
    pub requested: SessionParams,
}

/// Server → client handshake response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerHello {
    /// The handshake succeeded with the given negotiated parameters.
    Accepted {
        /// The server's protocol version (equals [`PROTOCOL_VERSION`]).
        version: u16,
        /// The parameters both peers will use.
        params: SessionParams,
    },
    /// The handshake was refused.
    Rejected {
        /// Human-readable reason (e.g. version mismatch).
        reason: String,
    },
}

/// Perform the client side of the handshake on `connection`.
///
/// Opens a control stream, sends a [`ClientHello`], and returns the negotiated
/// [`SessionParams`] on success.
pub async fn client_handshake(
    connection: &Connection,
    requested: SessionParams,
) -> Result<SessionParams> {
    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .context("failed to open control stream")?;

    let hello = ClientHello {
        version: PROTOCOL_VERSION,
        requested,
    };
    write_msg(&mut send, &hello).await?;
    send.finish().context("failed to finish control stream")?;

    match read_msg::<ServerHello>(&mut recv).await? {
        ServerHello::Accepted { version, params } => {
            if version != PROTOCOL_VERSION {
                bail!("server protocol version {version} does not match client {PROTOCOL_VERSION}");
            }
            Ok(params)
        }
        ServerHello::Rejected { reason } => bail!("server rejected handshake: {reason}"),
    }
}

/// Perform the server side of the handshake on `connection`.
///
/// Accepts the client's control stream, validates the protocol version, and
/// returns the negotiated [`SessionParams`]. `offered` is what the server is
/// willing to provide.
pub async fn server_handshake(
    connection: &Connection,
    offered: SessionParams,
) -> Result<SessionParams> {
    let (mut send, mut recv) = connection
        .accept_bi()
        .await
        .context("failed to accept control stream")?;

    let hello = read_msg::<ClientHello>(&mut recv).await?;

    if hello.version != PROTOCOL_VERSION {
        let reason = format!(
            "unsupported protocol version {} (server speaks {PROTOCOL_VERSION})",
            hello.version
        );
        let _ = write_msg(
            &mut send,
            &ServerHello::Rejected {
                reason: reason.clone(),
            },
        )
        .await;
        let _ = send.finish();
        bail!("rejected client handshake: {reason}");
    }

    let params = offered.negotiate(&hello.requested);
    write_msg(
        &mut send,
        &ServerHello::Accepted {
            version: PROTOCOL_VERSION,
            params,
        },
    )
    .await?;
    send.finish().context("failed to finish control stream")?;

    Ok(params)
}

/// Length-prefix and write a `postcard`-encoded message to a QUIC send stream.
async fn write_msg<T: Serialize>(send: &mut SendStream, msg: &T) -> Result<()> {
    let bytes = postcard::to_allocvec(msg).context("failed to encode control message")?;
    let len = u32::try_from(bytes.len()).context("control message too large to frame")?;
    send.write_all(&len.to_be_bytes())
        .await
        .context("failed to write control message length")?;
    send.write_all(&bytes)
        .await
        .context("failed to write control message body")?;
    Ok(())
}

/// Read a length-prefixed `postcard`-encoded message from a QUIC recv stream.
async fn read_msg<T: DeserializeOwned>(recv: &mut RecvStream) -> Result<T> {
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf)
        .await
        .context("failed to read control message length")?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_CONTROL_MSG_LEN {
        bail!("control message length {len} exceeds limit {MAX_CONTROL_MSG_LEN}");
    }

    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf)
        .await
        .context("failed to read control message body")?;
    postcard::from_bytes(&buf).context("failed to decode control message")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negotiate_takes_the_smaller_values() {
        let a = SessionParams {
            mtu: 1500,
            keepalive_secs: 10,
        };
        let b = SessionParams {
            mtu: 1400,
            keepalive_secs: 15,
        };
        let negotiated = a.negotiate(&b);
        assert_eq!(negotiated.mtu, 1400);
        assert_eq!(negotiated.keepalive_secs, 10);
    }

    #[test]
    fn negotiate_clamps_keepalive_to_at_least_one() {
        let a = SessionParams {
            mtu: 1500,
            keepalive_secs: 0,
        };
        let negotiated = a.negotiate(&a);
        assert_eq!(negotiated.keepalive_secs, 1);
    }
}
