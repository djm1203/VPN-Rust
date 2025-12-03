# VPN-Rust - Product Requirements Document

## Vision

A **learning-focused VPN implementation** in Rust that demonstrates systems-level networking concepts. The project serves as both a functional VPN client/server and an educational resource for understanding secure tunneling, TUN/TAP devices, and async network programming in Rust.

## Problem Statement

### Learning Challenge
Understanding VPN internals requires hands-on implementation experience. Existing VPN solutions (OpenVPN, WireGuard) are production-optimized but difficult to learn from due to complexity and legacy code.

### Technical Knowledge Gaps
- How TUN/TAP virtual network interfaces work
- How packets are encapsulated and tunneled
- How TLS secures network communication
- How async I/O handles network operations efficiently
- How VPN routing and IP assignment function

### Goals
1. **Educational**: Create a readable, well-documented VPN implementation
2. **Functional**: Build a working VPN that can tunnel traffic
3. **Modern**: Use current Rust patterns and async best practices
4. **Extensible**: Design for future protocol and platform additions

## Core Users

### Primary: The Developer (Self)
A developer learning systems programming who wants to understand VPN internals by building one. Needs clear code, good documentation, and incremental complexity.

### Secondary: Other Learners
Developers interested in network programming who might reference this codebase for learning. Need readable code with explanatory comments and documentation.

### Tertiary: Personal Use
Using the VPN for personal traffic tunneling between trusted machines. Need reasonable security and reliability.

## User Stories

### Phase 1: Core Infrastructure (Current)
- As a developer, I want to create TUN interfaces programmatically so I can inject and capture packets
- As a developer, I want to establish TLS connections so traffic is encrypted
- As a developer, I want a simple packet echo system so I can verify the tunnel works
- As a developer, I want logging so I can debug packet flow

### Phase 2: Actual VPN Functionality
- As a user, I want the VPN to route my traffic through the tunnel so I can access remote networks
- As a user, I want the server to forward packets to the internet so the VPN is functional
- As a user, I want DNS queries to go through the tunnel so my browsing is private
- As a user, I want automatic reconnection so temporary network issues don't require manual intervention

### Phase 3: Usability
- As a user, I want a CLI interface so I can easily start/stop the VPN
- As a user, I want configuration files so I don't need to pass many arguments
- As a user, I want a TUI dashboard so I can monitor connection status
- As a user, I want the client to run without root so I don't need elevated privileges for daily use

### Phase 4: Production Features
- As a user, I want certificate-based authentication so only authorized clients can connect
- As a user, I want multiple client support so I can connect from multiple devices
- As a user, I want traffic statistics so I can monitor bandwidth usage
- As a user, I want kill switch functionality so traffic doesn't leak if the VPN disconnects

## Success Criteria

### Phase 1 Targets (Current)
- [x] TUN interface creation works on Linux
- [x] TLS tunnel established between client and server
- [x] Packets successfully echo through the tunnel
- [ ] Clear logging shows packet flow
- [ ] Code is well-documented and readable

### Phase 2 Targets
- [ ] Actual IP traffic routes through tunnel
- [ ] Server performs NAT/forwarding to internet
- [ ] DNS resolution works through tunnel
- [ ] Connection survives brief network interruptions
- [ ] Latency overhead < 50ms for typical operations

### Phase 3 Targets
- [ ] CLI with subcommands (connect, disconnect, status)
- [ ] Configuration file support (.toml or .yaml)
- [ ] TUI shows connection status and statistics
- [ ] Non-root operation via capabilities or helper daemon

### Phase 4 Targets
- [ ] Mutual TLS authentication (client certificates)
- [ ] Support 5+ simultaneous clients
- [ ] Real-time bandwidth monitoring
- [ ] Kill switch prevents traffic leaks
- [ ] Cross-platform support (Linux, macOS, Windows)

## Core Capabilities

### Network Interface Management
Create and manage TUN virtual network interfaces for packet capture and injection. Handle IP assignment, MTU configuration, and interface lifecycle.

### Secure Communication
Establish TLS-encrypted tunnels using modern cryptography (rustls). Support certificate validation and secure key exchange.

### Packet Tunneling
Encapsulate IP packets for transmission over the TLS tunnel. Handle fragmentation, reassembly, and protocol overhead.

### Traffic Routing (Planned)
Configure system routing tables to direct traffic through the VPN tunnel. Implement split tunneling for selective routing.

### CLI/TUI Interface (Planned)
Provide command-line interface for VPN control. Optional terminal UI for status monitoring and configuration.

