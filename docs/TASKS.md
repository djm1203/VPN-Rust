# VPN-Rust - Project Tasks & Milestones

## Timeline Summary

### Completed
- **Phase 1.1**: Project Setup & Core Infrastructure ✅

### Current Focus
- **Phase 1.2**: Code Quality & Testing
- **Phase 2**: Actual VPN Functionality (Routing & Forwarding)

### Future Phases
- **Phase 3**: CLI & Usability
- **Phase 4**: Production Features
- **Phase 5**: Cross-Platform Support

---

## Phase 1: Core Infrastructure

### Milestone 1.1: Project Foundation ✅ COMPLETED

#### Project Setup
- [x] Initialize Cargo project with proper structure - Completed
- [x] Set up Cargo.toml with core dependencies - Completed
- [x] Create module structure (lib.rs, net/, config/) - Completed
- [x] Set up .gitignore for Rust projects - Completed
- [x] Create README.md with project overview - Completed

#### TUN Interface
- [x] Implement TunInterface struct in net/tun.rs - Completed
- [x] Add server TUN creation (rustvpn0, 10.8.0.1/30) - Completed
- [x] Add client TUN creation (rustvpn1, 10.8.0.2/30) - Completed
- [x] Implement async read_packet() method - Completed
- [x] Implement async write_packet() method - Completed
- [x] Add cleanup on Drop (interface deletion) - Completed
- [x] Add MTU configuration (1500) - Completed

#### TLS Implementation
- [x] Implement connect_tls() for client connections - Completed
- [x] Implement start_tls_server() for server - Completed
- [x] Set up certificate loading from files - Completed
- [x] Add webpki-roots for CA verification - Completed
- [x] Create gen_certs.sh for test certificates - Completed

#### Basic Tunnel
- [x] Create server binary with TLS + TUN integration - Completed
- [x] Create client binary with TLS + TUN integration - Completed
- [x] Implement length-prefixed packet protocol - Completed
- [x] Add basic packet echo (server → client) - Completed

#### CI/CD
- [x] Set up GitHub Actions workflow - Completed
- [x] Add cargo build step - Completed
- [x] Add cargo test step - Completed

### Milestone 1.2: Code Quality & Testing (Current)

#### Documentation
- [ ] Add doc comments to all public items in lib.rs
- [ ] Add doc comments to TunInterface methods
- [ ] Add doc comments to TLS functions
- [ ] Add doc comments to config module
- [ ] Create module-level documentation

#### Error Handling Improvements
- [ ] Replace unwrap() calls in server.rs with proper error handling
- [ ] Replace unwrap() calls in client.rs with proper error handling
- [ ] Add context to all error paths with .with_context()
- [ ] Create custom error types if needed

#### Logging
- [ ] Add structured logging throughout codebase
- [ ] Log TUN interface creation/destruction
- [ ] Log TLS connection establishment
- [ ] Log packet send/receive (debug level)
- [ ] Add connection state logging

#### Testing
- [ ] Add unit tests for packet framing
- [ ] Add unit tests for config parsing
- [ ] Add integration test for TLS connection (mocked)
- [ ] Add integration test for packet roundtrip
- [ ] Set up test fixtures

#### Code Cleanup
- [ ] Run cargo clippy and fix warnings
- [ ] Run cargo fmt on entire codebase
- [ ] Remove commented-out code in main.rs
- [ ] Consolidate constants (IPs, ports, MTU)

---

## Phase 2: Actual VPN Functionality

### Milestone 2.1: Bidirectional Traffic

#### Server-Side Improvements
- [ ] Handle multiple packet types (not just echo)
- [ ] Read from TUN and send to client
- [ ] Implement proper bidirectional forwarding
- [ ] Add connection state management

#### Client-Side Improvements
- [ ] Implement bidirectional packet handling
- [ ] Read from TLS and write to TUN
- [ ] Read from TUN and write to TLS concurrently
- [ ] Handle connection drops gracefully

### Milestone 2.2: Traffic Routing

#### Route Management
- [ ] Add route to VPN subnet on client
- [ ] Configure default gateway through tunnel (optional)
- [ ] Implement split tunneling support
- [ ] Add route cleanup on disconnect

#### Server Forwarding
- [ ] Enable IP forwarding on server (sysctl)
- [ ] Implement NAT for outbound traffic
- [ ] Forward packets from TUN to physical interface
- [ ] Forward responses back to client

### Milestone 2.3: DNS Handling

