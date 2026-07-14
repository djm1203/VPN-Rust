---
title: "API CONTRACT"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: ApiContract
author: Derek Martinez
---

# API Contract — VPN-Rust

## Overview

VPN-Rust has no HTTP/REST surface. Its "API" is the set of interfaces it exposes to
other software and to operators:

1. The **tunnel wire protocol** carried inside the TLS stream (machine-to-machine).
2. The **TLS / mTLS handshake contract** that establishes the tunnel.
3. The **CLI contract** of the `vpn-rust` binary (operator-facing).
4. The **configuration file contract** — TOML (`config.toml`) and the OpenVPN `.ovpn` subset.

This document describes each as it exists in the current build (`vpn-rust` v0.1.0),
plus a clearly-labeled Future section for the planned extended protocol. Every claim
here is grounded in `src/constants.rs`, `src/config/*`, `src/net/tls.rs`, and
`config.example.toml`.

## Compatibility note (read first)

The tunnel wire protocol is **unversioned today**. There is no magic byte, version
field, or type field on the wire — a receiver cannot detect a protocol mismatch and
will misframe. **Client and server must be built from the same commit.** Do not mix
builds. Breaking changes to framing require a coordinated redeploy of both ends until
the versioned extended protocol (below) lands.

---

## 1. Tunnel Wire Protocol

### Transport

- **TLS 1.2 / 1.3 over TCP.** rustls with `with_safe_defaults()` (`src/net/tls.rs`);
  no downgrade path below TLS 1.2.
- **Default port: 4433** (`DEFAULT_SERVER_PORT`). Default bind `127.0.0.1`
  (`DEFAULT_SERVER_ADDR`); production deployments override to `0.0.0.0` via config.
- Once the TLS handshake completes, the byte stream carries framed L3 IP packets in
  both directions. The tunnel is full-duplex and symmetric — client and server use
  the same framing.

### Frame format

Each frame is a 2-byte big-endian length prefix followed by exactly that many bytes
of raw IP packet:

```
 0        1        2                                 2+N
 +--------+--------+---------------------------------+
 |   length (u16)  |        IP packet (N bytes)      |
 |  big-endian BE  |     (raw L3 packet from TUN)    |
 +--------+--------+---------------------------------+
```

- `length` is `u16` big-endian, so a single frame carries at most **65535 bytes**
  (`MAX_PACKET_SIZE`). In practice packets are bounded by the interface MTU of 1500
  (`DEFAULT_MTU`); the read buffer is 1504 bytes (`PACKET_BUFFER_SIZE`, MTU + 4).
- `data` is an unmodified L3 IP packet as read from / written to the TUN device
  (`rustvpn0` server, `rustvpn1` client). No per-packet encryption or MAC is added
  by the application — confidentiality and integrity are provided entirely by the
  TLS layer.
- There is **no type byte, no version byte, and no sequence number** in the current
  frame.

### Keepalive marker

A frame whose length field is **0** is a keepalive, not a data packet:

```
 +--------+--------+
 |  0x00     0x00  |   length == 0  → KEEPALIVE_MARKER, no payload follows
 +--------+--------+
```

- `KEEPALIVE_MARKER = 0` (`src/constants.rs`).
- Keepalives are sent every **10 s** (`KEEPALIVE_INTERVAL_SECS`) when idle.
- A peer that receives no frame (data or keepalive) for **30 s**
  (`CONNECTION_TIMEOUT_SECS`) treats the connection as dead.
- On the client, a dead connection triggers reconnect with exponential backoff from
  **1 s** (`RECONNECT_INITIAL_DELAY_MS`) up to **30 s** (`RECONNECT_MAX_DELAY_MS`).

### Receiver obligations

- Read exactly 2 length bytes, then exactly `length` payload bytes.
- `length == 0` → process as keepalive, read no payload.
- `length > MAX_PACKET_SIZE` is structurally impossible for a `u16`, but receivers
  should still bound the payload read to the buffer size and drop/close on anomaly.

