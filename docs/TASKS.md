# VPN-Rust - Project Tasks & Milestones

## Timeline Summary

### Completed
- **Phase 1.1**: Project Setup & Core Infrastructure ✅
- **Phase 1.2**: Code Quality & Testing ✅ (2024-12-03)

### Current Focus
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

### Milestone 1.2: Code Quality & Testing ✅ COMPLETED 2024-12-03

#### Documentation
- [x] Add doc comments to all public items in lib.rs - 2024-12-03
- [x] Add doc comments to TunInterface methods - 2024-12-03
- [x] Add doc comments to TLS functions - 2024-12-03
- [x] Add doc comments to config module - 2024-12-03
- [x] Create module-level documentation - 2024-12-03

#### Error Handling Improvements
- [x] Replace unwrap() calls in server.rs with proper error handling - 2024-12-03
- [x] Replace unwrap() calls in client.rs with proper error handling - 2024-12-03
- [x] Add context to all error paths with .with_context() - 2024-12-03
- [ ] Create custom error types if needed (deferred - anyhow sufficient for now)

#### Logging
- [x] Add structured logging throughout codebase - 2024-12-03
- [x] Log TUN interface creation/destruction - 2024-12-03
- [x] Log TLS connection establishment - 2024-12-03
- [x] Log packet send/receive (debug/trace level) - 2024-12-03
- [x] Add connection state logging - 2024-12-03

#### Testing
- [ ] Add unit tests for packet framing (deferred)
- [x] Add unit tests for config parsing - 2024-12-03
- [ ] Add integration test for TLS connection (deferred)
- [ ] Add integration test for packet roundtrip (deferred)
- [ ] Set up test fixtures (deferred)

#### Code Cleanup
- [x] Run cargo clippy and fix warnings - 2024-12-03
- [x] Run cargo fmt on entire codebase - 2024-12-03
- [x] Remove commented-out code in main.rs - 2024-12-03
- [x] Consolidate constants (IPs, ports, MTU) - 2024-12-03 (new constants.rs module)

---

## Phase 2: Actual VPN Functionality

### Milestone 2.1: Bidirectional Traffic ✅ COMPLETED 2024-12-08

#### Server-Side Improvements
- [x] Handle multiple packet types (not just echo) - 2024-12-08
- [x] Read from TUN and send to client - 2024-12-08
- [x] Implement proper bidirectional forwarding - 2024-12-08
- [x] Add connection state management - 2024-12-08

#### Client-Side Improvements
- [x] Implement bidirectional packet handling - 2024-12-08
- [x] Read from TLS and write to TUN - 2024-12-08
- [x] Read from TUN and write to TLS concurrently - 2024-12-08
- [x] Handle connection drops gracefully - 2024-12-08

### Milestone 2.2: Traffic Routing ✅ COMPLETED 2024-12-08

#### Route Management
- [x] Add route to VPN subnet on client - 2024-12-08
- [ ] Configure default gateway through tunnel (optional - deferred)
- [ ] Implement split tunneling support (deferred)
- [x] Add route cleanup on disconnect - 2024-12-08

#### Server Forwarding
- [x] Enable IP forwarding on server (sysctl) - 2024-12-08
- [x] Implement NAT for outbound traffic - 2024-12-08
- [x] Forward packets from TUN to physical interface - 2024-12-08
- [x] Forward responses back to client - 2024-12-08

**New module added:** `src/net/route.rs` - Route and NAT management

### Milestone 2.3: DNS Handling (Deferred)

- [ ] Capture DNS queries on client
- [ ] Forward DNS through tunnel
- [ ] Configure DNS servers via DHCP-like protocol
- [ ] Restore original DNS on disconnect

### Milestone 2.4: Connection Resilience ✅ COMPLETED 2024-12-08

- [x] Implement keepalive packets - 2024-12-08
- [x] Detect connection loss - 2024-12-08
- [x] Add automatic reconnection logic - 2024-12-08
- [x] Implement exponential backoff for reconnects - 2024-12-08
- [x] Handle server restart gracefully - 2024-12-08

**Implementation details:**
- Keepalive packets sent every 10 seconds (length=0 marker)
- Connection timeout after 30 seconds of inactivity
- Exponential backoff: 1s → 2s → 4s → ... up to 30s max
- Client automatically reconnects on connection loss

---

## Phase 3: CLI & Usability

### Milestone 3.1: Command Line Interface ✅ COMPLETED 2024-12-08

#### Argument Parsing
- [x] Add clap for CLI argument parsing - 2024-12-08
- [x] Implement server subcommand - 2024-12-08
- [x] Implement client subcommand - 2024-12-08
- [x] Add --config flag for configuration file - 2024-12-08
- [x] Add --verbose/-v flag for debug output - 2024-12-08

