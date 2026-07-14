---
title: "DEPENDENCIES"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Dependencies
author: Derek Martinez
---

# Dependencies — VPN-Rust

## Dependency Manifests

- `Cargo.toml` — declared dependencies and features.
- `Cargo.lock` — exact pinned versions (committed; source of truth for reproducible builds).

Package: `vpn-rust` v0.1.0, Rust 2021 edition.

## Runtime Dependencies

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `tokio` | 1.38 | Async runtime and task scheduling | `features = ["full"]` |
| `ratatui` | 0.25.0 | Terminal UI framework | Used by `src/tui/` |
| `crossterm` | 0.27 | Terminal backend for ratatui (input/raw mode) | |
| `clap` | 4.5 | CLI argument parsing | `features = ["derive"]`; `server`/`client` subcommands |
| `serde` | 1.0 | Serialization framework | `features = ["derive"]`; config structs |
| `toml` | 0.8 | TOML config parsing | Used by `src/config/toml_config.rs` |
| `rustls` | 0.21 | Pure-Rust TLS implementation | Memory-safe; no OpenSSL dependency |
| `webpki-roots` | 0.22.6 | Bundled Mozilla root CA store | Client-side server verification |
| `x509-parser` | 0.16 | X.509 certificate parsing | mTLS — client cert CN extraction |
| `tun` | 0.6 | Virtual TUN/TAP interface creation | Linux-only in current usage |
| `anyhow` | 1.0 | Application-level error handling | Used with `.with_context()` |
| `log` | 0.4.27 | Logging facade | Paired with `env_logger` |
| `env_logger` | 0.11.8 | `RUST_LOG`-driven logger backend | |
| `tokio-rustls` | 0.24 | Async TLS (tokio + rustls integration) | |
| `rustls-pemfile` | 1.0 | Load certs/keys from PEM files | |

## Platform-Specific Dependencies

| Crate | Version | Target | Purpose | Notes |
|-------|---------|--------|---------|-------|
| `winapi` | 0.3.9 | `cfg(windows)` | Windows networking APIs | `features = ["iphlpapi"]`; declared for future Windows support, backend not yet implemented |

## Development Dependencies

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `tokio-test` | 0.4.4 | Async test utilities | For async unit/integration tests |

## Dependency Policy

- **Prefer vetted, memory-safe crates.** rustls is deliberately chosen over OpenSSL to avoid a
  system C dependency and gain memory safety and modern TLS defaults.
- **Pin versions via `Cargo.lock`** (committed) for reproducible builds.
- **Review before adding** — assess license, maintenance activity, and security posture of any
  new dependency before introducing it.
- Actively used across the app: `clap` (CLI), `ratatui`/`crossterm` (TUI), `serde`/`toml`
  (config), `tokio`/`tokio-rustls`/`rustls` (async TLS tunnel).

## Update Policy

- **Patch updates:** apply promptly once tests pass.
- **Minor updates:** evaluate, apply if tests pass.
- **Major updates:** schedule and test thoroughly before adoption (note rustls, tokio-rustls,
  and webpki-roots versions are interdependent — bump together).
- **Security patches:** apply promptly for critical advisories.

## Audit

- No security audit has been run yet. `cargo audit` is **not** currently part of CI.
- Recommended: add `cargo audit` (RustSec advisory DB) as a CI step — tracked in backlog B-016.
- No known vulnerable dependencies have been identified, but this has not been formally verified.

## License Compliance

- All dependencies above are permissively licensed (MIT/Apache-2.0 dual-license is typical
  across the Rust ecosystem) and compatible with this project.
- Copyleft (GPL) dependencies would require review before inclusion.