---

## 2. TLS / mTLS Handshake Contract

Implemented in `src/net/tls.rs` (rustls / tokio-rustls).

### Server-authenticated TLS (default)

- Server presents its certificate chain from `certs/server.crt` (`SERVER_CERT_PATH`)
  with key `certs/server.key` (`SERVER_KEY_PATH`). Keys are loaded PKCS#8-first with
  an RSA fallback (`load_private_key`).
- Client verifies the server against a root store. `connect_tls` uses the
  `webpki-roots` bundled trust anchors (`TLS_SERVER_ROOTS`). For self-signed
  development certs, `connect_tls_with_config` with a `ClientTlsConfig` pointing at a
  custom CA (`certs/ca.crt`, `CA_CERT_PATH`) is used instead.
- The client verifies the server hostname via `ServerName::try_from(domain)` — the
  `hostname` config key (default `localhost`) must match the certificate's name.

### Mutual TLS (mTLS, optional)

- Enabled server-side via `ServerTlsConfig::with_mtls()` /
  `start_tls_server_with_config`. The server builds an `AllowAnyAuthenticatedClient`
  verifier from the CA in `certs/ca.crt` and **requires** a client certificate.
- Enabled client-side via `ClientTlsConfig::with_mtls()` — presents
  `certs/client.crt` / `certs/client.key` (`CLIENT_CERT_PATH`, `CLIENT_KEY_PATH`).
- After the handshake, the server may extract the client identity with
  `get_client_cert_cn(&tls_stream)`, which parses the peer X.509 certificate
  (via `x509-parser`) and returns the **Common Name (CN)**. This is the only client
  identity surfaced today; there is no authorization/ACL layer on top of it yet.

### Contract summary

| Property | Value |
|----------|-------|
| Protocol versions | TLS 1.2 and 1.3 (rustls safe defaults) |
| Server auth | Required (cert chain from `certs/server.*`) |
| Client auth | Optional; required when mTLS enabled |
| Trust anchors | `webpki-roots` bundle, or custom CA (`certs/ca.crt`) |
| Client identity | CN extracted from client cert (mTLS only) |
| Cipher suites / groups | rustls `with_safe_defaults()` |

---

## 3. CLI Contract

Unified binary parsed with clap 4.5.

```
vpn-rust <SUBCOMMAND> [OPTIONS]

Subcommands:
  server      Run the VPN server (binds TCP:4433, creates rustvpn0, sets up NAT)
  client      Run the VPN client (connects to server, creates rustvpn1)

Global options:
  --config <path>    Path to a TOML configuration file
  -v, --verbose      Increase log verbosity
```

- **Legacy binaries** `bin/server` and `bin/client` remain and mirror the `server` /
  `client` subcommands respectively.
- **Environment:** `RUST_LOG` controls log level (`trace|debug|info|warn|error`),
  read by `env_logger`.
- **Privileges:** both subcommands create TUN interfaces and require
  root / `CAP_NET_ADMIN`. The server additionally runs `sysctl` and `iptables`
  (IP forwarding + NAT MASQUERADE) — see DEPLOYMENT.md.
- **Platform:** Linux only. The build does not compile on Windows/macOS (uses
  `AsyncFd` / `from_raw_fd`).

### Stability

The subcommand names (`server`, `client`) and the `--config` / `--verbose` flags are
the stable operator contract. Additional flags may be added additively; existing
flags will be deprecated before removal (see SCHEMA_CHANGE_POLICY.md).

---

## 4. Configuration File Contract

### 4a. TOML config (`config.toml`)

Parsed by serde in `src/config/toml_config.rs`. Two optional top-level tables,
`[server]` and `[client]`; either may be absent. Every key has a serde default, so a
partial file (or empty file) is valid.

**`[server]` section**

