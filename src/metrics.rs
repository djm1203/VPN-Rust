//! Optional Prometheus-style metrics exporter.
//!
//! This module exposes the engine's [`LiveStats`](crate::engine::LiveStats) as
//! Prometheus text (exposition format v0.0.4) over a tiny, dependency-free
//! HTTP/1.1 endpoint. It is **off by default**: the process only starts the
//! exporter when the operator explicitly opts in (e.g. via a `--metrics-addr`
//! flag), and nothing here is reachable otherwise.
//!
//! ## Exposure considerations
//!
//! The exporter serves plaintext operational telemetry with **no
//! authentication**. On a VPN host this endpoint reveals liveness, throughput,
//! peer counts, and RTT — useful to an attacker profiling the tunnel. It should
//! therefore normally bind to a loopback address (`127.0.0.1`) and be scraped
//! locally or through a trusted side channel. Binding it to a public interface
//! deliberately publishes that telemetry to anyone who can reach the port, so
//! do so only behind separate access control.
//!
//! The [`serve`] endpoint handles one request per connection then closes it.
//! Only `GET /metrics` (and `GET /`) return metrics; any other path returns
//! `404`.

use std::fmt::Write as _;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::engine::{LiveStats, StatsSnapshot};

/// Prometheus `Content-Type` for the text exposition format v0.0.4.
const CONTENT_TYPE: &str = "text/plain; version=0.0.4";

/// Upper bound on the request bytes we read while looking for the end of the
/// HTTP headers. Requests larger than this are rejected — a metrics scrape has
/// no legitimate reason to send a large request.
const MAX_REQUEST_BYTES: usize = 8 * 1024;

/// Render a [`StatsSnapshot`] as a Prometheus text exposition document.
///
/// The output is self-contained: each metric is preceded by its `# HELP` and
/// `# TYPE` lines, followed by a single unlabelled sample. All values are plain
/// numbers, so the result is always valid exposition format.
///
/// `vpnrust_rtt_seconds` is emitted only when the snapshot carries an RTT;
/// there is no meaningful "0" for an unknown RTT, so the series is simply
/// absent until the transport reports one.
pub fn render_prometheus(snap: &StatsSnapshot) -> String {
    let mut out = String::with_capacity(1024);

    // Cumulative traffic counters.
    metric(
        &mut out,
        "vpnrust_bytes_up_total",
        "Total bytes sent to the peer.",
        "counter",
        snap.bytes_up,
    );
    metric(
        &mut out,
        "vpnrust_bytes_down_total",
        "Total bytes received from the peer.",
        "counter",
        snap.bytes_down,
    );
    metric(
        &mut out,
        "vpnrust_packets_up_total",
        "Total datagrams sent to the peer.",
        "counter",
        snap.packets_up,
    );
    metric(
        &mut out,
        "vpnrust_packets_down_total",
        "Total datagrams received from the peer.",
        "counter",
        snap.packets_down,
    );

    // Connection state as its numeric discriminant.
    metric(
        &mut out,
        "vpnrust_connection_state",
        "Connection state (0=Disconnected 1=Connecting 2=Handshaking 3=Connected 4=Reconnecting).",
        "gauge",
        snap.state as u8 as u64,
    );

    // RTT — only present when known.
    if let Some(rtt) = snap.rtt {
        gauge_f64(
            &mut out,
            "vpnrust_rtt_seconds",
            "Smoothed path round-trip time in seconds.",
            rtt.as_secs_f64(),
        );
    }

    // Session uptime (0 when not currently connected).
    gauge_f64(
        &mut out,
        "vpnrust_connected_seconds",
        "Seconds the current session has been connected (0 if not connected).",
        snap.connected_for.map(|d| d.as_secs_f64()).unwrap_or(0.0),
    );

    // Reconnect attempts since the last clean connect.
    metric(
        &mut out,
        "vpnrust_reconnect_attempts",
        "Reconnection attempts since the last clean connect.",
        "gauge",
        u64::from(snap.reconnect_attempts),
    );

    // Role: 1 for server, 0 for client.
    metric(
        &mut out,
        "vpnrust_role",
        "Process role (1 if running as the server, 0 if client).",
        "gauge",
        u64::from(snap.is_server),
    );

    out
}

/// Append a `# HELP`/`# TYPE` header and a single integer sample.
fn metric(out: &mut String, name: &str, help: &str, kind: &str, value: u64) {
    let _ = writeln!(out, "# HELP {name} {help}");
    let _ = writeln!(out, "# TYPE {name} {kind}");
    let _ = writeln!(out, "{name} {value}");
}

/// Append a `# HELP`/`# TYPE` header and a single floating-point gauge sample.
fn gauge_f64(out: &mut String, name: &str, help: &str, value: f64) {
    let _ = writeln!(out, "# HELP {name} {help}");
    let _ = writeln!(out, "# TYPE {name} gauge");
    let _ = writeln!(out, "{name} {value}");
}

