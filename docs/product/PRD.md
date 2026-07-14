---
title: "PRD"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Prd
author: Derek Martinez
---

# Product Requirements — VPN-Rust

## Overview

VPN-Rust is a **personal point-to-point VPN** written in Rust (2021 edition). It began as a
learning-focused prototype (`vpn-rust` v0.1.0) demonstrating systems-level networking — TUN
interfaces, TLS tunneling, async I/O, routing/NAT — and, as of the **2026-07-13 production pivot**,
is being productionized into secure personal software: a **QUIC/UDP tunnel** between machines the
operator owns (one **Linux server** plus **Linux/macOS/Windows clients**), authenticated by
**pinned keypairs** (SPKI fingerprint pinning, no CA/PKI), and driven from a **polished TUI control
dashboard** as the primary UX. The v0.1.0 prototype exists and works on Linux; the production
milestones M0–M5 (see `docs/planning/EXECUTION_PLAN.md`) are ahead.

### Vision

A trustworthy, easy-to-run VPN for one person's own devices: connect any of your Linux, macOS, or
Windows machines to your Linux server over a modern QUIC transport, with mutual authentication by
pinned keys and no PKI to operate, all controlled and observed from a single TUI cockpit. The
readable, well-documented codebase remains — the learning origin becomes a maintainable production
implementation rather than a demo.

### Problem Statement

The operator wants a secure tunnel between the machines they personally own, without the
operational weight of production VPNs (OpenVPN, WireGuard PKI/config sprawl) or a multi-tenant
service. The learning prototype proved the internals but has three production blockers: it runs
TLS-over-TCP (TCP-meltdown under loss), it is Linux-only (does not build on Windows/macOS), and its
self-signed/mTLS trust model is not fit for real use. The product closes these gaps for a
single-operator, point-to-point use case.

### Goals

1. **Secure by construction** — QUIC/TLS 1.3 transport, mutual authentication via pinned keypairs,
   zeroized secrets, no payload logging.
2. **Cross-platform clients** — Linux, macOS, and Windows clients connect to the Linux server.
3. **Operable** — the TUI control dashboard is the primary way to run and observe the VPN; clean
   install and teardown.
4. **Modern & maintainable** — current Rust patterns (Tokio, `quinn`, rustls, `tracing`,
   `thiserror`, ratatui) behind trait seams (`Transport`, `TunDevice`, `NetConfigurator`) so
   platforms and wire formats stay swappable and testable.

## Technical Context

- **Primary language:** Rust (2021 edition); prototype is ~5,195 LOC across 40 tracked files.
- **Build system:** Cargo (`Cargo.toml` / `Cargo.lock`).
- **Runtime target — today:** Linux only (prototype TUN via `AsyncFd`); **does not build on
  Windows/macOS**.
- **Runtime target — production:** Linux server; Linux/macOS/Windows clients (M2 delivers the
  cross-platform `TunDevice`).
- **Transport — today:** TLS-over-TCP :4433 (prototype). **Production:** QUIC/UDP via `quinn`
  (M1).
- **Auth — today:** self-signed certs + opt-in mTLS (prototype). **Production:** pinned keypairs
  via `rcgen`, SPKI-fingerprint pinning, no CA (M3).
- **Solution scope:** single crate `vpn-rust` today — library plus unified `vpn-rust` binary
  (`server`/`client` subcommands) and standalone `server`/`client` binaries. A Cargo workspace
  split is under consideration (OQ-12 / backlog B-009).

## User Personas

| Persona | Description | Needs |
|---------|-------------|-------|
| The operator (primary) | The person running a personal VPN across their own Linux/macOS/Windows devices, connecting to a Linux server they host. | Secure, reliable point-to-point tunnel; a clear TUI to connect/observe/disconnect; simple key management (pin a fingerprint, no PKI); clean cross-platform install and teardown. |
| Developer (self) | The author, maintaining and extending the codebase. | Readable, well-documented code; trait seams and tests that make change safe. |
| Other learners | Developers referencing the codebase to learn modern async network programming. | Readable code, explanatory comments, architecture docs. |

## Requirements

### User Stories — prototype phases (v0.1.0, implemented)

The phases below describe the implemented learning prototype. Several capabilities are being
**re-implemented** (not net-new) under the production milestones that follow.

**Phase 1 — Core infrastructure**
- Create TUN interfaces programmatically to capture/inject packets.
- Establish a TLS-encrypted connection between client and server.
- Verify the tunnel with a basic packet path and clear logging.

**Phase 2 — VPN functionality**
- Route real IP traffic through the tunnel (bidirectional forwarding).
- Server performs NAT / IP forwarding to the internet.
- Survive brief network interruptions via keepalive + automatic reconnect.

