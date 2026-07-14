---
title: "HANDOFF"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Handoff
author: Derek Martinez
---

# Handoff

## Where We Stopped

This session **committed to the production-direction pivot** and documented it. VPN-Rust is going
from a learning prototype to production-ready personal software: a **QUIC/UDP point-to-point VPN**
between machines the operator owns (one Linux server + Linux/macOS/Windows clients), authenticated
by **pinned keypairs** (no CA/PKI), driven from a **TUI control dashboard**.

- The core planning docs are rewritten to this direction: `docs/planning/BACKLOG.md`,
  `docs/planning/EXECUTION_PLAN.md` (milestones M0–M5), `docs/state/DECISIONS.md` (D-10…D-18), and
  `docs/architecture/ARCHITECTURE.md`. The derivative docs (STATUS, HANDOFF, RISKS,
  OPEN_QUESTIONS, PRD, CAPABILITIES) are aligned to them.
- The **framework governance files** (`docs/governance/`, `docs/process/`, and other BEACON
  scaffolding) remain as pre-populated boilerplate and still need project-specific review.
- Nothing committed this session (per commit policy, commit only when explicitly asked).

Code-wise, the project is unchanged: the v0.1.0 prototype (Phases 1-4) is implemented (see
STATUS.md). The build succeeds on Linux and **fails to compile on Windows** (the current dev host)
due to Linux-only TUN code, so `cargo test` cannot run here.

**The next session starts M0 (Foundation & unblock).**

## What's Next

**M0 — Foundation & unblock (do these in order):**

1. **Establish a Linux/WSL build + test path FIRST** — the crate cannot compile on this Windows
   host, so nothing else can be verified until there is a place `cargo build`/`test` runs.
2. **CI matrix + gates** — multi-OS CI with clippy, `cargo fmt --check`, and `cargo audit`.
3. **Root-free loopback integration-test harness** — exercise networking without root /
   CAP_NET_ADMIN.
4. **Migrate `log` → `tracing`** (D-14).
5. **Introduce `thiserror` error seams** (D-15) and the trait seams (`Transport`, `TunDevice`,
   `NetConfigurator`, D-16).

**Then M1 — QUIC transport core:** add the `Transport` trait and a `quinn` implementation (IP
packets over QUIC datagrams + a versioned reliable control stream); **this deletes `tls.rs`** and
the length-prefixed TCP protocol.

Later: M2 cross-platform TUN + `NetConfigurator`; M3 pinned-key hardening; M4 TUI dashboard; M5
release readiness.

## Quick Reference

```bash
# Build
cargo build
cargo build --release

# Test / lint / format (test requires Linux)
cargo test
cargo clippy
cargo fmt

# Run the VPN (Linux, requires root for TUN)
sudo cargo run --bin server
sudo cargo run --bin client

# Certificates and cleanup
./gen_certs.sh        # generate self-signed dev certs
./cleanup_vpn.sh      # tear down TUN interfaces / processes
```

## What to Watch

- **Cannot compile/verify on this Windows host until M0/M2** — the current dev host cannot build
  or test; do all work on the Linux/WSL path M0 establishes. Cross-platform client builds land in
  M2 (`TunDevice` / `tun-rs`).
- **The QUIC rewrite deletes `tls.rs`** — M1 removes the TLS-over-TCP transport and the
  length-prefixed framing entirely; don't invest in that code.
- **Dependency majors upgrade *within* milestones** (D-18) — rustls 0.21→0.23 + drop
  webpki-roots with QUIC (M1), `tun` 0.6→`tun-rs` with cross-platform TUN (M2), ratatui 0.25→0.29
  with the TUI (M4). Don't do a big-bang bump.
- **Minimal test coverage** — networking code is largely unguarded until the M0 loopback harness
  exists; verify manually on Linux until then.
- **Self-signed certificates are prototype-only** — superseded by pinned keypairs in M3; do not
  treat the current trust model as production-secure.
