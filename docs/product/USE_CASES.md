---
title: "USE CASES"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: UseCases
author: Derek Martinez
---

# Use Cases — VPN-Rust

## Actors

| Actor | Description |
|-------|-------------|
| VPN User | Operator running the client on a trusted Linux host to tunnel traffic. |
| Server Operator | Operator running the VPN server (gateway) on a trusted Linux host. |
| Learner | Developer reading the codebase to understand VPN internals. |
| System | Kernel TUN device, `ip`/`iptables`/`sysctl`, and automated loops (keepalive, reconnect, IP allocation). |

Global precondition for runtime use cases: Linux host with `/dev/net/tun`, root or
`CAP_NET_ADMIN`, and valid PEM certificates under `certs/`.

## Use Case Catalog

### UC-1: Establish an encrypted tunnel between two trusted Linux hosts
- **Actor:** VPN User, Server Operator
- **Precondition:** Server reachable on TCP :4433; matching self-signed certs on both ends.
- **Main flow:**
  1. Operator starts the server (`vpn-rust server`); it creates TUN `rustvpn0`
     (10.8.0.1/30) and a rustls TLS acceptor on :4433.
  2. User starts the client (`vpn-rust client`); it creates TUN `rustvpn1`
     (10.8.0.2/30) and connects via `connect_tls`.
  3. TLS handshake completes; the bidirectional packet loop begins.
- **Postcondition:** An encrypted L3 tunnel is active over the 10.8.0.0/30 subnet.
- **Status:** Implemented.

### UC-2: Route client traffic to the internet via server NAT
- **Actor:** VPN User, System
- **Precondition:** UC-1 established; server allowed to enable IP forwarding/NAT.
- **Main flow:**
  1. Client apps send IP packets; kernel routes them into `rustvpn1`.
  2. Client frames each packet (2-byte length prefix) and writes it to the TLS stream.
  3. Server decodes and writes packets to `rustvpn0`; `net/route.rs` applies NAT / IP
     forwarding via `iptables`/`sysctl` and forwards to the internet.
  4. Return traffic is reverse-NATed, framed, and sent back to the client's TUN.
- **Postcondition:** Client's internet traffic egresses through the server.
- **Status:** Implemented (DNS fully through the tunnel is deferred).

### UC-3: Authenticate clients via mTLS certificates
- **Actor:** Server Operator, VPN User
- **Precondition:** Server configured with a CA and client certs issued; mTLS enabled.
- **Main flow:**
  1. Server requires client certificates (`AllowAnyAuthenticatedClient` against the CA).
  2. Client presents its certificate during the TLS handshake.
  3. Server extracts the client CN (`get_client_cert_cn`) and uses it as the client id.
  4. Unauthenticated clients are rejected.
- **Postcondition:** Only clients with valid CA-signed certs connect; identity = cert CN.
- **Status:** Implemented (self-signed CA; no revocation/CRL).

### UC-4: Monitor the connection via the TUI
- **Actor:** VPN User
- **Precondition:** Client running with the TUI enabled.
- **Main flow:**
  1. User launches the ratatui/crossterm dashboard (`tui/`).
  2. The UI shows connection status, traffic statistics, session duration, and a log panel.
  3. The UI updates as traffic flows and events are logged.
- **Postcondition:** User has a live view of tunnel health and throughput.
- **Status:** Implemented.

### UC-5: Multiple clients connect simultaneously with automatic IP assignment
- **Actor:** Server Operator, VPN Users, System
- **Precondition:** Server subnet wide enough to allocate multiple host IPs.
- **Main flow:**
  1. Each client connects; `ClientManager` registers it and `IpPool` allocates a VPN IP
     (DHCP-like), keyed by cert CN or a generated id.
  2. Server tracks per-client `ClientStats` and an IP→socket map for routing.
  3. On disconnect/idle, the IP is released and the client unregistered.
- **Postcondition:** Multiple clients coexist, each with a distinct VPN IP and stats.
- **Status:** Implemented — but the default `/30` subnet yields only one usable client
  IP; a wider subnet is required for true multi-client operation.

### UC-6: Survive network interruption via keepalive + reconnect
- **Actor:** System, VPN User
- **Precondition:** Active tunnel (UC-1).
- **Main flow:**
  1. Each side sends keepalive frames (length 0) every 10s.
  2. If no traffic/keepalive is seen for 30s, the peer is treated as dead.
  3. The client reconnects with exponential backoff (1s → 30s) until the tunnel is
     re-established.
- **Postcondition:** Transient outages recover without manual intervention.
- **Status:** Implemented.

### UC-7: Kill switch blocks non-VPN traffic on drop
- **Actor:** System, VPN User
- **Precondition:** `SecurityManager` kill switch enabled.
- **Main flow:**
  1. On tunnel drop, `net/security.rs` installs iptables rules blocking traffic outside
     the VPN path.
  2. DNS-leak and IPv6-leak prevention rules keep DNS/IPv6 from bypassing the tunnel.
  3. Rules are lifted on clean shutdown / reconnect.
- **Postcondition:** No plaintext traffic leaks while the VPN is down.
- **Status:** Implemented.

### UC-8: Learning / reference reading of the codebase
- **Actor:** Learner
- **Precondition:** Repository cloned.
- **Main flow:**
  1. Learner reads `docs/` and the well-commented `src/` modules (config, net, tui, cli).
  2. Learner traces the packet path: TUN → framing → TLS → routing/NAT.
- **Postcondition:** Learner understands TUN, TLS tunneling, and async Rust networking.
- **Status:** Implemented (documentation-supported).

## Technical Context

- **Primary language:** Rust (2021 edition), ~5,195 LOC.
- **Build system:** Cargo (`Cargo.toml` / `Cargo.lock`).
- **Runtime target:** Linux only; TUN requires root / `CAP_NET_ADMIN`.

## Traceability

| Use Case | Requirement | Capability | Status |
|----------|-------------|------------|--------|
| UC-1 | Phase 1 secure tunnel | C-001, C-002 | Implemented |
| UC-2 | Phase 2 routing/NAT | C-004 | Implemented |
| UC-3 | Phase 4 mTLS auth | C-006 | Implemented |
| UC-4 | Phase 3 TUI monitoring | C-011 | Implemented |
| UC-5 | Phase 4 multi-client | C-007, C-008 | Implemented (subnet-limited) |
| UC-6 | Phase 2 reliability | C-005 | Implemented |
| UC-7 | Phase 4 kill switch | C-009 | Implemented |
| UC-8 | Educational goal | (docs) | Implemented |