| Key | Type | Default | Meaning |
|-----|------|---------|---------|
| `bind` | string | `"0.0.0.0"` | Address to bind the listener |
| `port` | u16 | `4433` | TCP listen port |
| `cert` | string | `"certs/server.crt"` | TLS certificate path |
| `key` | string | `"certs/server.key"` | TLS private key path |
| `tun_name` | string | `"rustvpn0"` | Server TUN interface name |
| `subnet` | string | `"10.8.0.0/24"` | VPN subnet (CIDR) |
| `server_ip` | string | `"10.8.0.1"` | Server IP within subnet |
| `enable_nat` | bool | `true` | Enable NAT for client internet access |
| `nat_interface` | string (optional) | unset → auto-detect | Outbound NAT interface |

**`[client]` section**

| Key | Type | Default | Meaning |
|-----|------|---------|---------|
| `server` | string | `"localhost"` | Server address to connect to |
| `port` | u16 | `4433` | Server port |
| `hostname` | string | `"localhost"` | Hostname for TLS cert verification |
| `tun_name` | string | `"rustvpn1"` | Client TUN interface name |
| `client_ip` | string | `"10.8.0.2"` | Client IP within subnet |
| `no_reconnect` | bool | `false` | Disable automatic reconnection |
| `max_reconnects` | u32 | `0` | Max reconnect attempts (0 = unlimited) |

> Note: the default `subnet` in code/`config.example.toml` is `/24`, while the
> point-to-point interface constants (`SERVER_IP_CIDR` / `CLIENT_IP_CIDR`) use
> `10.8.0.0/30`. The `/30` values describe the two-host tunnel link; the `/24`
> subnet is the routable VPN network the server NATs on behalf of. See
> `config.example.toml` for a fully commented reference file.
>
> Load-time **validation is currently deferred** — values are accepted as parsed and
> only fail later at bind / interface / TLS setup.

### 4b. OpenVPN `.ovpn` subset

Parsed by `src/config/ovpn.rs`. This is a deliberately minimal, compatibility-only
parser — **only the `remote` directive is interpreted.**

- `remote <host> [port]` → sets `remote_addr` and `remote_port`. If the port is
  omitted it defaults to **1194** (`DEFAULT_OVPN_PORT`).
- Lines beginning with `#` or `;` are comments; blank lines are skipped.
- **All other directives** (`client`, `dev tun`, `proto tcp`, `<ca>`, etc.) are
  **ignored gracefully** — they are read but not acted upon.
- A file with no `remote` directive is a hard error.

Example (`client.ovpn` in repo):

```
client
dev tun
proto tcp
remote 127.0.0.1 443
```

Only `remote 127.0.0.1 443` is consumed → connect to `127.0.0.1:443`. `dev tun` and
`proto tcp` are informational; certificate inlining is not yet supported.

---

## 5. Future: Extended Wire Protocol (NOT YET IMPLEMENTED)

Planned in `docs/PLANNING.md`. This section documents intent, not current behavior —
**none of it is implemented.** Do not build clients against it.

A versioned frame header is planned to replace the bare length prefix, enabling
protocol negotiation, message typing, and reordering/loss detection:

```
 +---------+---------+------------------+------------------+-----------...
 | version |  type   |  length (u16 BE) | sequence (u32?)  |  payload
 |  (u8)   |  (u8)   |                  |                  |
 +---------+---------+------------------+------------------+-----------...
```

Alongside it, a control-message channel (see `PLANNING.md` "Planned Control
Protocol") would carry `Authenticate`, `Keepalive`/`KeepaliveAck`, `ServerInfo`,
`Disconnect`, and `Error` messages distinct from data frames.

Migration expectation: the first byte becomes a `version` discriminator so a receiver
can distinguish legacy (unversioned) framing from versioned framing during a
transition window. Until this ships, the compatibility note at the top of this
document applies: **matched client/server builds only.**

---

## Change Policy

Changes to any interface in this document follow SCHEMA_CHANGE_POLICY.md: prefer
additive/backward-compatible changes; breaking changes to framing, CLI, or config
keys require a version bump, a CHANGELOG entry, and a migration note.