- [ ] Capture DNS queries on client
- [ ] Forward DNS through tunnel
- [ ] Configure DNS servers via DHCP-like protocol
- [ ] Restore original DNS on disconnect

### Milestone 2.4: Connection Resilience

- [ ] Implement keepalive packets
- [ ] Detect connection loss
- [ ] Add automatic reconnection logic
- [ ] Implement exponential backoff for reconnects
- [ ] Handle server restart gracefully

---

## Phase 3: CLI & Usability

### Milestone 3.1: Command Line Interface

#### Argument Parsing
- [ ] Add clap for CLI argument parsing
- [ ] Implement server subcommand (start, stop, status)
- [ ] Implement client subcommand (connect, disconnect, status)
- [ ] Add --config flag for configuration file
- [ ] Add --verbose/-v flag for debug output

#### Configuration Files
- [ ] Design TOML configuration schema
- [ ] Implement config file parsing
- [ ] Support environment variable overrides
- [ ] Add config file validation
- [ ] Create example configuration files

### Milestone 3.2: Terminal UI (Optional)

- [ ] Set up ratatui application structure
- [ ] Create connection status display
- [ ] Add traffic statistics (bytes sent/received)
- [ ] Show connection duration
- [ ] Add log viewer panel

---

## Phase 4: Production Features

### Milestone 4.1: Security Hardening

#### Authentication
- [ ] Implement client certificate authentication
- [ ] Add certificate validation on server
- [ ] Support certificate revocation checking
- [ ] Add pre-shared key option

#### Security Features
- [ ] Add kill switch (block non-VPN traffic)
- [ ] Implement DNS leak prevention
- [ ] Add IPv6 leak prevention
- [ ] Secure memory handling for keys

### Milestone 4.2: Multi-Client Support

- [ ] Design client management system
- [ ] Implement client connection tracking
- [ ] Add IP address assignment (DHCP-like)
- [ ] Support client-to-client routing
- [ ] Add per-client traffic statistics

### Milestone 4.3: Performance Optimization

- [ ] Profile and optimize hot paths
- [ ] Consider multi-threaded runtime
- [ ] Implement connection pooling
- [ ] Add packet batching for throughput
- [ ] Optimize buffer allocations

---

## Phase 5: Cross-Platform Support

### Milestone 5.1: macOS Support

- [ ] Implement utun interface creation
- [ ] Adapt route management for macOS
- [ ] Test TLS/network stack on macOS
- [ ] Update CI to test macOS builds

### Milestone 5.2: Windows Support

- [ ] Integrate Wintun or TAP-Windows
- [ ] Implement Windows-specific TUN handling
- [ ] Adapt route management for Windows
- [ ] Test on Windows
- [ ] Update CI to test Windows builds

---

## Task Backlog (Unprioritized)

### Nice to Have
- [ ] Add compression (LZ4)
- [ ] Implement UDP transport option
- [ ] Add traffic obfuscation
- [ ] Support IPv6 tunneling
- [ ] Create systemd service file
- [ ] Add Prometheus metrics export
- [ ] Implement WireGuard protocol compatibility
- [ ] Add web-based management interface

### Technical Debt
- [ ] Refactor TunInterface for cross-platform
- [ ] Abstract protocol layer for extensibility
- [ ] Improve error messages for users
- [ ] Add configuration validation
- [ ] Create comprehensive test suite

---

## Success Criteria by Phase

### Phase 1 Success ✅ (Partially Achieved)
- [x] TUN interface creation works
- [x] TLS tunnel established
- [x] Packets echo through tunnel
- [ ] Code is well-documented
- [ ] Tests pass with good coverage

### Phase 2 Success
- [ ] Real IP traffic routes through tunnel
- [ ] DNS works through tunnel
- [ ] Server can forward to internet
- [ ] Connection survives interruptions
- [ ] Latency < 50ms overhead

### Phase 3 Success
- [ ] CLI is intuitive and documented
- [ ] Config files work reliably
- [ ] TUI shows useful information
- [ ] Error messages are helpful

### Phase 4 Success
- [ ] Client authentication works
- [ ] Multiple clients supported
- [ ] Kill switch prevents leaks
- [ ] Performance meets targets

### Phase 5 Success
- [ ] Works on Linux, macOS, Windows
- [ ] CI tests all platforms
- [ ] Platform-specific docs exist

---

## Notes

- Mark tasks with [x] and date when completed
- Add discovered tasks to appropriate milestone
- Update success criteria as project evolves
- Keep phase scope realistic for personal project
- Prioritize learning over feature completeness