## Non-Functional Requirements

### Performance
- Tunnel throughput: > 100 Mbps on local network
- Latency overhead: < 20ms for packet encapsulation
- Memory usage: < 50 MB for client, < 100 MB for server
- CPU usage: < 5% idle, < 30% under load

### Security
- TLS 1.2+ for all communications
- No plaintext secrets in logs or config files
- Certificate validation in production mode
- Secure random number generation

### Reliability
- Graceful handling of network interruptions
- Clean shutdown without resource leaks
- Automatic interface cleanup on crash
- Reconnection attempts with backoff

### Maintainability
- Modular code architecture
- Comprehensive documentation
- Unit and integration tests
- CI/CD with automated testing

### Portability
- Primary: Linux (full support)
- Secondary: macOS (planned)
- Tertiary: Windows (prepared, not implemented)

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        VPN Client                           │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    │
│  │ TUN Device  │───▶│   Tunnel    │───▶│ TLS Client  │────┼──▶ Internet
│  │ (rustvpn1)  │◀───│   Manager   │◀───│             │◀───┼──
│  └─────────────┘    └─────────────┘    └─────────────┘    │
│       │                                                     │
│       ▼                                                     │
│  Local Apps                                                 │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ TLS Tunnel (Port 4433)
                              │
┌─────────────────────────────────────────────────────────────┐
│                        VPN Server                           │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    │
│  │ TLS Server  │───▶│   Tunnel    │───▶│ TUN Device  │────┼──▶ Forward
│  │             │◀───│   Manager   │◀───│ (rustvpn0)  │◀───┼──  to Net
│  └─────────────┘    └─────────────┘    └─────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## Protocol Design

### Current: Simple Echo Protocol
```
┌────────────────────────────────────┐
│  2 bytes   │    N bytes           │
│  Length    │    Packet Data       │
│  (BE u16)  │    (IP packet)       │
└────────────────────────────────────┘
```

### Future: Extended Protocol
```
┌────────────────────────────────────────────────────────┐
│ 1 byte │ 1 byte │ 2 bytes │ 4 bytes  │    N bytes    │
│ Version│  Type  │ Length  │ Sequence │  Packet Data  │
└────────────────────────────────────────────────────────┘

Types:
- 0x00: Data packet
- 0x01: Keepalive
- 0x02: Control message
- 0x03: Error
```

## Constraints

### Technical Constraints
- Linux TUN/TAP requires root or CAP_NET_ADMIN
- Rust async model requires careful resource management
- Self-signed certificates limit production use
- Single-threaded tokio runtime limits concurrency

### Scope Constraints
- Personal/learning project, not production VPN
- Single developer, limited time
- Focus on learning over features
- Linux-first development

### Security Constraints
- No formal security audit
- Self-signed certificates for development
- No protection against sophisticated attacks
- Trust model assumes trusted endpoints

## Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Security vulnerabilities | High | Medium | Use vetted crates (rustls), follow best practices |
| Platform compatibility issues | Medium | High | Abstract platform-specific code, test early |
| Performance bottlenecks | Medium | Medium | Profile early, use async I/O throughout |
| Scope creep | Low | High | Stick to phased approach, document decisions |
| Complexity overwhelm | Medium | Medium | Keep modules small, document as you go |

## Success Definition

### Minimum Viable Product (MVP)
A working VPN that can:
1. Create TUN interfaces on client and server
2. Establish TLS-encrypted tunnel
3. Route IP traffic through the tunnel
4. Handle basic DNS resolution

### Learning Success
Understanding achieved in:
1. TUN/TAP device programming
2. TLS implementation details
3. Async Rust networking patterns
4. VPN protocol design

### Project Success
Codebase that:
1. Others can learn from
2. Works reliably for personal use
3. Can be extended with new features
4. Demonstrates modern Rust practices

## Future Considerations

### Potential Enhancements
- WireGuard protocol compatibility
- Multi-hop routing
- Traffic obfuscation
- Mobile client support
- Web-based management interface

### Technology Evolution
- Monitor async Rust ecosystem changes
- Consider io-uring for Linux performance
- Evaluate QUIC as alternative transport
- Watch for cross-platform TUN libraries

## References

- [Linux TUN/TAP Documentation](https://www.kernel.org/doc/Documentation/networking/tuntap.txt)
- [rustls Documentation](https://docs.rs/rustls)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [OpenVPN Protocol](https://openvpn.net/community-resources/)
- [WireGuard Whitepaper](https://www.wireguard.com/papers/wireguard.pdf)