**Phase 3 — Usability**
- Control the VPN through a CLI (`vpn-rust server|client`, `--config`, `--verbose`).
- Configure via file (TOML; OpenVPN `.ovpn` parsing also supported).
- Monitor status, traffic, duration, and logs through a TUI dashboard.

**Phase 4 — Production-leaning features**
- Authenticate clients with mutual TLS (client certificates).
- Support multiple clients with automatic IP assignment.
- Track per-client traffic statistics.
- Kill switch + DNS/IPv6 leak prevention to stop leaks on disconnect.

### User Stories — production milestones (M0–M5, planned)

**M0 — Foundation & unblock**
- As the developer, I can build and test the crate on a Linux/WSL path and see CI catch
  regressions (clippy / fmt / audit gates) so work is compile-verified.
- As the developer, I can run networking integration tests over loopback without root.
- Structured `tracing` logs and `thiserror` error seams replace `log`/`env_logger` and blanket
  `anyhow`.

**M1 — QUIC transport core**
- As the operator, my tunnel runs over QUIC/UDP (IP packets over datagrams, versioned control
  stream) instead of TLS-over-TCP, so it survives packet loss without TCP-meltdown.

**M2 — Cross-platform TUN + network config**
- As the operator, I can run a **macOS or Windows client** and connect to my Linux server.
- Routes/NAT are configured and cleanly torn down on exit and on crash.

**M3 — Security hardening (P2P)**
- As the operator, I generate a keypair (`vpn keygen`) and pin my peer by SPKI fingerprint; a
  connection is rejected on fingerprint mismatch, and private keys are zeroized and never logged.

**M4 — TUI control dashboard**
- As the operator, I can fully run the VPN from the TUI: connect, watch live throughput/RTT and
  peer/route panels, filter logs, and disconnect.

**M5 — Release readiness**
- As the operator, I can install the server (daemon/systemd) and a client from packaged artifacts
  and docs alone and establish a tunnel.

## Success Criteria

Status reflects the implementation as of the current codebase (source: `docs/TASKS.md`,
dated 2024-12-08). "Deferred" items are intentionally not implemented yet.

### Phase 1 — Core infrastructure
- [x] TUN interface creation on Linux.
- [x] TLS tunnel established between client and server.
- [x] Packet path verified through the tunnel.
- [x] Logging shows packet/connection flow (`log` + `env_logger`).

### Phase 2 — VPN functionality
- [x] Real IP traffic routes through the tunnel (bidirectional forwarding).
- [x] Server performs NAT / IP forwarding to the internet (ip/iptables/sysctl).
- [x] Connection survives interruptions (keepalive 10s + reconnect with exponential
      backoff 1s→30s, 30s connection timeout).
- [ ] DNS resolution fully through the tunnel — **Deferred** (DNS-in-tunnel not done).

### Phase 3 — Usability
- [x] CLI with `server`/`client` subcommands, `--config`, `--verbose`.
- [x] TOML configuration file support (plus `.ovpn` parsing).
- [x] TUI dashboard (connection status, traffic stats, duration, log panel).
- [ ] Non-root operation via capabilities/helper — **Deferred** (TUN still needs
      root / `CAP_NET_ADMIN`).

### Phase 4 — Production-leaning features
- [x] Mutual TLS authentication (client certificates; CN-based identity). *(Superseded by pinned
      keypairs, M3.)*
- [x] Multi-client management (`ClientManager` + `IpPool` DHCP-like allocation). *(Re-scoped to a
      single-peer P2P session, M2.)*
- [x] Per-client traffic statistics.
- [x] Kill switch + DNS/IPv6 leak prevention (iptables). *(Re-implemented cross-platform via
      `NetConfigurator`, M2.)*
- [ ] Cross-platform support (macOS/Windows) — **Planned (M2)** via the `TunDevice` abstraction.
- [ ] Performance optimization — deferred to M5.

### Production milestones (M0–M5)
Exit criteria are defined per milestone in `docs/planning/EXECUTION_PLAN.md`; summary:
- [ ] **M0** — `build`/`test`/`clippy`/`fmt --check` green in CI (Linux, plus Windows/macOS legs
      building what compiles), and a root-free loopback integration test runs.
- [ ] **M1** — two nodes complete a QUIC handshake and pass IP packets end-to-end over loopback;
      the TLS-over-TCP path is deleted; protocol version negotiated on connect.
- [ ] **M2** — Windows and macOS clients build and establish a tunnel to the Linux server;
      routes/NAT set and cleanly torn down on exit and crash.
- [ ] **M3** — a connection is rejected on SPKI-fingerprint mismatch; private keys zeroized and
      never logged; misconfig produces actionable errors.
