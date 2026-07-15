---
title: "RISKS"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Risks
author: Derek Martinez
---

# Risks — VPN-Rust

## R-1: Windows/macOS build blocked / cross-platform portability

**Severity:** High → Low
**Status:** Resolved (M2) — build fixed; runtime unverified
**Mitigation:** The Linux-only `net/tun.rs` is deleted; `net::device::SystemTun` (via `tun-rs`)
provides cross-platform async TUN, and **the crate now compiles natively on Windows** and Linux.
Residual: the Windows (wintun) / macOS (utun) clients have not been *run* on real hosts — see R-12.

## R-2: Minimal test coverage / no integration tests

**Severity:** High → Medium
**Status:** Mitigated (M0/M1)
**Mitigation:** Root-free loopback integration tests exist (QUIC datagram echo, control handshake)
plus CI gates (clippy `-D warnings` / fmt / cargo-audit) and unit tests (36, incl. `LiveStats`,
`logbuf`, and headless `TestBackend` dashboard renders). Still wanted: a full data-plane end-to-end
test (needs root or netns) and a live-tunnel run of the `--tui` dashboard.

## R-3: Prototype self-signed certificates, no revocation, not production-audited

**Severity:** Medium → Low
**Status:** Resolved (M3) — not externally audited
**Mitigation:** The self-signed `gen_certs.sh` flow is replaced by `crypto::NodeIdentity` +
`vpn-rust keygen`; the peer is pinned by its exact certificate and identified by a SHA-256
fingerprint (no CA/PKI). Private keys are zeroized and `0600`. Residual: no external security audit
(acceptable for a personal project; note it before any wider use).

## R-4: Requires root / CAP_NET_ADMIN

**Severity:** Medium
**Status:** Open
**Mitigation:** TUN interface creation and routing require elevated privileges. Document the
requirement clearly; consider granting CAP_NET_ADMIN for unprivileged operation (OQ-9).

## R-5: No DNS-through-tunnel — potential DNS leak

**Severity:** Medium
**Status:** Open
**Mitigation:** DNS is not routed through the tunnel; leaks are only *prevented* via iptables
rules in `src/net/security.rs`. If the rules are bypassed or misconfigured, DNS queries can leak.
Implement DNS-through-tunnel (OQ-7) to close the gap.

## R-6: Runtime performance unmeasured; TUI blocks a worker thread

**Severity:** Low/Medium
**Status:** Open
**Mitigation:** The latency/throughput figures in the docs are targets, not measurements. The
runtime is the default multi-threaded Tokio runtime (`#[tokio::main]`), which is what lets the
`--tui` design run the engine on a background task while the blocking dashboard event loop occupies
one worker (D-21) — so the engine keeps progressing. Residual: profile hot paths under load before
making performance claims, and consider moving the dashboard's input polling off a worker thread
(e.g. `spawn_blocking` or an async event stream) if worker-thread pressure ever matters.

## R-7: Prototype mTLS trust model assumes trusted endpoints

**Severity:** Medium → Low
**Status:** Resolved (M3)
**Mitigation:** Replaced by certificate pinning (D-13/D-19): the client trusts only the pinned
server certificate. Residual: client→server auth is not yet enforced (server accepts any client);
mutual pinning is a straightforward follow-up if wanted.

## R-8: QUIC transport rewrite scope

**Severity:** Medium → resolved
**Status:** Done (M1)
**Mitigation:** The QUIC transport landed behind a `Transport` seam, validated incrementally with
loopback integration tests; `tls.rs` and the length-prefixed protocol are deleted. No big-bang
cutover occurred.

## R-9: Dependency major-version upgrade breakage

**Severity:** Low
**Status:** Mostly done
**Mitigation:** The rustls-0.21 stack is gone (quinn brings rustls 0.23); `tun` 0.6 → `tun-rs`;
webpki-roots/tokio-rustls/rustls-pemfile/x509-parser dropped — all folded into their milestone
rewrites (D-18) and test-validated. Remaining: **ratatui 0.25 → 0.29** (lands with the M4 TUI).

## R-10: Pinned-key UX and rotation

**Severity:** Low/Medium
**Status:** Open — design in M3
**Mitigation:** SPKI-fingerprint pinning (D-13) means rotating a node's key requires re-pinning on
the peer, and first-connection trust relies on out-of-band fingerprint verification (TOFU).
Mitigate with clear fingerprint display, a `vpn keygen` flow, and documented re-pin steps so key
changes are an expected, low-friction operation.

## R-11: Cannot compile/verify on the current Windows dev host until a Linux/WSL path exists

**Severity:** High → resolved
**Status:** Resolved (M0)
**Mitigation:** A WSL Ubuntu (Rust 1.95) build+test path is established and used for all
verification; the crate also builds natively on Windows now. Local edits are compile/test-verified
via WSL.

## R-12: macOS/Windows runtime and full-tunnel behavior unverified on real hosts

**Severity:** Medium
**Status:** Open
**Mitigation:** The crate compiles for Windows/macOS, but the wintun (needs `wintun.dll`) and utun
clients have not been *run* on real hosts, and the full packet-forwarding path has not been
exercised end-to-end (needs root; the dev WSL has no passwordless sudo). `NetConfigurator` is a
warn-noop on macOS/Windows (B-022). Validate on real Linux/macOS/Windows hosts (or netns) before
claiming cross-platform runtime support.
