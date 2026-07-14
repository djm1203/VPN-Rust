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

This session committed to the production pivot **and executed M0–M3 — the full functional VPN
core**. VPN-Rust is now a **QUIC/UDP point-to-point VPN**: `engine::{run_server,run_client}` pump IP
packets between a cross-platform `TunDevice` (`tun-rs`) and QUIC datagrams (`quinn`), with a
versioned control handshake, QUIC keep-alive, client reconnect, **pinned-certificate** auth
(`keygen`, fingerprints, zeroized keys), and host **NAT/routing** via `NetConfigurator`. The legacy
TLS-over-TCP stack is deleted. **Only M4 (TUI) and M5 (release readiness) remain.**

- **The crate now builds natively on Windows** (was 4 compile errors at session start) and on
  Linux; a WSL Ubuntu path runs `cargo build`/`test`/`clippy`/`fmt`.
- Committed in clean increments (see `git log`): docs+CI, tracing+thiserror, QUIC spike, control
  handshake, TunDevice, engine+identity, binary rewire + legacy removal, cert-SAN fix.
- **Verified:** 17 unit + 2 loopback integration + 2 doc tests green; `clippy -D warnings` + `fmt`
  clean; the real binary generates its identity and binds the QUIC endpoint (TUN creation is
  root-gated, so full packet flow is not yet exercised here — no passwordless sudo in WSL).

## What's Next

Next session starts at **M4 (TUI control dashboard)** — the primary UX:

1. **M4 — TUI control dashboard (start here):**
   - Instrument the engine with a shared live-stats handle (connection state, bytes/packets
     up/down, RTT from `quinn::Connection::rtt()`, negotiated params, peer address). Thread it
     through `engine::pump` and the client/server session functions.
   - Upgrade `ratatui` 0.25→0.29 and `crossterm` 0.27→0.29; rewrite `src/tui/*` as an event-driven
     cockpit: state view, up/down throughput sparklines, RTT gauge, byte/packet counters,
     peer/route panels, filterable log viewer (fed by a `tracing` layer), keybindings + help
     overlay, dark theme + cyan accent (unless the operator says otherwise).
   - Add a `--tui` mode (run engine + TUI concurrently). **Verify headlessly with ratatui's
     `TestBackend`** (render a frame, assert on the buffer) since there's no interactive terminal in
     the dev harness.
2. **M5 — Release readiness:** metrics, `--daemon` + systemd unit, per-OS packaging, docs, SemVer.
3. **Small items:** inner-MTU/PMTU clamp (B-016); config validation (B-029); native macOS/Windows
   `NetConfigurator` (B-022, currently a warn-noop).
4. **On-target verification:** run the tunnel end-to-end with root on Linux (or netns); validate
   Windows (wintun) and macOS (utun) clients on real hosts.

## Quick Reference

Build/test run on the WSL Ubuntu path (this Windows host can't create TUN devices):

```bash
# In WSL Ubuntu, from the repo (on /mnt/c or a native clone):
cargo build --all-targets
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# Native Windows build also works now:
#   cargo build            (from PowerShell)

# Run the VPN (Linux, requires root for TUN). Server generates its identity on
# first run at certs/server-cert.der — pin that file on the client:
sudo ./target/debug/vpn-rust server --bind 0.0.0.0 --port 4433
sudo ./target/debug/vpn-rust client --server <host> --server-cert server-cert.der
```

## What to Watch

- **Full tunnel not yet exercised end-to-end** — needs root (no passwordless sudo in the dev WSL);
  use `sudo` on a real Linux box or network namespaces. The QUIC/handshake/identity paths are
  covered by loopback integration tests and a binary smoke test.
- **No routing/NAT yet** — packets move between the two TUNs, but host routes and server NAT are
  not configured until the `NetConfigurator` work (M2, B-020–022). The Linux `route`/`security`
  modules are present but unwired (gated `cfg(unix)`).
- **Pinning is cert-exact, not SPKI-fingerprint yet** — refine in M3 (with fingerprint display /
  TOFU and `zeroize`). Don't treat the current model as final.
- **TUI is stale** — the prototype ratatui 0.25 dashboard is unused by the engine; rebuild in M4.
- **Windows/macOS runtime unverified** — the crate compiles for both, but wintun (needs
  `wintun.dll`) and utun have not been run on real hosts.
