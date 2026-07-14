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
