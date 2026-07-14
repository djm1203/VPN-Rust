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
