---
title: "BUILD AND TEST"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: BuildAndTest
author: Derek Martinez
---

# Build & Test — VPN-Rust

> **Platform note:** VPN-Rust builds and runs on **Linux only**. `src/net/tun.rs` uses
> `File::from_raw_fd` and `tokio::io::unix::AsyncFd`, which are Unix/Linux-specific, so
> `cargo build` and `cargo test` **fail on a Windows host**. Build and test on Linux (or WSL).

## Build Commands

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Format check (does not modify files)
cargo fmt --check

# Apply formatting
cargo fmt

# Lint
cargo clippy
```

## Test Commands

```bash
# Run unit + doc tests (Linux only)
cargo test

# Verbose (matches CI invocation)
cargo test --verbose
```

## Build System

- **Tool:** Cargo (Rust 2021 edition, stable toolchain).
- **Manifests:** `Cargo.toml` (dependency + package manifest), `Cargo.lock` (pinned versions).
- **Binary:** unified `vpn-rust` (`default-run = "vpn-rust"`) with `server` / `client`
  subcommands. Legacy `bin/server` and `bin/client` targets also exist under `src/bin/`.

## CI Pipeline

Defined in `.github/workflows/ci.yml` (workflow name **"Rust CI"**). Triggers on push and
pull requests targeting `main`. Single job `build` on `ubuntu-latest`:

1. **Checkout** — `actions/checkout@v3`.
2. **Set up Rust** — `actions/setup-rust@v1`, `rust-version: stable`.
3. **Build** — `cargo build --verbose`.
4. **Test** — `cargo test --verbose`.

> The CI does **not** currently run `cargo clippy`, `cargo fmt --check`, or `cargo audit`.
> Adding those as gates is tracked in the backlog (B-016). Formatting and linting are expected
> to be run locally before committing (see `docs/CLAUDE.md`).

## Test Categories

| Category | Command | Status | Notes |
|----------|---------|--------|-------|
| Unit | `cargo test` | Minimal | A few config-parsing unit tests plus `///` doc tests. |
| Integration | `cargo test` (planned) | Deferred | TLS handshake and packet-roundtrip tests planned (backlog B-002); none exist yet. |
| E2E / manual | run server + client, then `ping 10.8.0.1` | Manual | Requires two terminals on Linux with root; see below. |

### Manual end-to-end test (Linux, root)

```bash
# Terminal 1 — server (root required for TUN)
sudo RUST_LOG=debug cargo run -- server

# Terminal 2 — client (root required for TUN)
sudo RUST_LOG=debug cargo run -- client

# Terminal 3 — connectivity check from client side
ping 10.8.0.1

# Optional: watch tunnel traffic
sudo tcpdump -i rustvpn0 -n
```

## Code Coverage

- **Target:** 80%+ for new code (advisory, not enforced).
- **Tool:** none configured yet (no `tarpaulin`/`llvm-cov` in the pipeline).
- **Enforcement:** none — CI does not gate on coverage. Current coverage is minimal.

## Build Artifacts

- Debug binary: `target/debug/vpn-rust`
- Release binary: `target/release/vpn-rust`
- `target/` is gitignored; no artifact retention/publishing configured.

## Prerequisites

- Rust stable toolchain (`rustup`), edition 2021.
- Linux with TUN/TAP support (`sudo modprobe tun`; verify with `lsmod | grep tun`).
- Root or `CAP_NET_ADMIN` capability (TUN creation and route/iptables changes require it).
- Test certificates generated via `./gen_certs.sh` before running the tunnel.
- Cleanup helper: `./cleanup_vpn.sh` (or `sudo ip link delete rustvpn0 / rustvpn1`).
