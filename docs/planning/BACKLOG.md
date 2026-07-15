---
title: "BACKLOG"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Backlog
author: Derek Martinez
---

# Backlog — VPN-Rust

> **Direction (2026-07-13):** VPN-Rust is being taken from a learning prototype to
> **production-ready personal software**: a **QUIC/UDP** tunnel between machines the
> operator owns — one **Linux server** plus **Linux / macOS / Windows** clients — joined by
> **pinned keypairs** (no CA/PKI), and driven from a **polished TUI control dashboard**.
> See [EXECUTION_PLAN.md](EXECUTION_PLAN.md) for the milestone sequence, the new-direction
> decisions D-10…D-18 in [../state/DECISIONS.md](../state/DECISIONS.md), and the target design
> in [../architecture/ARCHITECTURE.md](../architecture/ARCHITECTURE.md).

## Tagging guide

Single-solution repo — every item is tagged `[CORE]`. Items also carry a milestone tag
(`M0`–`M5`) that maps to the execution plan.

**Status values:** `Pending` → `In Progress` → `Shipped` | `Blocked`
When an item ships: `Shipped — <commit> — <date>`.

**Priority:** `HIGH` (blocks the milestone) · `MED` (needed for the milestone) · `LOW` (nice to have)

---

## M0 — Foundation & unblock

*Nothing else can be verified until the crate builds and CI has teeth.*

| ID | Tag | Title | Priority | Status |
|----|-----|-------|----------|--------|
| B-001 | [CORE] M0 | Establish a Linux/WSL build+test path (current Windows host cannot compile the crate) | HIGH | Shipped — 2026-07-13 (WSL Ubuntu, Rust 1.95: build 23s, 17 unit + 6 doc tests green, clippy `-D warnings` clean) |
| B-002 | [CORE] M0 | CI: multi-OS build matrix (ubuntu + windows + macos) | HIGH | In Progress — matrix authored in ci.yml (win/macos non-blocking until M2); pending push to verify |
| B-003 | [CORE] M0 | CI gates: `cargo clippy -D warnings`, `cargo fmt --check`, `cargo test` | HIGH | In Progress — `cargo fmt` applied, gates authored; verified locally; pending push |
| B-004 | [CORE] M0 | CI: `cargo-deny` / `cargo-audit` for advisories + license/dup checks | MED | In Progress — `cargo-audit` job authored; pending push to verify |
| B-005 | [CORE] M0 | Test harness: loopback integration tests that need no root (transport/crypto without a real TUN) | HIGH | Done (local) — `tests/quic_echo.rs` round-trips a QUIC datagram over loopback, no root; first integration test |
| B-006 | [CORE] M0 | Privileged/netns CI job for real-TUN end-to-end tests | MED | Pending |
| B-007 | [CORE] M0 | Migrate logging `log` + `env_logger` → `tracing` + `tracing-subscriber` | MED | Done (local) — verified on Linux (build/clippy `-D warnings`/tests green); pending commit |
| B-008 | [CORE] M0 | Introduce `thiserror` library error types at module boundaries; keep `anyhow` at the binary boundary | MED | In Progress — `ConfigError` pattern established on the stable `config` module; `net/` modules stay on anyhow until rewritten (D-18) |
| B-009 | [CORE] M0 | (Optional) Split into a Cargo workspace: `vpncore` lib + `vpn` bin (+ `vpn-tui`) | LOW | Pending |

## M1 — QUIC transport core

*Replace TLS-over-TCP with QUIC; this is the architectural heart.*

| ID | Tag | Title | Priority | Status |
|----|-----|-------|----------|--------|
| B-010 | [CORE] M1 | Define a `Transport` trait to decouple the engine from the wire implementation | HIGH | Done (local) — `transport::Transport` (send/recv datagram + max size) |
| B-011 | [CORE] M1 | Implement the `Transport` over `quinn` (QUIC/UDP; pulls modern rustls stack) | HIGH | Done (local) — `QuicTransport` wired into `engine` and the `vpn-rust` binary |
| B-012 | [CORE] M1 | Carry tunneled IP packets over **QUIC datagrams** (unreliable — no reliability-over-reliability) | HIGH | Done (local) — `engine::pump` moves TUN packets ↔ QUIC datagrams |
| B-013 | [CORE] M1 | **Control stream**: versioned handshake (protocol version, MTU/keepalive negotiation) | HIGH | Done (local) — `transport::control` (ClientHello/ServerHello, param negotiation); `tests/control_handshake.rs` |
| B-014 | [CORE] M1 | Port keepalive + reconnect (exp backoff) onto QUIC timers / 0-RTT resumption | MED | Done (local) — QUIC keep-alive + idle-timeout in `quic`; client reconnect w/ backoff in `engine` |
| B-015 | [CORE] M1 | Remove length-prefixed TLS-over-TCP protocol, echo path, and dead `tls.rs` code | HIGH | Done (local) — deleted `net/tls.rs`, `net/tun.rs`, `net/clients.rs`, old bins; dropped rustls 0.21 / tokio-rustls / webpki-roots / rustls-pemfile / x509-parser / tun 0.6 / winapi |
| B-016 | [CORE] M1 | Path MTU handling for QUIC-over-UDP (avoid fragmentation; clamp inner MTU) | MED | Pending |

