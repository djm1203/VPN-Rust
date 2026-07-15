---
title: "DECISIONS"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Decisions
author: Derek Martinez
---

# Decisions

## D-1: Use TUN (Layer 3) instead of TAP (Layer 2)

**Context:** The tunnel needs to carry client traffic between two endpoints; the choice is between
Layer 3 (IP packets) and Layer 2 (Ethernet frames).

**Decision:** Use a TUN device (L3, `tun::Layer::L3`) on both client (rustvpn1) and server (rustvpn0).

**Alternatives:** TAP (L2) for bridging Ethernet frames.

**Consequences:** Simpler implementation, sufficient for IP tunneling and NAT/routing. No support
for non-IP protocols or L2 bridging; acceptable for the VPN use case.

## D-2: Use rustls instead of OpenSSL

**Context:** The tunnel needs transport encryption with a mature TLS stack.

**Decision:** Use rustls 0.21 with tokio-rustls 0.24 for async integration.

**Alternatives:** OpenSSL via native bindings.

**Consequences:** Memory-safe, pure-Rust TLS with modern TLS 1.2/1.3 defaults and no system
OpenSSL dependency. Bound to the rustls API surface and its certificate/PEM handling conventions.

## D-3: Use Tokio async runtime

**Context:** The client and server must handle TUN I/O and TLS streams concurrently.

**Decision:** Use Tokio for async I/O and task scheduling; `AsyncFd` integrates the TUN file
descriptor into the async reactor; `tokio::spawn` runs the read/write directions concurrently.

**Alternatives:** Thread-per-direction blocking I/O; async-std.

**Consequences:** Industry-standard, full-featured async. A single-threaded runtime is used for
current scope (see D-3 consequence in RISKS R-6). `AsyncFd` ties the TUN path to Unix/Linux (see D-5).

## D-4: Simple length-prefixed wire protocol with keepalive marker

**Context:** IP packets need to be framed over the TLS byte stream.

**Decision:** Prefix each packet with a 2-byte big-endian u16 length, followed by the raw IP
packet. A length of 0 is a keepalive marker (no payload).

**Alternatives:** A richer control protocol with message types/versioning (sketched in
`docs/PLANNING.md` as a future `ControlMessage` enum).

**Consequences:** Trivial to implement and debug; keepalive reuses the same framing. No protocol
versioning, no explicit control channel yet; extension deferred.

## D-5: Linux-first development with AsyncFd (accepting Windows build breakage)

**Context:** TUN handling differs per OS; the primary target and use case is Linux.

**Decision:** Implement `src/net/tun.rs` against Linux APIs (`File::from_raw_fd` + `AsyncFd`) and
treat Linux as the only supported target for now.

**Alternatives:** Abstract a cross-platform TUN layer up front (Wintun/TAP-Windows on Windows,
utun on macOS).

**Consequences:** Clean, focused Linux implementation, but the crate **fails to compile on
Windows** (E0432/E0433/E0599) and `cargo test` cannot run on a Windows dev host. Cross-platform
support is deferred; this is tracked as risk R-1 and open question OQ-8.

## D-6: Unified `vpn-rust` binary with clap subcommands

**Context:** The project has both server and client roles plus separate `bin/server.rs` and
`bin/client.rs`.

**Decision:** Provide a unified `vpn-rust` binary with clap `server`/`client` subcommands
(`--config`, `--verbose`), alongside the standalone bin targets.

**Alternatives:** Two entirely separate binaries with duplicated argument handling.

**Consequences:** Consistent CLI surface and shared argument parsing; a single entry point for
users while retaining the dedicated bin targets for development.

## D-7: Mutual TLS (mTLS) for client authentication

**Context:** The server needs to authenticate clients, not just encrypt transport.

**Decision:** Use mutual TLS — clients present a certificate, the server validates it and extracts
the certificate CN (`get_client_cert_cn`), via `ClientTlsConfig`/`ServerTlsConfig` in
`src/net/tls.rs`.

