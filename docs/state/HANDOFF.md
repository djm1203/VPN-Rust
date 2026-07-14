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

This session committed to the production pivot **and executed M0, M1, and most of M2**. VPN-Rust is
now a **QUIC/UDP point-to-point VPN**: `engine::{run_server,run_client}` pump IP packets between a
cross-platform `TunDevice` (`tun-rs`) and QUIC datagrams (`quinn`), with a versioned control
handshake, QUIC keep-alive, client reconnect, and **pinned-certificate** authentication. The legacy
TLS-over-TCP stack is deleted.

- **The crate now builds natively on Windows** (was 4 compile errors at session start) and on
  Linux; a WSL Ubuntu path runs `cargo build`/`test`/`clippy`/`fmt`.
- Committed in clean increments (see `git log`): docs+CI, tracing+thiserror, QUIC spike, control
  handshake, TunDevice, engine+identity, binary rewire + legacy removal, cert-SAN fix.
- **Verified:** 17 unit + 2 loopback integration + 2 doc tests green; `clippy -D warnings` + `fmt`
  clean; the real binary generates its identity and binds the QUIC endpoint (TUN creation is
  root-gated, so full packet flow is not yet exercised here — no passwordless sudo in WSL).

## What's Next

Pick up at the **M2/M3 remainder**, then M4/M5:

1. **`NetConfigurator` (M2, B-020–022):** trait for address/route/NAT/DNS with rollback; Linux impl
   wrapping the existing `net::route`/`net::security`; macOS/Windows impls. Wire server NAT + client
   routes into the engine. Add the inner-MTU/PMTU clamp (B-016).
2. **Security hardening (M3):** `keygen` CLI subcommand; SPKI-fingerprint pinning + fingerprint
   display (TOFU); `zeroize` private keys; config validation.
3. **M4 — TUI control dashboard:** ratatui 0.25→0.29; event-driven cockpit (connect/disconnect,
   live throughput graphs, RTT, peer/route panels, log viewer, keybindings, theming) fed by an
   engine stats/event channel.
4. **M5 — Release readiness:** metrics, `--daemon` + systemd unit, per-OS packaging, docs, SemVer.
5. **On-target verification:** run the tunnel end-to-end with root on Linux (or netns); validate
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
