---
title: "ARCHITECTURE"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Architecture
author: Derek Martinez
---

# Architecture — VPN-Rust

VPN-Rust is being taken from a learning prototype to **production-ready personal software**:
a **QUIC/UDP** point-to-point tunnel between machines the operator owns — one **Linux server**
plus **Linux / macOS / Windows** clients — authenticated with **pinned keypairs** and driven
from a **TUI control dashboard**.

This document has two parts:
- **[Current architecture](#current-architecture-as-is-prototype)** — what exists today (the
  `v0.1.0` prototype), retained as an accurate as-is reference.
- **[Target architecture](#target-architecture-production--quic-p2p)** — the design we are
  building toward, per decisions D-10…D-18 in [DECISIONS.md](../state/DECISIONS.md) and the
  milestones in [../planning/EXECUTION_PLAN.md](../planning/EXECUTION_PLAN.md).

---

# Current architecture (as-is prototype)

The current build (`vpn-rust` v0.1.0, Rust 2021) establishes a TLS-encrypted Layer 3 tunnel
between two hosts using TUN virtual interfaces, framing raw IP packets with a simple
length-prefixed wire protocol and forwarding them over a single TCP connection. It targets
**Linux only** at runtime (see the AsyncFd constraint below). Milestone M1/M2/M3 will supersede
the transport, TUN, and auth layers respectively.

## Language Breakdown

| Language | % |
|----------|---|
| Rust     | ~100% (~5,195 LOC across 40 tracked files) |
| Shell / config (certs, CI, helper scripts) | trace |

## Build System

**Cargo** (Rust toolchain, edition 2021).

Manifests: `Cargo.toml`, `Cargo.lock`.

Key build facts:
- Single crate `vpn-rust` producing a library plus binaries. `default-run = "vpn-rust"`.
- Binaries: unified `vpn-rust` (`src/main.rs`, with `server`/`client` subcommands via
  clap) and the standalone `src/bin/server.rs` and `src/bin/client.rs`.
- Common commands: `cargo build`, `cargo build --release`, `cargo test`, `cargo clippy`,
  `cargo fmt`. TUN operations require root / `CAP_NET_ADMIN` at runtime.
- Note: `winapi` is listed under `[target.'cfg(windows)'.dependencies]`, but the crate
  does **not** build on Windows (see Data Flow / constraints).

## Frameworks

| Concern | Library | Version | Role |
|---------|---------|---------|------|
| Async runtime | Tokio (`full` features) | 1.38 | Async I/O, task scheduling, `AsyncFd`, timers |
| TLS | rustls | 0.21 | Pure-Rust TLS 1.2/1.3 (no OpenSSL) |
| Async TLS | tokio-rustls | 0.24 | rustls + Tokio integration (`TlsConnector`, `TlsAcceptor`) |
| TLS trust store | webpki-roots | 0.22.6 | Baked-in root CA anchors for server verification |
| Cert parsing | x509-parser / rustls-pemfile | 0.16 / 1.0 | X.509/PEM loading; client-cert CN extraction |
| TUN device | tun | 0.6 | Virtual L3 network interface creation |
| CLI | clap | 4.5 (derive) | `vpn-rust server|client`, `--config`, `--verbose` |
| TUI | ratatui / crossterm | 0.25 / 0.27 | Terminal dashboard (status, stats, logs) |
| Config | serde / toml | 1.0 / 0.8 | TOML config deserialization |
| Errors | anyhow | 1.0 | Application errors with `.with_context()` |
| Logging | log / env_logger | 0.4 / 0.11 | `RUST_LOG`-driven structured logging |

## Module Structure

```
src/
├── lib.rs              # Library root; exports cli, config, constants, net, tui
├── main.rs             # Unified `vpn-rust` binary (server/client subcommands)
├── cli.rs              # clap argument/subcommand definitions
├── constants.rs        # Central constants (TUN names, IPs, port, timeouts, protocol markers)
├── bin/
│   ├── server.rs       # Standalone VPN server binary
│   └── client.rs       # Standalone VPN client binary
├── config/
│   ├── mod.rs          # Re-exports: OVPNConfig, Config, ClientConfig, ServerConfig
│   ├── ovpn.rs         # OpenVPN `.ovpn` file parsing
│   └── toml_config.rs  # TOML configuration model + parsing
├── net/
│   ├── mod.rs          # Network module root; exports clients, route, security, tls, tun
│   ├── tun.rs          # TunInterface: create/configure TUN, async read/write, cleanup
│   ├── tls.rs          # connect_tls, start_tls_server, mTLS (ClientTlsConfig/ServerTlsConfig), get_client_cert_cn
│   ├── route.rs        # Route setup + NAT / IP forwarding via ip / iptables / sysctl
│   ├── security.rs     # SecurityManager: kill switch, DNS-leak & IPv6-leak prevention (iptables)
│   └── clients.rs      # IpPool (DHCP-like allocation), ClientStats, ClientConnection, ClientManager
└── tui/
    ├── mod.rs          # TUI module root
    ├── app.rs          # Application state (connection status, traffic, duration, logs)
    ├── ui.rs           # ratatui widget layout / rendering
    └── runner.rs       # crossterm event loop driving the TUI
```

## Data Flow

### Topology

- **Transport:** single TCP connection wrapped in TLS, server listening on **port 4433**.
- **Server TUN:** `rustvpn0` = `10.8.0.1/30` (gateway).
- **Client TUN:** `rustvpn1` = `10.8.0.2/30`.
- **VPN subnet:** `10.8.0.0/30`. MTU 1500.

### Wire protocol

Each frame is a **2-byte big-endian u16 length prefix** followed by that many bytes
of raw IP packet:

```
┌──────────┬────────────────────────┐
│ 2 bytes  │        N bytes         │
│ length   │   IP packet (L3)       │
│ (BE u16) │                        │
└──────────┴────────────────────────┘
```

A length field of **0** is the **keepalive marker** (`KEEPALIVE_MARKER`) — a frame
carrying no packet body, sent every `KEEPALIVE_INTERVAL_SECS` (10s) to keep the
tunnel alive and detect dead peers (`CONNECTION_TIMEOUT_SECS` = 30s).

### Egress path (client app → internet)

```
┌─────────┐   IP pkt   ┌──────────┐  length   ┌───────────┐   TLS record   ┌─────────────┐
│ Local   │──────────▶ │  TUN     │──frame──▶ │  TLS      │──over TCP:4433─▶│ TLS acceptor│
│ apps    │            │ rustvpn1 │           │  stream   │                 │  (server)   │
└─────────┘            └──────────┘           └───────────┘                 └──────┬──────┘
                                                                                   │ decode frame
                                                                                   ▼
                                                                            ┌────────────┐
                                                                            │   TUN      │
                                                                            │  rustvpn0  │
                                                                            └─────┬──────┘
                                                                                  │ NAT / IP forward
                                                                                  ▼  (ip/iptables/sysctl)
                                                                             Internet
```

### Ingress path (internet → client app)

The reverse: return packets arrive on the server host, are routed back through
`rustvpn0` (reverse NAT), re-framed with the length prefix, written to the client's
TLS stream, decoded, and written to the client's `rustvpn1`, where the kernel delivers
them to the originating application.

### End-to-end ASCII overview

```
┌──────────────────────┐              ┌──────────────────────┐
│      VPN Client      │    TLS       │      VPN Server      │
│  ┌────────────────┐  │   Tunnel     │  ┌────────────────┐  │
│  │  TUN rustvpn1  │  │◀────────────▶│  │  TUN rustvpn0  │  │
│  │  10.8.0.2/30   │  │  TCP :4433   │  │  10.8.0.1/30   │  │
│  └───────┬────────┘  │              │  └───────┬────────┘  │
│          ▼           │              │          ▼           │
│  ┌────────────────┐  │              │  ┌────────────────┐  │
│  │ Packet loop /  │  │              │  │ Packet loop /  │  │
│  │ length framing │  │              │  │ length framing │  │
│  └───────┬────────┘  │              │  └───────┬────────┘  │
│          ▼           │              │          ▼           │
│  ┌────────────────┐  │              │  ┌────────────────┐  │
│  │  TLS stream    │──┼──────────────┼──│  TLS acceptor  │  │
│  └────────────────┘  │              │  └───────┬────────┘  │
└──────────────────────┘              │          ▼           │
                                      │   NAT / forward →    │
                                      │      Internet        │
                                      └──────────────────────┘
```

## TLS Handshake Flow

- **Client** (`connect_tls`): opens a TCP connection to the server, builds a rustls
  `ClientConfig`. For standard connections it trusts `webpki-roots::TLS_SERVER_ROOTS`;
  a custom CA (`certs/ca.crt`) can be supplied for self-signed servers. The domain
  name argument is used for certificate verification. `TlsConnector` performs the
  handshake and yields an async TLS stream.
- **Server** (`start_tls_server`): loads `certs/server.crt` + `certs/server.key`,
  builds a rustls `ServerConfig`, binds a `TcpListener`, and wraps accepted connections
  with `TlsAcceptor`.
- **mTLS** (`ClientTlsConfig` / `ServerTlsConfig`): the server can require client
  certificates via `AllowAnyAuthenticatedClient` against a trusted CA. After the
  handshake, `get_client_cert_cn` extracts the client certificate's Common Name (CN),
  which becomes the client's identity for the `ClientManager`.

Security note: certificates are **self-signed**, with no revocation/CRL support. The
trust model assumes trusted endpoints; the code is not production-audited.

## Packet Tunnel Loop

Per connection, both peers run a bidirectional loop:
1. Read an IP packet from the local TUN (`TunInterface::read_packet`, up to
   `PACKET_BUFFER_SIZE` = 1504 bytes), prepend the 2-byte length, write to the TLS stream.
2. Concurrently, read framed data from the TLS stream, strip the length prefix, and
   write the packet to the local TUN.
3. A timer emits keepalive frames (length 0) every 10s; a peer that receives nothing
   for 30s treats the connection as dead.

## Keepalive & Reconnect

- **Keepalive:** length-0 frames every `KEEPALIVE_INTERVAL_SECS` (10s); dead-peer
  detection at `CONNECTION_TIMEOUT_SECS` (30s).
- **Reconnect (client):** exponential backoff from `RECONNECT_INITIAL_DELAY_MS`
  (1000ms) up to `RECONNECT_MAX_DELAY_MS` (30000ms), re-establishing the TLS tunnel
  after a drop.

## Multi-Client IP Assignment

The server's `ClientManager` (`net/clients.rs`) owns an `IpPool` built from the VPN
subnet CIDR (DHCP-like allocation, excluding network/gateway/broadcast addresses).
On connect, each client is registered (identity from mTLS cert CN or a generated id),
allocated a VPN IP, and tracked with per-client `ClientStats` (atomic byte/packet
counters). `ip_to_socket` maps VPN IPs to sockets for routing; optional
client-to-client routing is gated by `allow_client_routing`. Idle clients can be
reaped via `remove_idle_clients`. (Note: at the `/30` default subnet only a single
client IP is available; a wider subnet is required for true multi-client use.)

## Platform Constraint (Linux-only)

`net/tun.rs` wraps the TUN device's raw file descriptor with `std::fs::File`
(`from_raw_fd`) and Tokio's `AsyncFd`, both of which are Unix/Linux-only. As a result
**Linux is the only supported build and runtime target**; the crate does not build on
Windows despite the `winapi` target dependency. TUN creation and `ip`/`iptables`/
`sysctl` invocations require root or `CAP_NET_ADMIN`.

---

# Target architecture (production — QUIC P2P)

The production design replaces three subsystems (transport, TUN, auth) behind **trait seams**
so platforms and wire formats are swappable and testable (D-16).

## Component overview

```
                         ┌────────────────────────────────────────┐
                         │              VPN engine                 │
   TUI (control) ◀──────▶│  session state machine · stats/events   │◀──────▶ tracing logs
   connect/disconnect    │  keepalive · reconnect · config         │
   live graphs, panels   └───────┬───────────────┬────────────┬────┘
                                 │               │            │
                         ┌───────▼──────┐ ┌──────▼──────┐ ┌───▼──────────────┐
                         │  Transport   │ │  TunDevice  │ │ NetConfigurator  │
                         │  (trait)     │ │  (trait)    │ │ (trait)          │
                         ├──────────────┤ ├─────────────┤ ├──────────────────┤
                         │ quinn (QUIC) │ │ tun-rs:     │ │ Linux: netlink/ip│
                         │  · datagrams │ │  linux      │ │ macOS: route/scutil
                         │  · control   │ │  utun (mac) │ │ Windows: netsh/  │
                         │    stream    │ │  wintun     │ │   iphlpapi       │
                         └──────────────┘ └─────────────┘ └──────────────────┘
```

- **`Transport` (trait):** send/recv datagrams + a control channel. Production impl = `quinn`.
- **`TunDevice` (trait):** async read/write of IP packets. Production impls via `tun-rs`.
- **`NetConfigurator` (trait):** set/tear-down addresses, routes, NAT, DNS — with rollback on
  drop/crash. Per-OS implementations.
- **VPN engine:** owns the connection state machine, keepalive/reconnect, and emits a
  stats/event stream consumed by the TUI (D-17) and `tracing` (D-14).

## Transport — QUIC over UDP (`quinn`)

- **Data plane:** tunneled IP packets ride **QUIC datagrams** (unreliable) — no
  reliability-over-reliability, no head-of-line blocking; QUIC provides congestion control and
  encryption (TLS 1.3 via rustls).
- **Control plane:** a **reliable bidirectional QUIC stream** carries a **versioned handshake**
  (protocol version, negotiated inner MTU, keepalive interval) and control messages.
- **MTU:** inner MTU is clamped to avoid UDP fragmentation over the path; PMTU handled explicitly.
- **Reconnect:** leverages QUIC connection migration / 0-RTT resumption where possible, with the
  existing exponential-backoff policy as the fallback.

```
┌──────────────┐   IP pkt    ┌──────────┐  QUIC datagram   ┌──────────┐   IP pkt   ┌──────────┐
│ client apps  │──────────▶  │ TunDevice│──(UDP, encrypted)▶│ TunDevice│──────────▶ │ NAT /    │
│              │             │ (client) │  ◀── control ──▶  │ (server) │            │ forward  │
└──────────────┘             └──────────┘   stream (QUIC)   └──────────┘            └────┬─────┘
                                                                                        ▼
                                                                                    Internet
```

## Authentication — pinned keypairs (no CA)

- Each node generates a keypair/self-cert with **`rcgen`** (`vpn keygen`).
- A **custom rustls certificate verifier** pins the peer by **SPKI fingerprint** — no
  `webpki-roots`, no CA chain. Trust-on-first-use with out-of-band fingerprint confirmation is
  offered; the fingerprint is shown in the CLI/TUI.
- Private key material is `zeroize`d; key files are permission-checked.

## Addressing (point-to-point)

Single peer, so a `/30` or `/31` point-to-point link is sufficient; addresses are **config-driven**
(the hardcoded constants become defaults). The multi-client `ClientManager`/`IpPool` is collapsed
to a single-peer session.

## Platform strategy

- **Server:** Linux only.
- **Clients:** Linux, macOS (`utun`), Windows (`wintun`) — all through the `TunDevice` and
  `NetConfigurator` traits. This closes the current Linux-only build constraint (R-1).

## Target module shape (indicative)

```
vpncore/            # library crate (optional workspace split, B-009)
├── engine/         # session state machine, keepalive, reconnect, stats/events
├── transport/      # Transport trait + quinn impl
├── tun/            # TunDevice trait + tun-rs backends
├── netcfg/         # NetConfigurator trait + per-OS impls
├── crypto/         # keygen (rcgen), SPKI pinning verifier, zeroize
└── config/         # validated TOML model
vpn/                # thin binary: CLI + wiring
vpn-tui/            # ratatui control dashboard
```

## Observability

`tracing` spans/events throughout; the TUI subscribes to the same stream for its log viewer, and
counters/histograms feed the live graphs and an optional export (M5).

## Decisions

See [DECISIONS.md](../state/DECISIONS.md) — prototype decisions D-1…D-9, production pivot
D-10…D-18.
