---
title: "EXECUTION PLAN"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: ExecutionPlan
author: Derek Martinez
---

# Execution Plan — VPN-Rust

## Direction

VPN-Rust is moving from a **learning prototype** to **production-ready personal software**.

**Product (decided 2026-07-13):**
- **Transport:** QUIC over UDP via `quinn` — fixes the TCP-over-TCP meltdown of the old
  TLS-over-TCP design, keeps TLS 1.3 (rustls), adds congestion control.
- **Topology / scope:** personal **point-to-point** — one **Linux server** the operator hosts,
  plus **Linux / macOS / Windows** clients the operator owns. No multi-user PKI.
- **Auth:** **pinned keypairs** (SPKI fingerprint pinning, generated with `rcgen`); no CA.
- **UX:** the **TUI is the primary control dashboard** — connect/disconnect, live graphs, peer
  and route panels, log viewer, theming.

See the sequenced items in [BACKLOG.md](BACKLOG.md), rationale in
[../state/DECISIONS.md](../state/DECISIONS.md) (D-10…D-18), and the target design in
[../architecture/ARCHITECTURE.md](../architecture/ARCHITECTURE.md).

## Guiding principles

1. **Compile-verified progress.** Work happens where it can build and run (Linux/WSL); no
   milestone is "done" without green CI on its target platforms.
2. **Refactor toward seams.** Introduce trait boundaries (`Transport`, `TunDevice`,
   `NetConfigurator`) so platforms and wire formats are swappable and testable.
3. **Fold upgrades into the rewrite.** Dependency modernization rides along with the milestone
   that touches each subsystem rather than as a separate big-bang.
4. **Security by construction.** Pinned keys, zeroized secrets, no payload logging in release.

---

## Milestones

### M0 — Foundation & unblock  ·  *status: NEXT*

**Goal:** a crate that builds and a CI that can actually catch regressions.

**Deliverables:** Linux/WSL build+test path; multi-OS CI matrix; clippy/fmt/audit gates;
root-free loopback integration-test harness; `tracing` logging; `thiserror` error seams.

**Exit criteria:** `cargo build`/`test`/`clippy`/`fmt --check` green in CI on ubuntu (and the
Windows/macOS legs building what compiles today); at least one loopback integration test running
without root.

### M1 — QUIC transport core

**Goal:** the tunnel runs over QUIC.

**Deliverables:** `Transport` trait; `quinn` implementation; IP packets over QUIC datagrams;
versioned control stream; keepalive/reconnect on QUIC; old TLS-over-TCP path removed.

**Exit criteria:** two nodes complete a QUIC handshake and pass IP packets end-to-end over
loopback in an integration test; TCP path deleted; protocol version negotiated on connect.

### M2 — Cross-platform TUN + network config

**Goal:** clients run on Linux, macOS, and Windows; routing is abstracted and reversible.

**Deliverables:** `TunDevice` trait + `tun-rs` backends (Linux/utun/wintun); `NetConfigurator`
trait + per-OS implementations with rollback; config-driven addressing; multi-client scaffolding
collapsed to a single-peer session.

**Exit criteria:** **Windows and macOS clients build and establish a tunnel to the Linux server**;
routes/NAT are set and cleanly torn down on exit and on crash.

### M3 — Security hardening (P2P)

**Goal:** a trustworthy pinned-key model replacing self-signed/CA trust.

**Deliverables:** `vpn keygen` (`rcgen`); SPKI-fingerprint pinning verifier; fingerprint
display/TOFU; `zeroize` + key-file permission checks; config validation; no-payload logging.

**Exit criteria:** a connection is rejected on fingerprint mismatch; private keys are zeroized and
never logged; misconfig produces actionable errors.

### M4 — TUI control dashboard

**Goal:** a polished cockpit that is the main way to run and watch the VPN.

**Deliverables:** ratatui 0.29 upgrade; event-driven UI fed by an engine channel; state-machine
view; throughput sparklines + RTT gauge; peer/route panels; filterable log viewer; connect/
disconnect/reconnect controls + keybindings + help; theming.

**Exit criteria:** the VPN can be fully operated from the TUI (connect → observe live stats →
disconnect) with a coherent, themed, responsive layout.

### M5 — Release readiness

**Goal:** installable, documented, releasable.

**Deliverables:** metrics surface; `--daemon` + systemd unit; per-OS client packaging; quickstart
+ threat-model + wire-protocol docs; SemVer discipline.

**Exit criteria:** a clean-machine operator can install the server + a client from artifacts and
docs alone and establish a tunnel.

---

## Timeline notes

- **Prototype foundation** (TUN, TLS-over-TCP tunnel, routing/NAT, CLI/TUI, mTLS + multi-client
  scaffolding, CI) — implemented through late 2024.
- **BEACON onboarding + full documentation** — 2026-07-13.
- **Pivot to production QUIC VPN** — decided 2026-07-13; execution begins at **M0**.

## Superseded from the prototype

- TLS-over-TCP length-prefixed protocol → replaced by QUIC (M1).
- Self-signed certs + CA/webpki-roots trust + opt-in mTLS → replaced by pinned keypairs (M3).
- Linux-only `from_raw_fd`/`AsyncFd` TUN → replaced by the `TunDevice` abstraction (M2).
- Multi-client `ClientManager`/`IpPool` → re-scoped to single-peer P2P (M2).