/// Serve `render_prometheus` over a minimal HTTP/1.1 endpoint on `addr`.
///
/// This binds a [`TcpListener`] and loops forever accepting connections. For
/// each connection it reads and discards the request head (up to the blank line
/// terminating the headers, capped at [`MAX_REQUEST_BYTES`]), then responds
/// once and closes the connection:
///
/// - `GET /metrics` or `GET /` → `200 OK` with the current metrics.
/// - anything else → `404 Not Found`.
///
/// Per-connection failures are logged and skipped; the accept loop only returns
/// `Err` if the listener itself cannot be bound.
pub async fn serve(addr: SocketAddr, stats: Arc<LiveStats>) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let local = listener.local_addr().unwrap_or(addr);
    tracing::info!(%local, "metrics exporter listening (Prometheus, {CONTENT_TYPE})");

    loop {
        let (mut socket, peer) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => {
                tracing::warn!(error = %e, "metrics: accept failed");
                continue;
            }
        };

        let stats = Arc::clone(&stats);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(&mut socket, &stats).await {
                tracing::debug!(%peer, error = %e, "metrics: connection handling failed");
            }
        });
    }
}

/// Read the request head, then write exactly one HTTP response.
async fn handle_connection<S>(socket: &mut S, stats: &LiveStats) -> Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let request_line = read_request_head(socket).await?;
    let (method, path) = parse_request_line(&request_line);

    let response = if method != "GET" {
        http_response(
            405,
            "Method Not Allowed",
            "text/plain; charset=utf-8",
            "method not allowed\n",
        )
    } else if path == "/metrics" || path == "/" {
        let body = render_prometheus(&stats.snapshot());
        http_response(200, "OK", CONTENT_TYPE, &body)
    } else {
        http_response(404, "Not Found", "text/plain; charset=utf-8", "not found\n")
    };

    socket.write_all(response.as_bytes()).await?;
    socket.flush().await?;
    Ok(())
}

/// Read bytes until the end of the HTTP header block (`\r\n\r\n`), returning the
/// first (request) line. Reads are capped at [`MAX_REQUEST_BYTES`].
async fn read_request_head<S>(socket: &mut S) -> Result<String>
where
    S: AsyncReadExt + Unpin,
{
    let mut buf = Vec::with_capacity(256);
    let mut chunk = [0u8; 512];

    loop {
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() >= MAX_REQUEST_BYTES {
            anyhow::bail!("request head exceeded {MAX_REQUEST_BYTES} bytes");
        }
        let n = socket.read(&mut chunk).await?;
        if n == 0 {
            break; // peer closed before finishing the head; parse what we have
        }
        buf.extend_from_slice(&chunk[..n]);
    }

    let head = String::from_utf8_lossy(&buf);
    Ok(head.lines().next().unwrap_or("").to_string())
}

/// Split an HTTP request line into `(method, path)`, ignoring the version.
///
/// The path is truncated at the first `?` so a query string does not defeat the
/// route match.
fn parse_request_line(line: &str) -> (&str, &str) {
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let raw_path = parts.next().unwrap_or("");
    let path = raw_path.split('?').next().unwrap_or(raw_path);
    (method, path)
}

/// Build a complete HTTP/1.1 response with a correct `Content-Length` and a
/// `Connection: close` (one request per connection).
fn http_response(status: u16, reason: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {len}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        len = body.len(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ConnectionState;
    use std::time::Duration;

    fn sample(rtt: Option<Duration>) -> StatsSnapshot {
        StatsSnapshot {
            state: ConnectionState::Connected,
            bytes_up: 4096,
            bytes_down: 8192,
            packets_up: 12,
            packets_down: 20,
            rtt,
            connected_for: Some(Duration::from_secs(90)),
            reconnect_attempts: 2,
            peer: None,
            negotiated: None,
            is_server: true,
            endpoint: None,
        }
    }

    #[test]
    fn renders_expected_metric_names_and_types() {
        let out = render_prometheus(&sample(Some(Duration::from_millis(25))));

        for name in [
            "vpnrust_bytes_up_total",
            "vpnrust_bytes_down_total",
            "vpnrust_packets_up_total",
            "vpnrust_packets_down_total",
            "vpnrust_connection_state",
            "vpnrust_connected_seconds",
            "vpnrust_reconnect_attempts",
            "vpnrust_role",
        ] {
            assert!(out.contains(name), "missing metric {name}");
        }

        // Header lines are present.
        assert!(out.contains("# TYPE vpnrust_bytes_up_total counter"));
        assert!(out.contains("# TYPE vpnrust_connection_state gauge"));
        assert!(out.contains("# HELP vpnrust_bytes_up_total"));

        // Concrete numeric samples.
        assert!(out.contains("vpnrust_bytes_up_total 4096"));
        assert!(out.contains("vpnrust_connection_state 3")); // Connected == 3
        assert!(out.contains("vpnrust_role 1")); // server
        assert!(out.contains("vpnrust_reconnect_attempts 2"));
    }

    #[test]
    fn rtt_emitted_when_present() {
        let out = render_prometheus(&sample(Some(Duration::from_millis(50))));
        assert!(out.contains("# TYPE vpnrust_rtt_seconds gauge"));
        assert!(out.contains("vpnrust_rtt_seconds 0.05"));
    }

    #[test]
    fn rtt_omitted_when_absent() {
        let out = render_prometheus(&sample(None));
        assert!(
            !out.contains("vpnrust_rtt_seconds"),
            "rtt series must be absent when rtt is None"
        );
    }

    #[test]
    fn role_zero_for_client() {
        let mut snap = sample(None);
        snap.is_server = false;
        let out = render_prometheus(&snap);
        assert!(out.contains("vpnrust_role 0"));
    }
}
