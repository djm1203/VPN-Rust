---
title: "CAPABILITIES"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Capabilities
author: Derek Martinez
---

# Capabilities — VPN-Rust

> **Production pivot (2026-07-13).** The capabilities below are the **implemented v0.1.0
> prototype**. Under the production direction, the transport, TUN, and auth capabilities are being
> **re-implemented** (not net-new): TLS-over-TCP → QUIC (M1), Linux-only TUN → cross-platform
> `TunDevice` (M2), self-signed/mTLS → pinned keypairs (M3), monitoring TUI → primary control
> dashboard (M4). See the Capability Matrix and Roadmap below, and
> `docs/planning/EXECUTION_PLAN.md`.

## Core Capabilities (prototype)

- **TUN interface management** (`net/tun.rs`) — create/configure Linux TUN (L3)
  devices (`rustvpn0` server, `rustvpn1` client), set IP/MTU, async read/write via
  `AsyncFd`, cleanup on drop.
- **TLS secure tunnel** (`net/tls.rs`) — rustls-based client (`connect_tls`) and
  server (`start_tls_server`) over TCP port 4433; webpki-roots or custom-CA trust.
- **mTLS client authentication** (`net/tls.rs`) — `ClientTlsConfig`/`ServerTlsConfig`,
  `AllowAnyAuthenticatedClient`, `get_client_cert_cn` for CN-based client identity.
- **Packet tunneling** — 2-byte big-endian length-prefixed framing of raw IP packets;
  length 0 = keepalive marker.
- **Traffic routing & NAT** (`net/route.rs`) — route setup and NAT / IP forwarding via
  system `ip`, `iptables`, and `sysctl` commands.
- **Multi-client management** (`net/clients.rs`) — `ClientManager` + `IpPool`
  (DHCP-like allocation), per-client `ClientStats`, `ClientConnection`, idle reaping.
- **Kill switch & leak prevention** (`net/security.rs`) — `SecurityManager` blocks
  non-VPN traffic on drop, plus DNS-leak and IPv6-leak prevention via iptables.
- **Keepalive & reconnect** — keepalive every 10s, 30s dead-peer timeout, exponential
  reconnect backoff 1s→30s.
- **CLI** (`cli.rs`, `main.rs`) — unified `vpn-rust` binary with `server`/`client`
  subcommands, `--config`, `--verbose` (clap 4.5).
- **TUI dashboard** (`tui/`) — ratatui/crossterm view of connection status, traffic
  stats, duration, and a log panel.
- **Configuration parsing** (`config/`) — OpenVPN `.ovpn` parsing (`OVPNConfig`) and
  TOML config (`Config`, `ClientConfig`, `ServerConfig`).

## Technical Capabilities

- **Primary language:** Rust (2021 edition).
- **Frameworks (prototype, being modernized inside the milestone that rewrites each subsystem —
  D-18):** Tokio 1.38 (async, `full`), rustls 0.21 / tokio-rustls 0.24 (TLS),
  tun 0.6 (TUN), ratatui 0.25 + crossterm 0.27 (TUI), clap 4.5 (CLI),
  serde/toml (config), anyhow (errors), log/env_logger (logging), x509-parser +
  rustls-pemfile + webpki-roots (certificates).
- **Target frameworks:** add `quinn` (QUIC, M1), rustls 0.23 + drop webpki-roots (M1),
  `tun-rs` replacing `tun` (M2), `rcgen` + `zeroize` (M3), ratatui 0.29 (M4), `tracing` +
  `tracing-subscriber` and `thiserror` (M0).
- **Codebase:** ~5,195 LOC of Rust across 40 tracked files (prototype).
- **Runtime target:** Linux only today (`AsyncFd`-based TUN); Linux/macOS/Windows clients target
  (M2).

## Capability Matrix

Status legend: **Implemented** (prototype, staying); **Implemented → superseded** (prototype code
replaced by a re-implementation under a milestone); **Planned (M#)** (target capability, not yet
built).