**Alternatives:** Pre-shared keys, token-based auth (both deferred).

**Consequences:** Certificate-based identity with no additional auth protocol. No certificate
revocation checking yet (risk R-3); relies on the self-signed dev trust model (D-9).

## D-8: Use anyhow for application error handling

**Context:** Networking code has many fallible operations that benefit from contextual errors.

**Decision:** Use `anyhow::Result` throughout with `.with_context()` on error paths; custom error
types deferred as unnecessary for now.

**Alternatives:** Custom error enums via `thiserror`.

**Consequences:** Fast, ergonomic error propagation with good diagnostic context. Less precise
programmatic error matching; acceptable for an application (not a library-API) surface.

## D-9: Self-signed certificates for development

**Context:** The tunnel needs certificates but the project is a learning-focused prototype.

**Decision:** Generate self-signed certificates via `gen_certs.sh` for local development.

**Alternatives:** A real CA-issued chain / managed PKI.

**Consequences:** Zero-friction local setup. Not production-grade: the trust model assumes trusted
endpoints, there is no revocation, and the setup is not security-audited (risks R-3, R-7).

---

> **Production pivot (2026-07-13).** Decisions D-10…D-18 below set the direction for taking
> VPN-Rust from a learning prototype to production-ready personal software. Several supersede
> prototype decisions above; the originals are retained as historical record. See
> [../planning/EXECUTION_PLAN.md](../planning/EXECUTION_PLAN.md).

## D-10: Product scope — personal point-to-point VPN

**Context:** "Production-ready" needs a concrete scope. The operator wants a secure tunnel between
machines they own, not a multi-tenant service.

**Decision:** Target **personal point-to-point**: one operator-hosted **Linux server** plus
**Linux/macOS/Windows** clients the operator owns. No multi-user accounts, no PKI service.

**Alternatives:** Self-hosted multi-user server; distributable product with enrollment/PKI.

**Consequences:** Key management stays simple (pinned keypairs, D-13). The multi-client
`ClientManager`/`IpPool` is re-scoped to a single-peer session (supersedes the multi-client
direction of prototype work). Addressing is point-to-point (`/30` or `/31`).

## D-11: Transport — QUIC over UDP via `quinn` (supersedes D-2 transport, D-4)

**Context:** The prototype tunneled IP inside TLS-over-TCP, which causes TCP-meltdown (throughput
collapse under loss). Production VPNs run over UDP.

**Decision:** Use **QUIC/UDP** via `quinn`. Tunneled IP packets ride **QUIC datagrams**
(unreliable — avoids reliability-over-reliability and head-of-line blocking); a **reliable QUIC
control stream** carries a versioned handshake, keepalive, and config negotiation.

**Alternatives:** Keep TLS-over-TCP; WireGuard-style Noise+UDP (`snow`); DTLS/custom UDP.

**Consequences:** Fixes meltdown; retains TLS 1.3 (rustls under quinn) so existing crypto knowledge
transfers; gains congestion control and 0-RTT resumption. Replaces the length-prefixed framing
(D-4). Requires a `Transport` trait (D-16) and PMTU handling.

## D-12: Cross-platform TUN via a `TunDevice` abstraction (supersedes D-5)

**Context:** The Linux-only `File::from_raw_fd` + `AsyncFd` path breaks the Windows/macOS build.
Clients must run on all three OSes.

**Decision:** Introduce a `TunDevice` trait with backends provided by **`tun-rs`** (Linux, macOS
`utun`, Windows `wintun`), all async.

**Alternatives:** Keep Linux-only; hand-roll per-OS backends with separate crates.

**Consequences:** Removes the `unsafe` fd hack and unblocks cross-platform builds (closes R-1).
Adds a dependency that spans all target OSes; the trait keeps us from being locked to it.

## D-13: Authentication — pinned keypairs, no CA (supersedes D-7, D-9)

**Context:** For personal P2P, a CA/PKI is overkill, and the prototype's opt-in mTLS accepted any
CA-signed cert while `gen_certs.sh` didn't even emit the client/CA material.

