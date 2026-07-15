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

**Last updated:** 2026-07-15 — M0–M4 complete + M5 docs/packaging done; the core QUIC VPN now has a
live TUI cockpit. Remaining M5 (metrics export, SemVer tooling) is LOW; on-target runtime
verification still pending.

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
- **M2 — cross-platform TUN + net config (done):** `net::device::{TunDevice, SystemTun}` via
  `tun-rs` (Linux/macOS/Windows) — **the crate compiles natively on Windows** (was 4 errors at
  session start); `net::netcfg::NetConfigurator` (Linux NAT/route with rollback on drop; warn-noop
  on other OSes) wired into the engine. `engine::{run_server,run_client}` pump packets between the
  TUN and QUIC datagrams; single-peer P2P; multi-client scaffolding removed.
- **M3 — security (done):** `crypto::NodeIdentity` (self-signed, load-or-generate, `Zeroizing`
  key, `0600` perms); `vpn-rust keygen` subcommand; QUIC client **pins the peer certificate**;
  SHA-256 fingerprints logged for out-of-band (TOFU) verification. Smoke-tested via the real binary.
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

- None — M0–M4 are complete plus the substantive M5 items (docs + packaging). Changes are **local
  and uncommitted** (see `git status`); awaiting the operator's go-ahead to commit.

## Recently done (2026-07-15)

- **M4 — TUI control dashboard (complete):** ratatui 0.25→0.29 / crossterm 0.28; new
  `engine::stats::LiveStats` telemetry handle written by the engine (counters, state, RTT, peer,
  negotiated params) and sampled by `tui::Dashboard` each tick; event-driven cockpit (colored state
  badge, Connection/Session panels, TX/RX sparklines, RTT gauge, byte/packet counters, filterable
  scrolling log viewer fed by a `tracing` layer, keybindings + help overlay, dark/cyan theme);
  `--tui` wiring (engine on a task, dashboard foreground) and `--daemon` headless flag; headless
  `TestBackend` render tests.
- **M5 — docs (B-041):** `QUICKSTART.md`, `THREAT_MODEL.md`, `WIRE_PROTOCOL.md`.
- **M5 — packaging (B-039/B-040):** `release.yml` matrix release workflow + `packaging/` (systemd
  unit, install docs).
- **Fix:** boxed the large `toml::de::Error` in `ConfigError` for `clippy -D warnings` (a stricter
  clippy started flagging `result_large_err`).

## Next

- **M5 leftovers (LOW):** standalone metrics export (B-038; live metrics already reach the TUI);
  SemVer enforcement tooling (B-042; already documented).
- **Remaining small items:** inner-MTU/PMTU clamp refinement (B-016); config validation (B-029);
  native macOS/Windows `NetConfigurator` (B-022); true `--daemon` double-fork detach (deferred —
  service managers supervise).
- **On-target verification (the big gap):** run the tunnel end-to-end with root on Linux (or netns);
  drive the `--tui` dashboard against a live session; validate the Windows (wintun) / macOS (utun)
  clients on real hosts.

## Known Issues / Limitations

- **Runtime verification is partial:** the crate builds on Linux **and Windows**, and the binary
  runs up to TUN creation (root-gated). The full packet-forwarding path is not yet exercised
  end-to-end (needs root; the dev WSL has no passwordless sudo), and the Windows/macOS clients have
  not been run on real hosts (need wintun.dll / a macOS box).
- **Routing/NAT is Linux-only:** the `NetConfigurator` wires server NAT + client route on Linux
  (rollback on drop); macOS/Windows are a warn-noop (B-022).
- **Security model:** pinning is by exact certificate (single-entry root store), identified by
  SHA-256 fingerprint (TOFU); private key is zeroized and stored `0600`. SPKI-fingerprint pinning is
  an optional future refinement (B-025).
- **TUI runtime unverified against a live tunnel:** the dashboard renders correctly under
  `TestBackend` and the crate builds, but it has not been driven against a real `--tui` session
  (needs root for the TUN + an interactive terminal).
- **`--daemon` does not detach:** it selects headless, ANSI-off logging suited to a systemd
  `Type=simple` service; true double-fork daemonization is deferred.
- **Privileges:** TUN creation requires root / CAP_NET_ADMIN.
