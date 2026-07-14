---
title: "STATUS"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Status
author: Derek Martinez
---

# Status — VPN-Rust

**Last updated:** 2026-07-13 — production-direction pivot committed.

## Project Snapshot

- **Direction (decided 2026-07-13):** moving from a learning prototype to **production-ready
  personal software** — a **QUIC/UDP point-to-point VPN** between machines the operator owns (one
  Linux server + Linux/macOS/Windows clients), authenticated by **pinned keypairs** (SPKI
  fingerprint pinning, no CA/PKI), driven from a **TUI control dashboard** as the primary UX. See
  `docs/planning/EXECUTION_PLAN.md`, `docs/state/DECISIONS.md` (D-10…D-18), and
  `docs/architecture/ARCHITECTURE.md`.
- **Language:** Rust (2021 edition), ~5,195 LOC across 40 tracked files (prototype)
- **Build system:** Cargo
- **Current runtime / key crates (prototype):** Tokio (async), rustls 0.21 / tokio-rustls 0.24
  (TLS), tun 0.6 (TUN L3), clap 4.5 (CLI), ratatui 0.25 (TUI), anyhow (errors), log + env_logger.
  These are being modernized inside the milestone that rewrites each subsystem (D-18).
- **Current architecture (prototype, being superseded):** client-server TLS tunnel over TCP :4433,
  L3 TUN interfaces (server rustvpn0 10.8.0.1/30, client rustvpn1 10.8.0.2/30), length-prefixed
  wire protocol (2-byte BE u16 + IP packet; length=0 = keepalive)
- **Target architecture:** QUIC transport (`quinn`; datagrams for packets + reliable control
  stream), cross-platform TUN via a `TunDevice` trait (`tun-rs`), single-peer P2P session.
- **Supported target today:** Linux only (TUN requires root / CAP_NET_ADMIN); cross-platform
  clients arrive in M2.
- **Framework onboarded:** 2026-07-13 (BEACON Framework)

## Done

- **Production pivot committed (this session):** committed to the production direction and rewrote
  the core planning docs to the QUIC P2P direction — `docs/planning/BACKLOG.md`,
  `docs/planning/EXECUTION_PLAN.md`, `docs/state/DECISIONS.md` (D-10…D-18), and
  `docs/architecture/ARCHITECTURE.md`. The derivative docs (STATUS, HANDOFF, RISKS,
  OPEN_QUESTIONS, PRD, CAPABILITIES) are aligned to it.
- **BEACON onboarding:** onboarded the project to the BEACON Framework and populated all project-content documentation (STATUS, HANDOFF, CHANGELOG, DECISIONS, OPEN_QUESTIONS, RISKS) with real architecture and current state.
- **Phase 1 — Core infrastructure:** async TUN interface (create/configure/read/write, cleanup on Drop, MTU 1500), rustls TLS tunnel (client connect + server acceptor), length-prefixed packet protocol, basic echo tunnel, GitHub Actions CI, doc comments across public API, anyhow error handling with context, structured logging, consolidated constants module.
- **Phase 2 — VPN functionality:** bidirectional packet forwarding (TUN <-> TLS concurrently), route management + NAT + IP forwarding (`src/net/route.rs`), connection resilience — application-level keepalive every 10s, 30s inactivity timeout, automatic reconnect with exponential backoff (1s -> 30s max).
- **Phase 3 — CLI & usability:** unified `vpn-rust` binary with clap `server`/`client` subcommands (`--config`, `--verbose`), TOML configuration (`src/config/toml_config.rs`, `config.example.toml`), ratatui TUI dashboard (connection status, traffic stats, duration, log viewer).
- **Phase 4 — Production features:** mTLS client-certificate authentication with server-side validation (`src/net/tls.rs`), multi-client support with `ClientManager` + DHCP-like `IpPool` and per-client stats (`src/net/clients.rs`), kill switch + DNS/IPv6 leak prevention via iptables (`src/net/security.rs`).

## In Flight

- **Production pivot planning complete; entering M0 (Foundation & unblock).** The direction and
  milestone plan (M0–M5) are decided and documented; execution begins at M0.

## Next

**M0 — Foundation & unblock (NEXT):**

1. **Establish a Linux/WSL build+test path** — the crate cannot compile on this Windows host, so
   the first task is a place where `cargo build`/`test` actually run.
2. **CI quality gates** — multi-OS CI matrix with clippy / fmt --check / `cargo audit` gates.
3. **Root-free loopback integration-test harness** — so networking code can be exercised without
   root / CAP_NET_ADMIN.
4. **Migrate `log` → `tracing`** (D-14) and **introduce `thiserror` error seams** (D-15).

**Then M1 — QUIC transport core:** introduce the `Transport` trait and a `quinn` implementation
(IP packets over QUIC datagrams + a versioned reliable control stream); the old TLS-over-TCP path
(`tls.rs`) is removed here.

Later milestones: M2 cross-platform TUN + `NetConfigurator`; M3 pinned-key security hardening; M4
TUI control dashboard (ratatui 0.29); M5 release readiness.

## Known Issues

- **Build FAILS on Windows** (the current dev host): `src/net/tun.rs` depends on Linux-only APIs;
  `cargo test` cannot run here. This is the #1 blocker — M0 establishes a Linux/WSL path and M2
  delivers the cross-platform `TunDevice` fix.
- **Minimal test coverage:** a handful of unit tests (config parsing) + doc tests; no integration
  tests. M0 adds the loopback harness.
- **Security posture (prototype):** self-signed certs only, no certificate revocation, trust model
  assumes trusted endpoints; not security-audited. Superseded by pinned keypairs in M3.
- **Privileges:** TUN interface creation requires root / CAP_NET_ADMIN.
