---
title: "SECURITY MODEL"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: SecurityModel
author: Derek Martinez
---

# Security Model — VPN-Rust

> **Scope note:** VPN-Rust is a learning-focused VPN client and server. This
> document describes the security features that are actually implemented today
> and is deliberately honest about the gaps. It is **not** a claim that the
> software is production-hardened. See *Known Gaps* below.

## Authentication

Authentication is built on TLS and, optionally, mutual TLS (mTLS). Implemented
in `src/net/tls.rs`.

- **Server authentication (always).** The server presents its certificate
  (`certs/server.crt` + `certs/server.key`, loaded via `start_tls_server` /
  `start_tls_server_with_config`). Clients verify the server against a trust
  store built from `webpki-roots` (`TLS_SERVER_ROOTS`) and/or a custom CA
  (`certs/ca.crt`). For the self-signed setup used in this project, the client
  supplies the project CA via `ClientTlsConfig::with_mtls()` /
  `ClientTlsConfig::custom(...)` and disables system roots.
- **Client authentication (optional, mTLS).** When enabled
  (`ServerTlsConfig::with_mtls()` sets `require_client_auth = true`), the server
  installs an `AllowAnyAuthenticatedClient` verifier backed by the CA in
  `certs/ca.crt` and requires each client to present `certs/client.crt` +
  `certs/client.key`. Clients without a valid, CA-signed certificate are
  rejected at the TLS handshake.
- **Client identity.** `get_client_cert_cn()` parses the peer certificate with
  `x509-parser` and extracts the Common Name (CN). That CN is used as the client
  identity/`client_id` in `ClientManager::register_client` (see
  `src/net/clients.rs`), which records `authenticated` and `cert_cn` on each
  `ClientConnection`.

> **Honest limitation:** `AllowAnyAuthenticatedClient` accepts *any* certificate
> that chains to the trusted CA. There is no per-CN allowlist, no certificate
> revocation check (no CRL/OCSP), and validity rests entirely on the CA being
> trusted. In the default flow the server is also constructed with
> `with_no_client_auth()` (`start_tls_server`) unless the mTLS variant is
> explicitly selected.

## Authorization

Authorization is coarse-grained: connection == authorization.

- Any client that completes the TLS (or mTLS) handshake is registered by
  `ClientManager` and allocated a VPN IP from the `IpPool` (DHCP-like allocation
  from the configured CIDR, e.g. `10.8.0.0/24`).
- There is **no finer-grained RBAC**, no per-client ACLs, and no policy on which
  destinations a client may reach beyond what NAT/forwarding permits. Optional
  client-to-client routing is a single boolean (`allow_client_routing`).
- Idle clients can be reaped (`remove_idle_clients`), releasing their IP back to
  the pool.

## Data Protection

- **In transit:** All tunnel traffic is carried inside a TLS 1.2/1.3 session
  (`rustls 0.21` with `with_safe_defaults()`, over `tokio-rustls` on TCP:4433).
  `rustls` was chosen for memory safety and to avoid a system OpenSSL
  dependency.
- **Tunneled payload:** Raw IP packets are transient and held only in memory
  during forwarding; they are not persisted.
- **No secrets in source code (R-8.1):** Certificate/key material lives on disk
  under `certs/`, referenced by path constants — not embedded in code.
- **No secrets in logs (R-8.2):** Operational logs record connection state and
  counters. Packet contents are only ever emitted at `trace` level and must not
  be enabled in any real deployment (see *Known Gaps*).
- **No secrets in version control (R-8.3):** `.gitignore` excludes session/local
  state; private keys must never be committed. Only public `.crt` files may be
  tracked; `.key` files must stay untracked.
- **At rest:** Private keys are stored as plaintext PEM files on disk. Protection
  relies on filesystem permissions (operator must `chmod 600` the key files).
  In-memory key zeroization is **deferred** (not implemented).

## Secret Management

- Certificates are **self-signed**, generated locally. `./gen_certs.sh` currently
  produces the server key/cert pair (`certs/server.key`, `certs/server.crt`) with
  CN=`localhost`, valid 365 days. The mTLS flow additionally expects a CA
  (`certs/ca.crt`) and a client key/cert pair (`certs/client.crt`,
  `certs/client.key`); those must be provisioned by the operator/CA process
  (the checked-in script does not yet emit them).
- Key files must be readable only by their owner (`chmod 600`) and must never be
  committed.
- There is no secret vault, no automated rotation, and no zeroize-on-drop for
  key bytes. Rotation = regenerate certs and redeploy.

## Threat Model & Assumptions

- **Intended environment:** personal / LAN / educational use with **trusted
  endpoints**. The operator controls both client and server and the CA.
- **Assumed adversary:** a passive network observer on the path between client
  and server. TLS protects confidentiality/integrity of the tunnel against this
  adversary.
- **NOT defended against:** a sophisticated or active attacker, a compromised
  endpoint, a hostile client presenting a validly-CA-signed but malicious cert,
  key theft from disk, denial-of-service (no rate limiting), or traffic-analysis.
- **No security audit** of this codebase has been performed.

## Network Security Features

Implemented in `src/net/security.rs` (client-side) and `src/net/route.rs`
(server-side). All are Linux-only and manipulate `iptables`/`ip6tables`/`ip`/
`sysctl`, so they require **root or CAP_NET_ADMIN**.

- **Kill switch** (`KillSwitch`): backs up existing `iptables` rules, installs a
  `VPN_KILLSWITCH` chain in `OUTPUT` that permits loopback, established
  connections, traffic to the VPN server IP/port, traffic out the TUN interface,
  and DHCP — then DROPs everything else. Restores the backup on disable/drop.
- **DNS leak prevention** (`DnsLeakPrevention`): rewrites `/etc/resolv.conf` to
  the VPN DNS server and installs a `VPN_DNS` chain that only allows port-53
  traffic out the TUN interface (and to localhost), dropping other DNS. This is
  **blocking-based leak prevention, not DNS-over-tunnel** — DNS is not tunneled
  as an application protocol.
- **IPv6 leak prevention** (`Ipv6LeakPrevention`): installs a `VPN_IPV6_BLOCK`
  chain that drops all IPv6 egress except loopback (blunt but effective for a
  v4-only tunnel).
- **NAT / forwarding** (server, `route.rs`): enables `net.ipv4.ip_forward`, adds
  `MASQUERADE` in `POSTROUTING` for the VPN subnet, and FORWARD accept rules so
  clients can reach the internet through the server.
- All three client features are aggregated by `SecurityManager`
  (`enable_all` / `disable_all`) and clean up on `Drop`.

## Known Gaps (be honest)

- **Self-signed certificates only**; trust rests on manually distributed CA.
- **No certificate revocation** (no CRL/OCSP) — a compromised client cert cannot
  be revoked short of rotating the CA.
- **No per-identity authorization** beyond "authenticated == allowed".
- **No key zeroization**; keys sit in memory and on disk as plaintext PEM.
- **DNS is not tunneled**; leak prevention is iptables blocking, which fails
  open if the rules are not applied (needs root; errors are logged, not fatal).
- **Single-threaded tokio runtime**; no rate limiting / DoS protection.
- **No security audit**, minimal test coverage on the security paths.
- **Linux-only**; the build does not currently succeed on Windows.
- Requires **root / CAP_NET_ADMIN** for all network manipulation and TUN setup.