| ID | Capability | Status | Priority | Owner |
|----|-----------|--------|----------|-------|
| C-001 | TUN interface management (create/configure/read/write) | Implemented → re-implemented cross-platform (M2) | HIGH | Derek Martinez |
| C-002 | Secure tunnel transport | Implemented (TLS/TCP :4433) → superseded by QUIC (M1) | HIGH | Derek Martinez |
| C-003 | Packet tunneling / framing | Implemented (length-prefixed) → superseded by QUIC datagrams + control stream (M1) | HIGH | Derek Martinez |
| C-004 | Traffic routing & NAT / IP forwarding | Implemented → re-implemented behind `NetConfigurator` with rollback (M2) | HIGH | Derek Martinez |
| C-005 | Keepalive + reconnect (backoff) | Implemented → re-implemented over QUIC (M1) | HIGH | Derek Martinez |
| C-006 | Node authentication | Implemented (mTLS) → superseded by pinned keypairs / SPKI pinning (M3) | HIGH | Derek Martinez |
| C-007 | Peer session management | Implemented (multi-client `ClientManager`/`IpPool`) → re-scoped to single-peer P2P (M2) | MEDIUM | Derek Martinez |
| C-008 | Traffic statistics | Implemented (per-client) → single-peer stats feeding the TUI (M4) | MEDIUM | Derek Martinez |
| C-009 | Kill switch + DNS/IPv6 leak prevention | Implemented → re-implemented cross-platform via `NetConfigurator` (M2/M3) | HIGH | Derek Martinez |
| C-010 | CLI (server/client subcommands) | Implemented | MEDIUM | Derek Martinez |
| C-011 | TUI control dashboard (live graphs, peer/route panels, controls, theming) | Implemented (monitoring-only) → Planned primary dashboard (M4) | HIGH | Derek Martinez |
| C-012 | Config parsing (TOML; `.ovpn`) | Implemented → config-driven addressing + validation (M2/M3) | MEDIUM | Derek Martinez |
| C-013 | DNS resolution fully through the tunnel | Planned (M2/M3) | MEDIUM | Derek Martinez |
| C-014 | Non-root operation (capabilities/helper) | Deferred | LOW | Derek Martinez |
| C-015 | Cross-platform clients (Linux/macOS/Windows) via `TunDevice` | Planned (M2) | HIGH | Derek Martinez |
| C-016 | Performance optimization / benchmarking | Planned (M5) | LOW | Derek Martinez |
| C-017 | Root-free loopback integration-test harness + CI gates | Planned (M0) | HIGH | Derek Martinez |
| C-018 | QUIC transport (`quinn`; datagrams + control stream) | Planned (M1) | HIGH | Derek Martinez |
| C-019 | Trait seams (`Transport`, `TunDevice`, `NetConfigurator`) | Planned (M0/M1/M2) | HIGH | Derek Martinez |
| C-020 | Pinned-keypair auth (`rcgen`, SPKI fingerprint pinning, `vpn keygen`) | Planned (M3) | HIGH | Derek Martinez |
| C-021 | `tracing` structured logging; `thiserror` error seams | Planned (M0) | MEDIUM | Derek Martinez |
| C-022 | Release readiness (daemon/systemd, packaging, docs, SemVer) | Planned (M5) | MEDIUM | Derek Martinez |

## Integration Points

- **System networking commands** — `ip` (interface/address/route config), `iptables`
  (NAT, kill switch, DNS/IPv6 leak rules), `sysctl` (enable IP forwarding). Invoked as
  subprocesses; require root / `CAP_NET_ADMIN`.
- **`/dev/net/tun`** — Linux kernel TUN/TAP device node used to create virtual L3
  interfaces.
- **webpki-roots trust store** — baked-in root CA anchors for standard server
  certificate verification (prototype; **dropped** under pinned-keypair auth, D-13/M3).
- **Certificate / key files** — prototype PEM/X.509 certs and keys under `certs/`; target model is
  per-node keypairs generated with `rcgen` and pinned by SPKI fingerprint (M3).

## Capability Roadmap

**Prototype (v0.1.0, implemented):** C-001…C-012 across the original four phases — TUN + TLS/TCP
tunnel, routing/NAT, keepalive/reconnect, mTLS, multi-client, stats, kill switch/leak prevention,
CLI, monitoring TUI, config parsing. Several are re-implemented below.

**Production milestones (see `docs/planning/EXECUTION_PLAN.md`):**

- **M0 — Foundation & unblock:** C-017 (root-free loopback harness + CI gates), C-021 (`tracing` +
  `thiserror`), start of C-019 (trait seams). Establishes the Linux/WSL build path.
- **M1 — QUIC transport core:** C-018 (QUIC via `quinn`) replacing C-002/C-003; C-005 re-implemented
  over QUIC; `Transport` seam (C-019). Old TLS-over-TCP path deleted.
- **M2 — Cross-platform TUN + net config:** C-015 (cross-platform clients) and C-001 re-implemented
  via `TunDevice`; C-004/C-009 re-implemented behind `NetConfigurator`; C-007 re-scoped to
  single-peer P2P; C-013 (DNS-in-tunnel) design.
- **M3 — Security hardening:** C-020 (pinned-keypair auth, SPKI pinning, `vpn keygen`) replacing
  C-006; zeroize + config validation.
- **M4 — TUI control dashboard:** C-011 built out as the primary dashboard (ratatui 0.29) fed by
  C-008 stats.
- **M5 — Release readiness:** C-022 (daemon/systemd, packaging, docs, SemVer) and C-016
  (perf/benchmarking).

**Deferred:** C-014 (non-root operation).
