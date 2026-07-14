---
title: "DATA GOVERNANCE"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: DataGovernance
author: Derek Martinez
---

# Data Governance — VPN-Rust

> **Scope note:** As a personal/educational VPN, VPN-Rust performs no third-party
> data processing and claims no regulatory scope. It still tunnels network
> traffic and handles key material, so the guiding principle is
> **data minimization and never logging payloads or keys**.

## Data Classification

| Level | Description | Handling in VPN-Rust |
|-------|-------------|----------------------|
| Secret | Private keys | `certs/*.key` on disk, `chmod 600`, never committed, not logged |
| Internal / Personal | Config, traffic stats, logs | Local to the operator's host; not shared |
| Transient | Tunneled IP packets | In memory only during forwarding; not persisted |

## What Data the System Touches

| Data | Description | Location | Persistence | Sensitivity |
|------|-------------|----------|-------------|-------------|
| Tunneled IP packets | Raw L3 IP packets carried through the TLS tunnel | In memory during forward | None — transient | Transient; not inspected beyond length-prefix framing |
| Config | Server address/port, cert paths, subnet/CIDR | Config file / args | On disk (operator) | No user PII |
| Certificates & keys | Self-signed server/CA/client certs and private keys | `certs/` | On disk | **Keys = Secret**; certs public |
| Per-client traffic stats | `bytes_sent`/`received`, `packets_*`, duration, idle time, assigned VPN IP, cert CN | `ClientManager` in memory | None — lost on restart | Internal/personal (contains client IP + identity CN) |
| Logs | Connection events, counters, IPs/packet metadata (debug), payloads (trace only) | `env_logger` → stderr | Not persisted by default | Internal; can expose metadata at debug/trace |

## Data Lifecycle

1. **Collection** — Only what the tunnel needs: packets to forward, minimal
   config, and per-client counters for observability. No accounts, no profiles.
2. **Storage** — Nothing is persisted by default. Keys/certs and config live on
   the operator's disk. Stats and packet buffers are memory-only.
3. **Processing** — Packets are framed (2-byte length prefix) and relayed; they
   are not deep-inspected. Least-privilege still requires root for TUN/iptables
   (a known coupling, not an application data-access grant).
4. **Retention** — None by default: no packet capture, no stats database, no log
   files written by the app itself. Process exit discards in-memory state.
5. **Deletion** — Rotating certs deletes/replaces key material; there is no
   application data store to purge. Key files should be securely removed when
   retired.

## Sensitive Data Handling

- **Private keys** are the crown jewels: plaintext PEM on disk, protected by file
  permissions (`chmod 600`), never committed (`.gitignore` keeps `.key`
  untracked), never logged. In-memory zeroize is **deferred** (a known gap).
- **Packet payloads** must never be logged in real use — only `trace` exposes
  them (R-8.2). Debug logs may reveal IP addresses and packet metadata; keep
  debug/trace off outside development.
- **Client identity** (certificate CN) and assigned VPN IP are retained only in
  memory for the life of the connection.

## Access Control

- All data is local to the host running the client or server; there is no
  multi-tenant data store and no service-account model.
- OS-level: TUN and network configuration require root/`CAP_NET_ADMIN`, so
  effective access to tunnel data equals host root.

## Retention Policy

| Data Type | Retention | Deletion Method |
|-----------|-----------|-----------------|
| Tunneled packets | None (transient) | Discarded after forwarding |
| Per-client stats | Process lifetime only | Dropped on disconnect/restart |
| Logs | Not persisted by app | N/A (terminal output) |
| Certificates/keys | Until rotated | Regenerate + securely delete old key files |
| Config | Operator-managed | Operator deletes |