#### Configuration Files
- [x] Design TOML configuration schema - 2024-12-08
- [x] Implement config file parsing - 2024-12-08
- [ ] Support environment variable overrides (deferred)
- [ ] Add config file validation (deferred)
- [x] Create example configuration files - 2024-12-08

**New modules added:**
- `src/cli.rs` - Command-line argument definitions with clap
- `src/config/mod.rs` - Config module root
- `src/config/ovpn.rs` - OpenVPN config parsing (moved)
- `src/config/toml_config.rs` - TOML config support
- `config.example.toml` - Example configuration file

**Unified binary:** `vpn-rust` with server/client subcommands

### Milestone 3.2: Terminal UI ✅ COMPLETED 2024-12-08

- [x] Set up ratatui application structure - 2024-12-08
- [x] Create connection status display - 2024-12-08
- [x] Add traffic statistics (bytes sent/received) - 2024-12-08
- [x] Show connection duration - 2024-12-08
- [x] Add log viewer panel - 2024-12-08

**New modules added:**
- `src/tui/mod.rs` - TUI module root with public exports
- `src/tui/app.rs` - Application state, events, stats tracking
- `src/tui/ui.rs` - UI rendering with ratatui (title, status, stats, logs, help panels)
- `src/tui/runner.rs` - Terminal setup/cleanup and event loop runner

---

## Phase 4: Production Features

### Milestone 4.1: Security Hardening ✅ COMPLETED 2024-12-08

#### Authentication
- [x] Implement client certificate authentication (mTLS) - 2024-12-08
- [x] Add certificate validation on server - 2024-12-08
- [ ] Support certificate revocation checking (deferred)
- [ ] Add pre-shared key option (deferred)

#### Security Features
- [x] Add kill switch (block non-VPN traffic) - 2024-12-08
- [x] Implement DNS leak prevention - 2024-12-08
- [x] Add IPv6 leak prevention - 2024-12-08
- [ ] Secure memory handling for keys (deferred)

**New/Modified modules:**
- `src/net/tls.rs` - Added mTLS support with `ClientTlsConfig`, `ServerTlsConfig`, `connect_tls_with_config()`, `start_tls_server_with_config()`, `get_client_cert_cn()`
- `src/net/security.rs` - NEW - Kill switch, DNS leak prevention, IPv6 leak prevention with `SecurityManager`
- `src/constants.rs` - Added CA_CERT_PATH, CLIENT_CERT_PATH, CLIENT_KEY_PATH

### Milestone 4.2: Multi-Client Support ✅ COMPLETED 2024-12-08

- [x] Design client management system - 2024-12-08
- [x] Implement client connection tracking - 2024-12-08
- [x] Add IP address assignment (DHCP-like) - 2024-12-08
- [x] Support client-to-client routing - 2024-12-08
- [x] Add per-client traffic statistics - 2024-12-08

**New module:** `src/net/clients.rs`
- `IpPool` - DHCP-like IP address allocation from CIDR subnet
- `ClientStats` / `ClientStatsSnapshot` - Per-client traffic statistics
- `ClientConnection` - Client connection info with auth status, cert CN
- `ClientManager` - Full client lifecycle management with async support

### Milestone 4.3: Performance Optimization (Deferred)

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

### Phase 1 Success ✅ COMPLETED
- [x] TUN interface creation works
- [x] TLS tunnel established
- [x] Packets echo through tunnel
- [x] Code is well-documented (comprehensive doc comments added)
- [x] Tests pass (5 unit tests, 5 doc tests)

### Phase 2 Success (Partial - DNS Deferred)
- [x] Real IP traffic routes through tunnel - 2024-12-08
- [ ] DNS works through tunnel (deferred to Phase 3)
- [x] Server can forward to internet (NAT configured) - 2024-12-08
- [x] Connection survives interruptions (keepalive + reconnect) - 2024-12-08
- [ ] Latency < 50ms overhead (needs testing)

### Phase 3 Success ✅ COMPLETED 2024-12-08
- [x] CLI is intuitive and documented - 2024-12-08
- [x] Config files work reliably - 2024-12-08
- [x] TUI shows useful information (connection status, traffic stats, logs) - 2024-12-08
- [x] Error messages are helpful - 2024-12-08

### Phase 4 Success (Partial - Performance Deferred)
- [x] Client authentication works (mTLS implemented) - 2024-12-08
- [x] Multiple clients supported (ClientManager with IP pool) - 2024-12-08
- [x] Kill switch prevents leaks (iptables-based) - 2024-12-08
- [ ] Performance meets targets (deferred to 4.3)

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
