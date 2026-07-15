---
title: "WIRE PROTOCOL"
project: vpn-rust
classification: high
created: 2026-07-14T00:00:00Z
updated: 2026-07-14T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: WireProtocol
author: Derek Martinez
---

# Wire Protocol — VPN-Rust

> **Status:** `PROTOCOL_VERSION = 1` (pre-1.0). The protocol is not yet stable;
> peers must run a **matched build** until it stabilizes (see
> [Versioning & compatibility](#versioning--compatibility) and backlog B-042).

This is the versioned specification of the on-the-wire protocol spoken between a
VPN-Rust server and client. It reflects the QUIC/UDP transport introduced in the
production pivot (decisions D-10…D-19 in
[/docs/state/DECISIONS.md](/docs/state/DECISIONS.md)) and is implemented in
`src/transport/quic.rs` (data plane) and `src/transport/control.rs` (control
plane). It supersedes the prototype's 2-byte length-prefixed TLS-over-TCP framing
(D-4), which is retained only as historical reference in
[/docs/architecture/ARCHITECTURE.md](/docs/architecture/ARCHITECTURE.md).

---

## 1. Transport

- **Bearer:** QUIC over UDP via [`quinn`], with TLS 1.3 (rustls under quinn)
  providing the session keys, confidentiality, and integrity. There is exactly
  **one QUIC connection** between the single client and the single server
  (point-to-point, D-10).
- **Data plane:** tunneled IP packets ride **unreliable QUIC datagrams**. Each
  datagram carries **exactly one encapsulated IP packet** and nothing else — no
  length prefix, no framing header. QUIC's own framing delimits the datagram.
  Using unreliable datagrams (rather than a QUIC stream) deliberately avoids
  *reliability-over-reliability* and head-of-line blocking: a lost tunneled
  packet is simply lost, exactly as on a physical link, and the inner transport
  (TCP/QUIC inside the tunnel) recovers it end-to-end.
- **Control plane:** a **single reliable bidirectional QUIC stream**, opened by
  the client immediately after the QUIC handshake and **before any datagram
  flows**, carries the versioned handshake (§3). It is used only for signaling.

Congestion control, loss detection for streams, packet-number encryption, and
key updates are all provided by QUIC/TLS 1.3 and are out of scope for this spec.

### 1.1 Datagram payload

```
QUIC datagram
┌───────────────────────────────────────────────┐
│  one raw IP packet (L3), verbatim              │
│  (IPv4/IPv6 header + payload)                  │
└───────────────────────────────────────────────┘
```

The payload length equals the datagram length. The maximum size is bounded by
the path's QUIC `max_datagram_size` (queried from the connection); the negotiated
**inner MTU** (§3.3) is chosen to keep an encapsulated packet within that bound so
datagrams are never dropped for being oversized. Packets that would exceed the
peer's current datagram limit are dropped by the sender and logged, not fragmented
(PMTU handling is tracked as B-016).

---

## 2. Endianness & encoding

- Multi-byte integers on the **control stream length prefix** are **big-endian**.
- Control message bodies are encoded with [`postcard`] (a compact,
  `serde`-based binary format). Field order and types below define the schema;
  `postcard` is not self-describing, so **both peers must agree on the exact
  struct/enum layout** — this is the core reason for the matched-build rule.

---

## 3. Control handshake

Constant: `PROTOCOL_VERSION: u16 = 1` (`src/transport/control.rs`).

### 3.1 Framing

Every control message is length-prefixed on the reliable QUIC stream:

```
┌──────────────┬─────────────────────────────────────┐
│  4 bytes     │  N bytes                             │
│  length (N)  │  postcard-encoded message body       │
│  BE u32      │                                      │
└──────────────┴─────────────────────────────────────┘
```

- The length prefix is a **4-byte big-endian `u32`** giving the body length `N`.
- A single control message body may not exceed **65 536 bytes (64 KiB)**
  (`MAX_CONTROL_MSG_LEN`); a receiver that reads a larger prefix aborts the
  connection.

### 3.2 Messages

The client sends exactly one `ClientHello`; the server replies with exactly one
`ServerHello`. Each side then finishes (half-closes) its direction of the stream.

```rust
// Client → server, first message on the control stream.
struct ClientHello {
    version:   u16,           // client's PROTOCOL_VERSION
    requested: SessionParams, // parameters the client would like
}

// Server → client, the reply.
enum ServerHello {
    Accepted { version: u16, params: SessionParams }, // negotiated result
    Rejected { reason: String },                       // e.g. version mismatch
}

// Parameters carried in both directions.
struct SessionParams {
    mtu:            u16, // inner MTU for tunneled IP packets, bytes
    keepalive_secs: u16, // application keepalive interval, seconds
}
```

`postcard` encodes the `ServerHello` enum with a leading variant index
(`0 = Accepted`, `1 = Rejected`) followed by that variant's fields.

### 3.3 Negotiation rules

The server computes the effective session parameters from its **offered**
parameters and the client's **requested** parameters
(`SessionParams::negotiate`):

- **MTU:** take the **smaller** of the two `mtu` values (so both peers can carry
  any packet the other sends).
- **Keepalive:** take the **smaller** of the two `keepalive_secs`, then **clamp to
  a minimum of 1 second**.
- **Version:** an **exact match is required** pre-1.0. If
  `ClientHello.version != PROTOCOL_VERSION`, the server replies
  `Rejected { reason }` and closes; the client aborts. On `Accepted`, the client
  additionally re-checks that `version == PROTOCOL_VERSION` and aborts on
  mismatch.

The default `SessionParams` are `mtu = 1300` (D-19 — chosen to stay safely under
the QUIC datagram limit on typical paths) and `keepalive_secs = 10`
(`KEEPALIVE_INTERVAL_SECS`). Both are overridable via the `--mtu` CLI flag.

### 3.4 Post-handshake

After a successful handshake both peers apply the negotiated `SessionParams` and
begin exchanging QUIC datagrams (§1.1). No further control messages are defined in
version 1; the control stream remains open but idle. Future control signaling
(rekey hints, stats, graceful teardown) is reserved for later versions.

---

## 4. Keepalive & idle timeout

Liveness is handled by QUIC's transport layer, tuned from the legacy constants
(`src/transport/quic.rs`, `tuned_transport_config`):

| Parameter                  | Source constant             | Value |
|----------------------------|-----------------------------|-------|
| QUIC keep-alive interval   | `KEEPALIVE_INTERVAL_SECS`   | 10 s  |
| QUIC max idle timeout      | `CONNECTION_TIMEOUT_SECS`   | 30 s  |

- The **keep-alive interval** makes QUIC emit a PING roughly every 10 s of
  inactivity so NAT bindings stay open and a dead peer is noticed.
- The **max idle timeout** tears the connection down after 30 s with no traffic
  in either direction. The negotiated `keepalive_secs` is signaled to the peer
  for symmetry; the enforced interval/timeout come from the constants above.
- On the client, a dropped or timed-out connection triggers **exponential-backoff
  reconnect** — `RECONNECT_INITIAL_DELAY_MS` (1000 ms) doubling up to
  `RECONNECT_MAX_DELAY_MS` (30 000 ms) — unless `--no-reconnect` is set.

---

## 5. Authentication

Authentication is by **pinned self-signed certificates — no CA, no PKI** (D-13,
refined by D-19). See [/docs/operations/THREAT_MODEL.md](/docs/operations/THREAT_MODEL.md)
for the full trust discussion.

- The server presents a **self-signed** certificate generated with `rcgen`
  (`src/crypto/identity.rs`). Its SAN equals the server's `--server-name`
  (default `localhost`).
- The client **pins that exact certificate**: it builds a rustls
  `RootCertStore` containing only the server's DER certificate
  (`client_config` in `src/transport/quic.rs`) and accepts a server only if it
  presents that certificate. There are no `webpki-roots` and no chain building.
- The peer's **identity is the SHA-256 fingerprint of the certificate DER**
  (`sha256:ab:cd:…`), printed by both `keygen` and the running server for
  **out-of-band (TOFU) verification**. The client logs the fingerprint of the
  cert it pinned so the operator can compare.
- The QUIC handshake still performs the standard TLS hostname check against the
  certificate SAN, so the client's `--server-name` must match the server's.

---

## 6. Connect → handshake → datagram flow

```
 Client                                                          Server
   │                                                               │
   │  1. UDP: QUIC + TLS 1.3 handshake                             │
   │─────────────────────────────────────────────────────────────▶│
   │        server presents self-signed cert                       │
   │◀─────────────────────────────────────────────────────────────│
   │        client pins exact cert (single-entry root store)       │
   │        + verifies SAN == --server-name                        │
   │                                                               │
   │  2. open bidirectional control stream                         │
   │─────────────────────────────────────────────────────────────▶│
   │     ClientHello { version=1, requested{mtu,keepalive} }       │
   │       [u32 BE len | postcard body]                            │
   │─────────────────────────────────────────────────────────────▶│
   │                                        negotiate: min(mtu),   │
   │                                        min(keepalive)≥1,      │
   │                                        require version==1     │
   │     ServerHello::Accepted { version=1, params }               │
   │◀─────────────────────────────────────────────────────────────│
   │       (or ::Rejected { reason } → both sides abort)           │
   │                                                               │
   │  3. data plane: one IP packet per unreliable QUIC datagram    │
   │═════════════════════════════════════════════════════════════▶│
   │◀═════════════════════════════════════════════════════════════│
   │        (bidirectional; QUIC keep-alive every 10 s;            │
   │         idle-timeout 30 s → client backoff-reconnects)        │
   │                                                               │
```

---

## 7. Versioning & compatibility

- **`PROTOCOL_VERSION` is a single `u16`** negotiated as an **exact match**. There
  is no minimum-version or feature-flag negotiation yet; any change to the
  handshake messages, the `SessionParams` layout, or the datagram payload
  contract is a **breaking protocol change** and MUST bump `PROTOCOL_VERSION`.
- **Matched build until stabilization (B-042).** Because control bodies are
  `postcard`-encoded (non-self-describing), server and client must be built from
  the **same protocol version** — in practice, the same release. Do not mix
  versions.
- **SemVer discipline.** Until the protocol reaches 1.0, treat every
  `PROTOCOL_VERSION` bump as a major, compatibility-breaking change and release
  server and client together. Once stabilized, `PROTOCOL_VERSION` will gain
  backward-compatibility guarantees (negotiated feature ranges) so mixed-version
  peers can interoperate within a major version.
- **Change checklist for any wire change:** (1) bump `PROTOCOL_VERSION`;
  (2) update §3 message definitions and §1 datagram contract here; (3) bump the
  crate version per SemVer; (4) note the change in
  [/docs/state/CHANGELOG.md](/docs/state/CHANGELOG.md).

[`quinn`]: https://docs.rs/quinn
[`postcard`]: https://docs.rs/postcard
