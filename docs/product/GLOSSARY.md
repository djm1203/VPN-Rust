---
title: "GLOSSARY"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Glossary
author: Derek Martinez
---

# Glossary — VPN-Rust

| Term | Definition |
|------|------------|
| TUN | A virtual Layer 3 (IP) network interface provided by the kernel. VPN-Rust uses TUN devices (`rustvpn0`, `rustvpn1`) to capture and inject raw IP packets. |
| TAP | A virtual Layer 2 (Ethernet frame) network interface. Not used by VPN-Rust — TUN (L3) is sufficient for IP tunneling. |
| TLS | Transport Layer Security; encrypts and authenticates the tunnel transport. VPN-Rust runs TLS over TCP :4433. |
| mTLS | Mutual TLS; both server and client present certificates. Used for client authentication, with identity taken from the client cert's CN. |
| rustls | Pure-Rust TLS library (v0.21) used for TLS, avoiding an OpenSSL dependency; integrated with Tokio via tokio-rustls. |
| tokio | Async runtime (v1.38, `full` features) providing the executor, timers, TCP I/O, and `AsyncFd`. |
| AsyncFd | Tokio wrapper that makes a raw file descriptor (the TUN device) usable with async I/O. Unix/Linux-only — the reason VPN-Rust is Linux-only. |
| CIDR (10.8.0.0/30) | Classless Inter-Domain Routing notation for the VPN subnet. `/30` gives 4 addresses; here 10.8.0.1 (server) and 10.8.0.2 (client) are the usable hosts. |
| MTU | Maximum Transmission Unit — largest packet size on the interface. Set to 1500; the read buffer is 1504 bytes (MTU + TUN header). |
| NAT | Network Address Translation; the server rewrites source addresses so client traffic can reach the internet and return. Configured via `iptables`. |
| IP forwarding | Kernel setting (enabled via `sysctl`) that lets the server route packets between the TUN interface and its internet-facing interface. |
| Kill switch | Security feature (`SecurityManager`) that blocks non-VPN traffic when the tunnel drops, preventing plaintext leaks. |
| DNS leak | When DNS queries bypass the VPN and reveal browsing to the local network/ISP. VPN-Rust adds iptables rules to prevent this. |
| IPv6 leak | When IPv6 traffic escapes a VPN that only tunnels IPv4. VPN-Rust blocks IPv6 leakage via iptables. |
| Keepalive | Periodic small frame (every 10s) that keeps the tunnel alive and detects a dead peer (30s timeout). |
| Exponential backoff | Reconnect strategy where the retry delay grows from 1s up to 30s, avoiding tight reconnect loops after a drop. |
| Length-prefixed protocol | Wire format: a 2-byte big-endian u16 length followed by that many bytes of raw IP packet. |
| Keepalive marker | A length field equal to 0, signaling a keepalive frame with no packet payload. |
| TUI | Terminal User Interface (ratatui + crossterm) showing connection status, traffic stats, duration, and logs. |
| TCP tunnel | The tunnel transport: a single TCP connection (port 4433) carrying TLS-encrypted, length-framed packets. |
| CAP_NET_ADMIN | Linux capability permitting network-administration operations (creating TUN devices, configuring routes) without full root. |
| webpki-roots | Crate providing baked-in root CA trust anchors used for standard server-certificate verification. |
| X.509 / PEM | X.509 is the certificate standard; PEM is the base64 text encoding used for the cert and key files under `certs/`. |
| IpPool | Server-side DHCP-like pool (`net/clients.rs`) that allocates VPN IP addresses to clients from the subnet CIDR, excluding network/gateway/broadcast. |
| Split tunneling | Routing only some traffic through the VPN while the rest uses the normal connection. Referenced as a future consideration; not implemented. |
| OpenVPN .ovpn | OpenVPN's client configuration file format. VPN-Rust can parse `.ovpn` files (`OVPNConfig`) for compatibility/config input. |
| ClientManager | Server component tracking connected clients, their allocated IPs, per-client stats, and IP→socket routing map. |
| SecurityManager | Component implementing the kill switch and DNS/IPv6 leak prevention via iptables. |
