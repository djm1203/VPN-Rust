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

This session executed **M4 in full and the substantive M5 items**, on top of the already-complete
M0–M3 core. VPN-Rust now has a **live TUI control cockpit** over the QUIC P2P engine.

- **M4 (TUI):** ratatui 0.25→0.29 / crossterm 0.28. New `engine::stats::LiveStats` (`Arc`, mostly
  atomics) is written by the engine on the hot path (byte/packet counters) and across the lifecycle
  (state, peer, negotiated params, RTT via `quinn::Connection::rtt()`, reconnect attempts), and
  sampled by `tui::Dashboard` each 150 ms tick — which derives throughput history by differencing
  the counters. `tui::ui::render` draws the cockpit; `tui::run_dashboard` owns the terminal + event
  loop; `tui::logbuf::{LogBuffer, LogLayer}` diverts `tracing` into a bounded ring the log panel
  renders. Wired via `--tui` (engine on a `tokio::spawn` task, dashboard in the foreground; quitting
  aborts the engine) and a headless `--daemon` flag (ANSI-off logging; conflicts with `--tui`).
- **M5:** docs (`QUICKSTART.md`, `THREAT_MODEL.md`, `WIRE_PROTOCOL.md`); packaging (`release.yml`
  matrix release workflow, `packaging/systemd/vpn-rust-server.service`, `packaging/` docs).
- **Fix:** boxed `toml::de::Error` in `ConfigError` (clippy `result_large_err`, newly enforced).
- **How it was built:** the M4 spine (deps, `LiveStats`, engine instrumentation, `logbuf`) was
  written directly; then **three parallel subagents** (disjoint file sets — `src/tui/` vs `docs/`
  vs `.github/`+`packaging/`) produced the TUI build-out, docs, and packaging; integration (`--tui`
  / `--daemon` wiring, the config fix) was done after.
- **Verified:** 36 unit + 2 integration + 2 doc (40 total) green; `clippy -D warnings` + `fmt`
  clean; native Windows `cargo build` and `cargo build --release` succeed; CLI smoke test confirms
  `--tui`/`--daemon` parse, the conflict is enforced, and `keygen` works. **The interactive TUI has
  not been run against a live tunnel** (needs root for the TUN + a real terminal).
- **State:** changes are **uncommitted** — commit only on the operator's request (R-10.5).

## What's Next

The core + UX + release scaffolding are done. What remains:

1. **On-target verification (highest value):** run `sudo ./target/release/vpn-rust server` and a
   `client` end-to-end with root on Linux (or a netns pair); drive the `--tui` dashboard against the
   live session to confirm counters/state/RTT/sparklines update; validate Windows (wintun +
   `wintun.dll`, admin) and macOS (utun, sudo) clients on real hosts.
2. **M5 leftovers (LOW):** standalone metrics export (B-038 — live metrics already reach the TUI);
   SemVer enforcement tooling (B-042 — already documented in `WIRE_PROTOCOL.md`).
3. **Small items:** inner-MTU/PMTU clamp (B-016); config validation with actionable errors (B-029);
   native macOS/Windows `NetConfigurator` (B-022, currently a warn-noop); true `--daemon` detach.

## How to try the dashboard

```bash
# Linux, needs root for the TUN. Server first (generates its identity on first run):
sudo ./target/release/vpn-rust server --bind 0.0.0.0 --port 4433 --tui
# Copy certs/server-cert.der to the client, then:
sudo ./target/release/vpn-rust client --server <host> --server-cert server-cert.der --tui
# Keys: q/Esc quit · f cycle log filter · ↑/↓/PgUp/PgDn scroll · g/Home top · c clear · ?/h help
```

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