**Decision:** Each node generates a keypair/cert with **`rcgen`**; peers pin each other by **SPKI
fingerprint** via a custom rustls certificate verifier. Trust-on-first-use with out-of-band
fingerprint verification is offered.

**Alternatives:** CA-issued chain; pre-shared symmetric keys.

**Consequences:** No CA infrastructure; strong mutual authentication for known peers. Drops
`webpki-roots`. Rotating a key requires re-pinning on the peer (acceptable for P2P).

## D-14: Logging — `tracing` (supersedes the `log` + `env_logger` choice)

**Context:** Async, QUIC, and a live TUI benefit from structured, span-aware, subscribable logs.

**Decision:** Migrate to `tracing` + `tracing-subscriber`; the TUI subscribes to the log stream.

**Alternatives:** Keep `log` + `env_logger`.

**Consequences:** Structured events with spans; the TUI log viewer is fed from the same pipeline.
One-time migration cost across the codebase.

## D-15: Error handling — `thiserror` in the library, `anyhow` at the binary (refines D-8)

**Context:** As modules grow into a reusable core, callers benefit from matchable error types.

**Decision:** Define `thiserror` error enums at module/library boundaries; keep `anyhow` for
context aggregation at the binary entry points.

**Alternatives:** All-`anyhow` (status quo).

**Consequences:** Precise programmatic error handling in the core; ergonomic top-level reporting.

## D-16: Introduce `Transport`, `TunDevice`, and `NetConfigurator` trait seams

**Context:** The prototype welded the engine to concrete Linux/TCP implementations and shelled out
to `ip`/`iptables` inline, which is fragile and untestable.

**Decision:** Define trait seams: `Transport` (wire), `TunDevice` (packet I/O), and
`NetConfigurator` (address/route/NAT/DNS with guaranteed rollback). Concrete impls sit behind them.

**Alternatives:** Continue with concrete, inlined implementations.

**Consequences:** Platforms and wire formats become swappable and mockable; enables root-free
loopback tests. Slightly more indirection.

## D-17: TUI is the primary control surface

**Context:** The operator wants the TUI to be the main, polished way to run and observe the VPN.

**Decision:** Invest in the TUI (ratatui 0.29) as an **event-driven control dashboard** — connect/
disconnect/reconnect controls, live throughput/RTT graphs, peer/route panels, filterable log
viewer, keybindings, help overlay, theming — fed by an engine→UI stats/event channel.

**Alternatives:** Monitoring-only TUI; headless/CLI-first.

**Consequences:** The engine must expose a clean event/stats stream (complements D-14/D-16). Larger
UI surface to design and maintain.

## D-18: Fold dependency modernization into the milestone that touches each subsystem

**Context:** Many deps are majors behind (rustls 0.21, tokio-rustls 0.24, rustls-pemfile 1,
webpki-roots 0.22, ratatui 0.25, `tun` 0.6, x509-parser 0.16).

**Decision:** Upgrade each dependency as part of the milestone that rewrites its subsystem — rustls
stack with QUIC (M1), TUN crate with cross-platform TUN (M2), ratatui with the TUI (M4) — rather
than as a separate big-bang.

**Alternatives:** One large dependency-bump PR up front.

**Consequences:** Upgrades are validated by the same tests that cover the rewrite; less churn on
throwaway code (e.g. no need to port `tls.rs` to rustls 0.23 only to delete it for QUIC).

## D-19: Implementation refinements during the M0–M3 build-out (2026-07-14)

**Context:** Executing M0–M3 required concrete choices that refine (not reverse) earlier decisions.

