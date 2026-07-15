---
title: "CHANGELOG"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Changelog
author: Derek Martinez
---

# Changelog

## 2026-07-15

- **M4 — TUI control dashboard (complete, local):** rebuilt the terminal UI as a live, event-driven
  cockpit on **ratatui 0.29 / crossterm 0.28** (B-030).
  - **Engine telemetry (B-031):** new `engine::stats::LiveStats` — an `Arc`-shared, mostly-atomic
    handle the engine writes on the hot path (cumulative byte/packet counters) and lifecycle
    transitions (state, peer, negotiated params, endpoint, RTT, reconnect attempts). `engine::pump`
    now records per-datagram counts and samples `quinn::Connection::rtt()` every second; the
    server/client/connect paths drive the `ConnectionState` machine. Signatures changed to
    `run_server(params, stats)` / `run_client(params, stats)`.
  - **Dashboard:** `tui::Dashboard` (rendering-free state) samples a `StatsSnapshot` each 150ms
    tick and derives TX/RX throughput history by differencing counters; `tui::ui::render` draws a
    title bar with a colored state badge, Connection + Session panels, TX/RX `Sparkline`s, an RTT
    `LineGauge`, and a scrolling/filterable log panel; `tui::run_dashboard` owns an RAII terminal
    guard + event loop (B-032–B-037).
  - **Log capture (B-035):** `tui::logbuf::{LogBuffer, LogLayer}` — a `tracing` layer feeding a
    bounded ring the dashboard renders (so logs don't corrupt the alternate screen).
  - **CLI wiring:** `--tui` (run engine on a task + dashboard in the foreground; quitting aborts the
    engine) and `--daemon` (headless, ANSI-off logging for journald; conflicts with `--tui`). Added
    to `server`/`client`.
  - Headless `TestBackend` render tests (no TTY needed).
- **M5 — release readiness (substantive items, local):**
  - **Docs (B-041):** `docs/QUICKSTART.md`, `docs/operations/THREAT_MODEL.md`,
    `docs/standards/WIRE_PROTOCOL.md` (versioned, `PROTOCOL_VERSION = 1`).
  - **Packaging (B-039/B-040):** `.github/workflows/release.yml` (tag-triggered matrix build → per-OS
    archives → GitHub Release); `packaging/systemd/vpn-rust-server.service` + `packaging/` install docs.
- **Fix:** boxed the large `toml::de::Error` in `config::ConfigError` (clippy `result_large_err`,
  newly enforced by a stricter clippy) so `clippy -D warnings` is clean again.
- **Hardening batch ("Group A", built via three parallel subagents on disjoint modules):**
  - **B-029 config validation:** `Config::validate` + `ConfigError::Invalid { field, value, reason }`
    (IPv4 / CIDR / port / non-empty checks, fail-fast). Wired `--config` to load+validate (bails
    with the actionable error) and overlay the file's addressing/paths onto the server/client params
    — closing the long-standing "config reserved but unwired" gap.
  - **B-016 PMTU guard:** `pump` now guards outbound packets against `max_datagram_size()`
    (`exceeds_datagram_limit` helper; rate-limited clean drop; startup log of the datagram ceiling
    vs inner MTU). IP fragmentation / ICMP PMTUD explicitly out of scope.
  - **B-028 no-payload logging:** audited engine/transport/device (none present) and documented the
    policy in the engine module (payload bytes only under `#[cfg(debug_assertions)]` at `trace!`).
  - **B-038 metrics:** `metrics::render_prometheus` + a dependency-free `serve` HTTP endpoint behind
    `--metrics-addr` (off by default; bind loopback on a VPN host).
  - **B-042 versioning:** `docs/standards/VERSIONING.md` (SemVer + `PROTOCOL_VERSION` matched-build
    policy).
- **Deferred:** B-009 workspace split (optional; structurally rewrites every path — better standalone).
- **Tests:** 50 unit + 2 integration + 2 doc (54 total) green; `clippy -D warnings` + `fmt` clean;
  native Windows `cargo build --release` succeeds; CLI smoke tests confirm `--tui`/`--daemon`
  parse + conflict, `--metrics-addr` is present, and a bad `--config` fails fast with an actionable
  error. **M0–M4 complete; M5 substantially complete (only optional tooling for B-042 enforcement
  remains). The open gap is on-target runtime verification.**

## 2026-07-14

- **M0 — Foundation:** established a Linux/WSL build+test path (WSL Ubuntu, Rust 1.95); hardened CI
  (rustfmt + `clippy -D warnings` + `cargo audit` + cross-platform build matrix); migrated
  `log`/`env_logger` → `tracing`; added `config::ConfigError` (`thiserror`).
- **M1 — QUIC transport core:** added `transport::Transport` seam + `quinn` `QuicTransport`
  (tunneled packets as QUIC datagrams); versioned control-stream handshake with parameter
  negotiation (`transport::control`, `postcard`); QUIC keep-alive + idle-timeout; client reconnect
  with exponential backoff. Removed the TLS-over-TCP path (`net/tls.rs`, `net/tun.rs`,
  `net/clients.rs`, old bins) and dropped rustls 0.21 / tokio-rustls / webpki-roots /
  rustls-pemfile / x509-parser / tun 0.6 / winapi.
- **M2 — cross-platform TUN:** `net::device::{TunDevice, SystemTun}` via `tun-rs`; **the crate now
  builds natively on Windows** (was failing at session start). `engine::{run_server,run_client}`
  wire the TUN to QUIC datagrams (single-peer P2P); multi-client scaffolding removed.
- **M2 — network config:** `net::netcfg::NetConfigurator` abstracts host routing/NAT with rollback
  on drop; `LinuxNetConfigurator` wraps `ip`/`iptables`/`sysctl` (warn-noop on other platforms),
  wired into the engine (server NAT via `--nat-interface`, client subnet route).
- **M3 — security (complete):** `crypto::NodeIdentity` (self-signed, load-or-generate, `Zeroizing`
  key, `0600` perms); `vpn-rust keygen` subcommand; QUIC client pins the peer certificate; SHA-256
  fingerprints logged for out-of-band (TOFU) verification; `certs/*.{der,key,crt,pem}` gitignored.
- **Milestone status:** M0–M3 complete (the full cross-platform QUIC VPN core); **M4 (TUI) and M5
  (release readiness) remain**.
- Tests: 18 unit + 2 loopback integration (QUIC echo, control handshake) + 2 doc, green on Linux;
  clippy `-D warnings` + fmt clean; native Windows `cargo build` succeeds. Committed in ~14 clean
  increments.

## 2026-07-13

- Populated all BEACON framework documentation with real VPN-Rust architecture and current
  project state (STATUS, HANDOFF, DECISIONS, OPEN_QUESTIONS, RISKS), replacing the generic
  "project" placeholder templates. Framework governance files left as pre-populated boilerplate.

## 2026-06-06

- Project scaffolded and migrated to BEACON Framework.

---

## Historical Code Milestones (context)

Summarized from git history and `docs/TASKS.md`; the implementation work predates BEACON onboarding
(dated Dec 2024 in the task log).

- **Phase 1 — Core infrastructure:** async TUN interface, rustls TLS tunnel, length-prefixed
  packet protocol, echo tunnel, GitHub Actions CI, doc comments, anyhow error handling,
  structured logging, constants module.
- **Phase 2 — VPN functionality:** bidirectional forwarding, route management + NAT + IP
  forwarding, application-level keepalive (10s) + reconnect with exponential backoff (1s -> 30s).
- **Phase 3 — CLI & usability:** unified `vpn-rust` binary with clap `server`/`client`
  subcommands, TOML configuration, ratatui TUI dashboard.
- **Phase 4 — Production features:** mTLS client-cert auth, multi-client support
  (`ClientManager` + DHCP-like `IpPool`), kill switch + DNS/IPv6 leak prevention.
- **Recent git history:** Initial commit -> First Progress Check -> README -> GitHub Actions for
  Rust -> compile fixes (`f68febd`) -> merge PR #1 (codex/find-and-fix-code-errors) -> docs
  updates (`e4954ea`, `1c60473`).