- [ ] **M4** — the VPN can be fully operated from the TUI (connect → live stats → disconnect) with
      a coherent, themed, responsive layout.
- [ ] **M5** — a clean-machine operator can install the server + a client from artifacts and docs
      alone and establish a tunnel.

## Non-Functional Requirements

### Performance (targets, not measured)
The following are **design targets**, not benchmarked results:
- Throughput target > 100 Mbps on a local network.
- Per-packet TLS encrypt/decrypt and framing overhead in the low-millisecond range.
- Memory target < 50 MB (client), < 100 MB (server with multiple clients).

### Security
- **Target:** TLS 1.3 over QUIC (rustls under `quinn`) for all tunnel traffic.
- **Target:** mutual authentication via **pinned keypairs** — each node generates a keypair with
  `rcgen`; peers pin each other by **SPKI fingerprint** (no CA/PKI). Connections are rejected on
  fingerprint mismatch (M3).
- **Target:** secrets zeroized (`zeroize`), key-file permission checks, config validation, and no
  payload logging in release builds.
- Kill switch and DNS/IPv6 leak prevention, re-implemented cross-platform via `NetConfigurator`.
- **Current (prototype) limits, superseded by M3:** self-signed certificates + opt-in mTLS only,
  no revocation/CRL, trust model assumes trusted endpoints, not production-audited.

### Reliability
- Graceful handling of network interruptions (keepalive + backoff reconnect).
- Interface cleanup on shutdown.
- Dead-peer detection via keepalive timeout.

### Portability
- **Server:** Linux. **Clients:** Linux, macOS, and Windows (target). At present the crate is
  Linux-only and does not build on Windows/macOS (`AsyncFd`-based TUN); M2 delivers cross-platform
  clients via the `TunDevice` trait backed by `tun-rs` (Linux/utun/wintun).

### Maintainability
- Modular architecture (config / net / tui / cli separation).
- Doc comments on public items; doc tests.

## Constraints

- **Scope:** **personal point-to-point** only (D-10) — one operator-hosted Linux server plus
  clients the operator owns. No multi-user accounts, no PKI service, no multi-tenant server.
  Single developer.
- **Technical:** TUN requires root / `CAP_NET_ADMIN` (Linux) or equivalent elevation; the tunnel
  is a single QUIC session between two peers; point-to-point `/30` (or `/31`) addressing.
- **Security:** target model is pinned keypairs with out-of-band fingerprint verification (TOFU);
  no protection against an attacker who can substitute a pinned key. Not formally audited.
- **Current-state constraints (being lifted):** the crate does not build on Windows/macOS (M2);
  transport is still TLS-over-TCP until M1; auth is still self-signed/mTLS until M3.
- **Testing:** minimal today — a few unit tests (config parsing, IP pool) plus doc tests; **no
  integration tests**. M0 adds a root-free loopback harness and CI gates.

## Risks and Mitigations

See `docs/state/RISKS.md` for the tracked risk register (R-1…R-11). Summary:

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Windows/macOS build blocked | High | High | `TunDevice` trait + `tun-rs` cross-platform backends (M2); Linux/WSL build path first (M0) |
| Minimal test coverage (no integration tests) | High | High | Root-free loopback harness + CI gates (M0); trait seams make paths mockable |
| Prototype self-signed/mTLS trust not production-grade | High | Medium | Superseded by pinned keypairs / SPKI-fingerprint pinning (M3) |
| QUIC transport rewrite scope | Medium | Medium | Land the `Transport` seam first; validate incrementally over loopback (M1) |
| Dependency major-version upgrades | Medium | Medium | Fold each upgrade into the milestone that rewrites its subsystem (D-18) |
| Unmeasured performance vs. targets | Low | Medium | Label numbers as targets; profile before optimizing (M5) |
| Scope creep | Low | High | Milestone approach; scope fixed to personal P2P (D-10) |

## Out of Scope

- **Multi-user / multi-tenant service, enrollment, and CA/PKI** — the product is single-operator
  point-to-point (D-10).
- **A formal third-party security audit** — the target hardening (pinned keys, zeroize, config
  validation) is engineering hardening for personal use, not a certified audit.
- Compression, traffic obfuscation, and WireGuard-protocol compatibility (future considerations
  only).
- DNS-fully-in-tunnel and split tunneling remain open design questions (OQ-4, OQ-7), scheduled
  around M2/M3 rather than committed scope.

Note: items previously listed here as out of scope are now **in scope** under the production
direction — QUIC/UDP transport (M1), macOS/Windows clients (M2), and a hardened trust model via
pinned keypairs (M3).