**Decision:** (a) Pin the peer by its **exact certificate** in a single-entry rustls root store,
and identify/verify it with a **SHA-256 fingerprint of the certificate DER** (`sha256:…`) rather
than an SPKI hash — functionally equivalent for exact-cert pinning and avoids re-adding an X.509
parser. (b) Store node identities as **DER files** (not PEM) to avoid PEM-parsing dependency
friction; private key bytes live in `Zeroizing<Vec<u8>>`. (c) The **`NetConfigurator`** Linux impl
wraps `ip`/`iptables`/`sysctl` (netlink deferred) and is a **warn-noop on macOS/Windows** for now.
(d) Default **inner MTU is 1300** to stay safely under the QUIC datagram size limit on typical
paths. (e) The generated server certificate's SAN equals `--server-name` (default `localhost`) so
the client's hostname check passes; a configurable SAN for arbitrary hostnames is future work.

**Alternatives:** True SPKI-fingerprint pinning with a custom verifier; PEM storage; netlink-based
routing; a fixed 1500 MTU.

**Consequences:** A working, secure, cross-platform core with minimal dependencies. Follow-ups:
SPKI-fingerprint pinning if key-rotation-without-re-pin is wanted (B-025 refinement); native
macOS/Windows `NetConfigurator` (B-022); PMTU discovery (B-016); configurable cert SAN.

## D-20: Engine→TUI telemetry via a shared `LiveStats` handle; TUI samples on a tick (M4)

**Context:** The M4 dashboard (D-17) needs live connection state, throughput, RTT, peer, and
negotiated params from the engine. The obvious "event channel" (mpsc of deltas) couples the engine
hot path to a consumer and risks unbounded growth if the UI stalls.

**Decision:** The engine publishes into an `Arc`-shared **`engine::stats::LiveStats`** — relaxed
atomics for cumulative byte/packet counters (hot path) plus a few `Mutex`-guarded fields (peer,
negotiated params, connect instant) that change only per (re)connect. The **TUI samples a
`StatsSnapshot` each 150 ms render tick** and derives throughput history by *differencing the
cumulative counters* itself, so the engine keeps no second clock and there is no per-packet channel
send. `ConnectionState` is driven from the server/client/connect paths.

**Alternatives:** An mpsc event stream of `AppEvent`s (the prototype's approach); a metrics crate.

**Consequences:** Engine writes are a few `fetch_add`s; the UI owns all rate/graph logic and is
trivially unit-testable via `TestBackend`. A torn read across atomics is invisible at human refresh
rates. `run_server`/`run_client` signatures gained a `stats: Arc<LiveStats>` parameter.

## D-21: TUI runs on the foreground; engine on a background task. Logs divert to a buffer.

**Context:** The dashboard event loop uses blocking `crossterm` polling, and `tracing` output to
stdout would corrupt the alternate screen.

**Decision:** Under `--tui`, spawn the engine future with `tokio::spawn` and run the (blocking)
dashboard loop in the foreground; quitting the dashboard `abort()`s the engine. `tracing` is routed
through a **`tui::logbuf::LogLayer`** into a bounded in-memory ring the dashboard renders, instead
of the stdout `fmt` subscriber. A separate **`--daemon`** flag selects headless operation with
plain (ANSI-off) logging for journald and conflicts with `--tui`.

**Alternatives:** `select!` both on one task (would starve the engine while the UI blocks); a fully
async input reader; double-fork daemonization (deferred — service managers supervise the process).

**Consequences:** Clean separation; the engine keeps running (and logging into the panel) even if it
errors, so the operator can read the failure before quitting. True detaching daemonization is not
implemented.

## D-22: Box the large TOML error variant in `ConfigError`

**Context:** A stricter `clippy` release began enforcing `result_large_err`; `toml::de::Error` is
large enough that inlining it in `ConfigError` bloats every `Result<_, ConfigError>` and fails
`clippy -D warnings` (the CI gate).

**Decision:** Store the parse error as `Box<toml::de::Error>` in both the `TomlFile` and `Toml`
variants, with a manual `From<toml::de::Error>` that boxes (replacing `#[from]`).

**Alternatives:** `#[allow(clippy::result_large_err)]` (hides the cost); pin an older clippy.

**Consequences:** `Result<_, ConfigError>` shrinks to a pointer-sized error; gate is green. One
manual `From` impl and a `Box::new` at the one construction site.

