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

**Last updated:** 2026-07-14 — QUIC engine landed; the crate now builds on Windows.

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

- **M0 — Foundation (done):** established a Linux/WSL build+test path (WSL Ubuntu, Rust 1.95);
  hardened CI (fmt + `clippy -D warnings` + `cargo audit` + cross-platform build matrix); migrated
  `log`/`env_logger` → `tracing`; introduced `thiserror` at the `config` boundary
  (`config::ConfigError`).
- **M1 — QUIC transport core (done):** `transport::Transport` seam + `quinn` implementation
  (`QuicTransport`) carrying tunneled IP packets as **QUIC datagrams**; versioned control-stream
  handshake with parameter negotiation (`transport::control`); QUIC keep-alive + idle-timeout;
  client reconnect with exponential backoff; **legacy TLS-over-TCP path removed** (`net/tls.rs`,
  `net/tun.rs`, `net/clients.rs`, old bins deleted; rustls 0.21 / tokio-rustls / webpki-roots /
  rustls-pemfile / x509-parser / tun 0.6 / winapi dropped).
- **M2 — cross-platform TUN (build unblocked):** `net::device::{TunDevice, SystemTun}` via
  `tun-rs` (Linux/macOS/Windows); **the crate now compiles natively on Windows** (was 4 errors at
  session start). `engine::{run_server,run_client}` pump packets between the TUN and QUIC
  datagrams; single-peer P2P; multi-client scaffolding removed.
- **M3 — security (partial):** `crypto::NodeIdentity` load-or-generate self-signed identity (DER;
  `0600` key perms on Unix); QUIC client **pins the peer certificate**. Smoke-tested: the real
  binary generates its identity and binds the QUIC endpoint (TUN step correctly root-gated).
- **Verification:** 17 unit + 2 integration (loopback QUIC echo + control handshake) + 2 doc tests
  green on Linux; `clippy -D warnings` and `fmt` clean; native Windows `cargo build` succeeds.
- **Production pivot committed (earlier this session):** committed to the production direction and rewrote
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

- Finishing **M2** (`NetConfigurator` route/NAT/DNS abstraction) and **M3** (security hardening
  refinements).

## Next

- **M2 remainder:** `NetConfigurator` trait + per-OS route/NAT/DNS with rollback (B-020–022),
  wrapping the Linux `route`/`security` modules; inner-MTU/PMTU clamp (B-016).
- **M3 remainder:** `keygen` CLI subcommand; SPKI-fingerprint pinning + fingerprint display (TOFU);
  `zeroize` private keys; config validation.
- **M4 — TUI control dashboard:** upgrade ratatui 0.25→0.29 and rebuild the TUI as an event-driven
  control cockpit (connect/disconnect, live throughput graphs, peer/route panels, log viewer,
  theming), fed by an engine stats/event channel.
- **M5 — Release readiness:** metrics, `--daemon` + systemd unit, per-OS packaging, docs, SemVer.
- **On-target verification:** run the tunnel end-to-end with root on Linux and validate the Windows
  (wintun) / macOS (utun) clients on real hosts.

## Known Issues / Limitations

- **Runtime verification is partial:** the crate builds on Linux **and Windows**, and the binary
  runs up to TUN creation (root-gated). The full packet-forwarding path is not yet exercised
  end-to-end (needs root; the dev WSL has no passwordless sudo), and the Windows/macOS clients have
  not been run on real hosts (need wintun.dll / a macOS box).
- **No routing/NAT wired yet:** the engine tunnels packets between the two TUN devices but does not
  yet configure host routes or server NAT — the `NetConfigurator` abstraction (M2, B-020–022) is
  pending; the Linux `route`/`security` modules exist but are not wired in.
- **Security refinements pending (M3):** pinning is by exact certificate (single-entry root store),
  not yet SPKI-fingerprint with display/TOFU; private keys are not yet zeroized.
- **TUI not yet reworked:** the prototype ratatui 0.25 dashboard is unused by the new engine and
  awaits the M4 rewrite.
- **Privileges:** TUN creation requires root / CAP_NET_ADMIN.
