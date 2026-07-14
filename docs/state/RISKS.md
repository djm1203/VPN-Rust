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

**Severity:** High
**Status:** Open — mitigated by M2
**Mitigation:** `src/net/tun.rs` uses Linux-only APIs (`File::from_raw_fd` + `AsyncFd`) and fails
to compile on Windows (E0432/E0433/E0599), so the current Windows dev host cannot build or run
`cargo test`. The production direction requires Linux/macOS/Windows clients: M2 introduces the
`TunDevice` trait backed by `tun-rs` (Linux/utun/wintun), which closes this (D-12). Until then,
M0 establishes a Linux/WSL build+test path so work can proceed.

## R-2: Minimal test coverage / no integration tests

**Severity:** High
**Status:** Open — mitigated by M0
**Mitigation:** Only a few config-parsing unit tests plus doc tests exist; networking code is
largely unguarded. M0 adds a **root-free loopback integration-test harness** plus CI gates
(clippy / fmt / audit); the trait seams (D-16) make transport and TUN mockable so packet paths can
be tested without root.

## R-3: Prototype self-signed certificates, no revocation, not production-audited

**Severity:** Medium
**Status:** Superseded by M3 (pinned keypairs)
**Mitigation:** The prototype uses self-signed certs (`gen_certs.sh`) with no revocation. This is
superseded by the pinned-keypair model (D-13): each node generates a keypair with `rcgen` and pins
its peer by SPKI fingerprint (no CA/PKI). Hardening lands in M3. Prototype trust model remains only
until then.

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

## R-6: Single-threaded runtime performance ceiling (unmeasured)

**Severity:** Low/Medium
**Status:** Open
**Mitigation:** The runtime is single-threaded and the latency/throughput figures in the docs are
targets, not measurements. Profile hot paths under load and evaluate a multi-threaded Tokio
runtime before making performance claims.

## R-7: Prototype mTLS trust model assumes trusted endpoints

**Severity:** Medium
**Status:** Superseded by M3 (pinned keypairs)
**Mitigation:** The prototype mTLS trust model assumes both endpoints and their certificates are
trusted, with no external validation authority. Superseded by SPKI-fingerprint pinning (D-13),
which gives strong mutual authentication between known peers without a CA. Hardening lands in M3.

## R-8: QUIC transport rewrite scope

**Severity:** Medium
**Status:** Open — planned (M1)
**Mitigation:** Replacing TLS-over-TCP with QUIC/`quinn` (datagrams for packets + reliable control
stream, D-11) is a substantial rewrite that deletes `tls.rs` and the length-prefixed protocol.
Mitigate by landing the `Transport` trait seam first (D-16) and validating incrementally with the
root-free loopback integration tests from M0, rather than a big-bang cutover.

## R-9: Dependency major-version upgrade breakage

**Severity:** Medium
**Status:** Open — mitigated by D-18
**Mitigation:** Several deps are majors behind (rustls 0.21→0.23, tokio-rustls, rustls-pemfile
1→2, drop webpki-roots, ratatui 0.25→0.29, `tun` 0.6→`tun-rs`, x509-parser). API breakage is
mitigated by **folding each upgrade into the milestone that rewrites its subsystem** (D-18) so the
same tests that cover the rewrite validate the upgrade — rather than a separate big-bang bump.

## R-10: Pinned-key UX and rotation

**Severity:** Low/Medium
**Status:** Open — design in M3
**Mitigation:** SPKI-fingerprint pinning (D-13) means rotating a node's key requires re-pinning on
the peer, and first-connection trust relies on out-of-band fingerprint verification (TOFU).
Mitigate with clear fingerprint display, a `vpn keygen` flow, and documented re-pin steps so key
changes are an expected, low-friction operation.

## R-11: Cannot compile/verify on the current Windows dev host until a Linux/WSL path exists

**Severity:** High (for M0)
**Status:** Open — M0 first task
**Mitigation:** The crate does not build on this Windows host, so no change can be compiled or
tested here today. M0's very first deliverable is a Linux/WSL build+test path (plus CI on
ubuntu); until it exists, treat all local edits as unverified.
