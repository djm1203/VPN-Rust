---
title: "MONITORING"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Monitoring
author: Derek Martinez
---

# Monitoring — VPN-Rust

> **Scope note:** This is a learning-focused, single-maintainer VPN. Monitoring
> today is logs + a terminal UI + manual inspection. There is no metrics
> pipeline, alerting, or on-call rotation. This document describes what exists
> and what to watch, and flags the gaps honestly.

## Observability Today

### Structured logging (`log` + `env_logger`)

The codebase logs through the `log` facade, rendered by `env_logger`. Verbosity
is controlled at runtime with the `RUST_LOG` environment variable
(e.g. `RUST_LOG=info`, `RUST_LOG=debug`, `RUST_LOG=vpn_rust=trace`).

| Level | What it captures |
|-------|------------------|
| `error` | Failures needing attention (bind failure, handshake failure, iptables failure) |
| `warn`  | Recoverable issues (rule "may already exist", idle client removal, cleanup-on-drop failures) |
| `info`  | Operational events: TLS server listening, TLS connection established, kill switch / DNS / IPv6 features enabled/disabled, client register/unregister, NAT configured |
| `debug` | Diagnostics: cert/key load counts, IP pool allocation/release, rule application details |
| `trace` | Very detailed — including packet contents |

**Security constraint (R-8.2):** packet payloads only appear at `trace`. Never
run `trace` in a real deployment; keys/payloads must not be logged in production.

Representative events already emitted:
- TUN interface create/destroy (via the TUN module) and MTU/address setup.
- TLS handshake / connection state ("TLS connection established to …",
  "TLS server listening on …", "Client certificate authentication enabled").
- Packet send/receive accounting at `debug`/`trace`.
- Per-client lifecycle: "Registered client … with VPN IP …", and on disconnect
  "Unregistered client …: sent N bytes, received M bytes".

### Terminal dashboard (ratatui)

A `ratatui`-based TUI surfaces live state: connection status, bytes sent /
received, connection duration, and a scrolling log panel — useful for watching a
single session interactively.

### Per-client statistics (`ClientManager`)

`src/net/clients.rs` maintains atomic counters per client
(`ClientStats` → `ClientStatsSnapshot`): `bytes_sent`, `bytes_received`,
`packets_sent`, `packets_received`, plus connection duration and idle time.
- `get_all_stats()` returns a snapshot for every connected client.
- `client_count()` gives the current number of connected clients.
- These live **in memory only** and are lost on restart (no persistence).

## Manual Monitoring (operator toolbox)

Because there is no exporter yet, use standard Linux tooling on the host:

| Goal | Command |
|------|---------|
| Watch tunnel traffic | `sudo tcpdump -i rustvpn0` (server) / `rustvpn1` (client) |
| Inspect interface + assigned IP | `ip addr show rustvpn0` |
| Verify routes | `ip route show` |
| Verify NAT / kill-switch / DNS / IPv6 rules | `sudo iptables -L -v` / `sudo iptables -t nat -L` / `sudo ip6tables -L` |
| Confirm IP forwarding | `sysctl net.ipv4.ip_forward` |
| Tail app logs at a chosen level | `RUST_LOG=debug sudo cargo run --bin server` |

## What to Watch (recommended signals)

- **Connection drops / reconnect churn** — repeated register/unregister for the
  same identity suggests an unstable link.
- **Keepalive / idle timeouts** — clients reaped by `remove_idle_clients`
  indicate lost liveness; correlate with client-side reconnect logs.
- **Certificate errors** — "TLS handshake … failed", "Failed to parse client
  certificate", "CA certificate required for client authentication". These often
  mean expired/self-signed cert mismatch — regenerate via `./gen_certs.sh`.
- **IP pool exhaustion** — "IP pool exhausted" means the subnet is too small for
  the client count; widen the CIDR.
- **iptables/sysctl failures** — any `error` from `security.rs`/`route.rs`
  usually means missing root/CAP_NET_ADMIN and that leak-prevention is **not**
  actually in effect.

## Gaps (honest)

- **No metrics export.** Prometheus/`/metrics` is a backlog item, not built.
- **No alerting** and **no on-call rotation** — single maintainer.
- **No health-check endpoint** or readiness/liveness probe.
- **No log aggregation** or retention policy; logs go to the terminal.
- Stats are **not persisted** and reset on process restart.