## M2 — Cross-platform TUN + network config

*Fix the Windows/macOS build and make routing testable.*

| ID | Tag | Title | Priority | Status |
|----|-----|-------|----------|--------|
| B-017 | [CORE] M2 | `TunDevice` trait abstracting the platform TUN backend | HIGH | Done (local) — `net::device::{TunDevice, SystemTun}` |
| B-018 | [CORE] M2 | TUN backends via `tun-rs`: Linux, macOS (utun), Windows (wintun) | HIGH | In Progress — `tun-rs` backend compiles on Linux; Windows/macOS compile + runtime verification pending |
| B-019 | [CORE] M2 | **Verify Windows client actually compiles and runs** (closes the #1 blocker) | HIGH | Compiles — native `cargo build` on windows-msvc succeeds (was 4 errors at session start); runtime verification (wintun.dll + admin, real macOS host) still pending |
| B-020 | [CORE] M2 | `NetConfigurator` trait for address/route/NAT/DNS, with guaranteed rollback on drop/crash | HIGH | Done (local) — `net::netcfg::NetConfigurator`; rollback on drop; wired into `engine` (server NAT, client route) |
| B-021 | [CORE] M2 | Linux `NetConfigurator` via netlink (`rtnetlink`) or wrapped `ip`, behind the trait | MED | Done (local) — `LinuxNetConfigurator` wraps `ip`/`iptables`/`sysctl` (netlink refinement optional) |
| B-022 | [CORE] M2 | macOS + Windows route/DNS configurators | MED | In Progress — Noop configurator (warns to configure manually) on non-Linux; native impls pending |
| B-023 | [CORE] M2 | Config-driven addressing; collapse multi-client `ClientManager`/`IpPool` to a single-peer session | MED | Done (local) — CLI-driven addressing; single-peer engine; `clients.rs` removed |

## M3 — Security hardening (P2P)

*Pinned keys, no CA. Replace the broken cert story.*

| ID | Tag | Title | Priority | Status |
|----|-----|-------|----------|--------|
| B-024 | [CORE] M3 | `vpn keygen` subcommand — generate node keypair/cert via `rcgen` (replaces `gen_certs.sh`) | HIGH | Done (local) — `vpn-rust keygen` (+ `crypto::NodeIdentity`); verified via the binary |
| B-025 | [CORE] M3 | Custom rustls verifier: pin peer by **SPKI fingerprint**; drop webpki-roots/CA trust | HIGH | Done (local) — pins peer by exact certificate (single-entry root store); webpki-roots dropped; SHA-256 cert fingerprint identifies the pin |
| B-026 | [CORE] M3 | Fingerprint display + out-of-band verify (TOFU option) in CLI/TUI | MED | Done (local) — server + client + keygen log `sha256:…` fingerprints for out-of-band comparison |
| B-027 | [CORE] M3 | `zeroize` private key material; key-file permission checks (chmod 600 / ACL) | MED | Done (local) — key held in `Zeroizing<Vec<u8>>`; key file `0600` on Unix |
| B-028 | [CORE] M3 | Enforce no-payload logging in release builds; audit trace-level packet logging | MED | Pending |
| B-029 | [CORE] M3 | Config validation with actionable errors; secrets never logged | MED | Pending |

## M4 — TUI control dashboard

*The primary UX — make it a cockpit, not a readout.*

| ID | Tag | Title | Priority | Status |
|----|-----|-------|----------|--------|
| B-030 | [CORE] M4 | Upgrade `ratatui` 0.25 → 0.29 and `crossterm` 0.27 → 0.29 | HIGH | Done (local) — ratatui 0.29 + crossterm 0.28 (the version ratatui 0.29 re-exports) |
| B-031 | [CORE] M4 | Event-driven TUI architecture with a stats/event channel from the VPN engine | HIGH | Done (local) — `engine::stats::LiveStats` (Arc, atomics) written by the engine hot path + lifecycle; `tui::Dashboard` samples a `StatsSnapshot` each 150ms tick |
| B-032 | [CORE] M4 | Connection state-machine view (Disconnected → Handshaking → Connected → Reconnecting) | HIGH | Done (local) — `ConnectionState` set across server/client/connect paths; colored badge in the title bar |
| B-033 | [CORE] M4 | Live up/down **throughput sparklines** + RTT/latency gauge | HIGH | Done (local) — TX/RX `Sparkline`s from per-tick byte deltas; RTT `LineGauge` (0–200ms) sampled from `quinn::Connection::rtt()` |
| B-034 | [CORE] M4 | Peer panel, route table panel, byte/packet counters | MED | Done (local) — Connection + Session panels (peer, endpoint, negotiated MTU/keepalive, reconnect attempts, byte/packet counters) |
| B-035 | [CORE] M4 | Scrolling, filterable log viewer (fed by `tracing`) | MED | Done (local) — `tui::logbuf::{LogBuffer, LogLayer}` `tracing` layer → bounded ring; scroll + level filter in the log panel |
| B-036 | [CORE] M4 | Controls: connect / disconnect / reconnect / quit with keybindings + help overlay | HIGH | Done (local) — keybindings (q/esc, f filter, ↑/↓/PgUp/PgDn scroll, g/Home, c clear, ?/h help) + centered help overlay |
| B-037 | [CORE] M4 | Theming (light/dark + accent), consistent layout system, responsive to terminal size | MED | Done (local) — dark theme + cyan accent, `Layout`-driven responsive panels, clamped; headless `TestBackend` render tests |

## M5 — Release readiness

| ID | Tag | Title | Priority | Status |
|----|-----|-------|----------|--------|
| B-038 | [CORE] M5 | Metrics surface (counters/histograms) feeding TUI and optional export | LOW | Partial — live counters/RTT surfaced via `LiveStats` to the TUI; a standalone export (e.g. Prometheus) is still pending |
| B-039 | [CORE] M5 | Headless `--daemon` mode + systemd unit (Linux server) | MED | Done (local) — `--daemon` flag (headless, ANSI-off logging for journald; conflicts with `--tui`); `packaging/systemd/vpn-rust-server.service` (Type=simple, CAP_NET_ADMIN, hardening). Note: true double-fork detach deferred — service managers supervise |
| B-040 | [CORE] M5 | Client packaging: release binaries / installers per OS | MED | Done (local) — `.github/workflows/release.yml` (tag-triggered matrix build → per-OS archives → GitHub Release); `packaging/README.md` per-OS install notes |
| B-041 | [CORE] M5 | Docs: quickstart, threat model, versioned wire-protocol spec | MED | Done (local) — `docs/QUICKSTART.md`, `docs/operations/THREAT_MODEL.md`, `docs/standards/WIRE_PROTOCOL.md` |
| B-042 | [CORE] M5 | SemVer discipline; matched-build guarantee until protocol stabilizes | LOW | Partial — documented (versioning section in WIRE_PROTOCOL.md); not yet enforced in tooling |

---

## Backlog (unscheduled / ideas)

| ID | Tag | Title | Priority | Status |
|----|-----|-------|----------|--------|
| B-100 | [CORE] | Optional traffic obfuscation / pluggable transport | LOW | Pending |
| B-101 | [CORE] | Compression for suitable traffic | LOW | Pending |
| B-102 | [CORE] | IPv6 tunneling inside the VPN | LOW | Pending |
| B-103 | [CORE] | Full-tunnel default-route mode with kill switch parity across OSes | LOW | Pending |
| B-104 | [CORE] | Mobile (iOS/Android) client exploration | LOW | Pending |

---

## Shipped (prototype foundation, pre-pivot)

| ID | Title | Status |
|----|-------|--------|
| B-200 | Async TUN interface (Linux) + IP configuration | Shipped — pre-2026 |
| B-201 | TLS tunnel over TCP (rustls) with length-prefixed framing | Shipped — pre-2026 *(superseded by M1)* |
| B-202 | Bidirectional forwarding, routing + NAT (Linux) | Shipped — pre-2026 |
| B-203 | Keepalive + reconnect with exponential backoff | Shipped — pre-2026 |
| B-204 | clap CLI (`server`/`client`) + TOML/`.ovpn` config parsing | Shipped — pre-2026 |
| B-205 | ratatui TUI (status/stats/logs) | Shipped — pre-2026 *(reworked in M4)* |
| B-206 | mTLS scaffolding + multi-client manager + IP pool | Shipped — pre-2026 *(re-scoped in M2/M3)* |
| B-207 | GitHub Actions CI (build + test) | Shipped — pre-2026 *(hardened in M0)* |
| B-208 | BEACON Framework onboarding + full docs population | Shipped — 2026-07-13 |
