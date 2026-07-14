---
title: "PERFORMANCE BUDGET"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: PerformanceBudget
author: Derek Martinez
---

# Performance Budget — VPN-Rust

> **GAP — no benchmarks exist yet.** Every number in this document is a **TARGET**
> taken from `docs/PLANNING.md`. Nothing here has been measured, profiled, or
> validated. Treat these as design goals to test against, not as observed behavior.
> The measurement plan at the end describes how these will eventually be verified.

## Latency Targets — TARGET (not yet benchmarked)

| Operation | Target | Notes |
|-----------|--------|-------|
| TUN read/write | < 1 ms | Kernel operation per packet |
| Packet framing | < 0.1 ms | Add/strip 2-byte length prefix (memory copy) |
| TLS encrypt/decrypt | < 1 ms | Per packet, rustls |
| Total tunnel overhead | < 5 ms | End-to-end, added latency vs. direct path |

## Throughput Targets — TARGET (not yet benchmarked)

| Metric | Target | Notes |
|--------|--------|-------|
| Single-connection throughput | > 100 Mbps | Over LAN |
| Packets per second | > 10,000 pps | Small packets |

## Memory Targets — TARGET (not yet benchmarked)

| Role | Target | Notes |
|------|--------|-------|
| Client | < 50 MB | Resident set, single tunnel |
| Server | < 100 MB | With 10 concurrent clients |

## CPU Targets — TARGET (not yet benchmarked)

| Condition | Target |
|-----------|--------|
| Idle | < 5% |
| Under load | < 30% |

## Resource Limits & Constants (authoritative — from `src/constants.rs`)

Unlike the targets above, these are hard values compiled into the binary:

| Constant | Value | Meaning |
|----------|-------|---------|
| `MAX_PACKET_SIZE` | 65535 bytes | Max framed payload (u16 length ceiling) |
| `DEFAULT_MTU` | 1500 | TUN interface MTU (practical per-packet bound) |
| `PACKET_BUFFER_SIZE` | 1504 bytes | Read buffer (MTU + 4) |
| `KEEPALIVE_INTERVAL_SECS` | 10 s | Idle keepalive cadence |
| `CONNECTION_TIMEOUT_SECS` | 30 s | Dead-peer detection window |
| `RECONNECT_INITIAL_DELAY_MS` | 1000 ms | Reconnect backoff floor |
| `RECONNECT_MAX_DELAY_MS` | 30000 ms | Reconnect backoff ceiling |
| `INTERFACE_CLEANUP_DELAY_MS` | 200 ms | Settle delay after interface teardown |

## Concurrency Model & Known Ceiling

The current runtime is a **single-threaded tokio runtime** (a deliberate decision in
`PLANNING.md` for the learning/personal scope). This is the most likely throughput
ceiling: all TUN I/O, framing, and TLS work for every connection share **one core**.
The 100 Mbps / 10k pps targets have not been shown reachable under this model and may
require a multi-threaded runtime and/or per-connection tasks to hit. Revisit the
runtime choice before promising higher throughput.

Additional structural factors to keep in mind when measuring:

- TLS record encryption/decryption is on the hot path for every packet.
- TCP-over-TLS transport can incur head-of-line blocking under loss (a reason UDP
  transport is an open question in `PLANNING.md`).
- No packet batching or compression today — one IP packet per frame.

## Measurement Plan (how to validate — not yet done)

1. **Throughput:** run `iperf3` server behind the tunnel and `iperf3` client on the
   peer; measure TCP and UDP throughput through `rustvpn0`/`rustvpn1`. Compare
   against a direct (non-tunneled) baseline on the same link.
2. **Latency overhead:** `ping 10.8.0.1` through the tunnel vs. `ping` of the
   underlay address; the delta approximates tunnel overhead. Confirm against the
   < 5 ms target.
3. **Packets per second:** small-packet flood (e.g. `iperf3 -u` with small datagram
   size, or a packet generator) while watching drop counts.
4. **Memory:** `/usr/bin/time -v ./target/release/vpn-rust server ...` and the client
   equivalent; record maximum resident set. Load the server with ~10 clients for the
   server figure.
5. **CPU:** sample idle vs. loaded CPU with `top`/`pidstat` during the iperf3 runs.
6. **Profiling (future):** `perf` for CPU hotspots and, once instrumented,
   `tokio-console` to inspect task scheduling and stalls on the single-threaded
   runtime.

Record results in `docs/state/CHANGELOG.md` and update this document's tables with
measured columns once data exists — at which point each row should show
**target vs. measured**.

## Optimization Guidelines

- Profile before optimizing — measure, don't guess. No numbers exist yet, so **the
  first task is measurement, not optimization.**
- Document any performance trade-off (e.g. switching runtimes, adding batching) in
  `docs/state/DECISIONS.md`.
- Add regression checks for performance-critical paths once a baseline is captured.
