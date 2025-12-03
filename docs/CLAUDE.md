# VPN-Rust - Development Guide

Purpose: Defines development workflow, coding standards, and best practices for VPN-Rust - a learning-focused VPN client and server implementation in Rust demonstrating systems-level networking concepts.

## Current Project Status

**Phase:** Phase 1 - Core Infrastructure
**Status:** Early Prototype
**Architecture:** Client-Server with TLS Tunnel

### Completed Features
- Asynchronous TUN interface creation and management
- Secure TLS tunnel using rustls
- Basic client-server packet echo over TLS
- Manual IP assignment via system commands
- Self-signed certificate generation
- GitHub Actions CI pipeline

### Project Components
- **VPN Server**: TLS server with TUN device integration
- **VPN Client**: TLS client with TUN device integration
- **TUN Module**: Linux TUN/TAP device abstraction
- **TLS Module**: rustls-based secure communication
- **Config Module**: OpenVPN config file parsing (basic)

## Mandatory Session Startup

Always read PLANNING.md at the start of every new conversation, check TASKS.md before starting work, mark completed tasks immediately, and add newly discovered tasks when found.

## Session Workflow

1. **Context Loading**: Read PLANNING.md, CLAUDE.md, and TASKS.md to understand current state
2. **Task Selection**: Pick the topmost uncompleted task from TASKS.md (or specific task if directed)
3. **Implementation Planning**: Propose changes before implementing
4. **Execution**: Implement upon approval, following project conventions
5. **Completion**: Mark task complete with timestamp immediately after finishing
6. **Discovery**: Add any newly discovered prerequisites or follow-up tasks to TASKS.md
7. **Testing**: Run `cargo build` and `cargo test` to verify changes
8. **Documentation**: Update TASKS.md progress, CLAUDE.md for major sessions

## Coding Standards

### Rust Conventions
- Use Rust 2021 edition features
- Follow standard Rust naming conventions (snake_case for functions/variables, PascalCase for types)
- Use `rustfmt` for code formatting
- Use `clippy` for linting (run `cargo clippy`)
- Prefer `Result<T, E>` over `panic!` for error handling
- Use `anyhow::Result` for application-level errors
- Add context to errors with `.with_context(|| "message")`

### Module Structure
```
src/
├── lib.rs           # Library root, exports public modules
├── main.rs          # Debug/test binary
├── config.rs        # Configuration parsing
├── net/
│   ├── mod.rs       # Network module root
│   ├── tun.rs       # TUN device abstraction
│   └── tls.rs       # TLS connection handling
└── bin/
    ├── server.rs    # VPN server binary
    └── client.rs    # VPN client binary
```

### Naming Conventions
- `snake_case` for files, functions, variables, and modules
- `PascalCase` for structs, enums, and traits
- `SCREAMING_SNAKE_CASE` for constants
- Descriptive names over abbreviations
- Prefix async functions with action verbs (e.g., `read_packet`, `connect_tls`)

### Async Patterns
- Use `tokio` runtime for all async operations
- Prefer `Arc<Mutex<T>>` for shared mutable state across tasks
- Use `tokio::spawn` for concurrent task execution
- Use `AsyncFd` for async I/O on raw file descriptors
- Avoid blocking operations in async contexts

### Error Handling
- Use `anyhow::Result` for functions that can fail
- Add context with `.with_context()`
- Avoid `unwrap()` in production code - use `expect()` with clear messages or proper error handling
- Log errors before propagating with `log::error!`

### Testing Requirements
- Write unit tests for pure functions
- Write integration tests for network components (when possible)
- Use `tokio-test` for async test utilities
- Mock external dependencies where feasible
- Test error cases, not just happy paths

### Documentation
- Add doc comments (`///`) to all public items
- Include examples in doc comments for complex functions
- Document safety requirements for `unsafe` code
- Keep README.md updated with setup instructions

## Logging Standards

### Log Levels
- `error!`: Failures requiring immediate attention
- `warn!`: Recoverable issues or degraded performance
- `info!`: General operational events (connection established, etc.)
- `debug!`: Detailed diagnostic information
- `trace!`: Very detailed tracing (packet contents, etc.)

### Logging Best Practices
```rust
use log::{debug, error, info, warn};

// Good: Include context
info!("TUN interface created: {}", interface_name);
debug!("Received packet: {} bytes, header: {:?}", len, &buf[..20]);
error!("Failed to connect to server {}: {}", addr, err);

// Bad: Vague messages
info!("Done");
error!("Error occurred");
```

## Git Workflow

- Commit messages: Present tense, imperative mood ("Add feature" not "Added feature")
- Atomic commits: One logical change per commit
- No commits of certificates, keys, or secrets
- Update TASKS.md in same commit when completing tasks
- Run `cargo fmt` and `cargo clippy` before committing

### Commit Message Format
```
<type>: <short description>

[optional body with more details]

[optional footer with references]
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

## File Organization

- Core networking code in `src/net/`
- Binary applications in `src/bin/`
- Configuration handling in `src/config.rs`
- Examples in `examples/`
- Helper scripts in project root
- Certificates in `certs/` (gitignored except for .crt)
- Documentation in `docs/`
- Never create files outside defined structure without explicit approval

## Security Considerations

- Never log sensitive data (keys, passwords, packet payloads in production)
- Use TLS 1.2+ for all network communication
- Validate certificate chains in production builds
- Sanitize user input in configuration files
- Use secure random number generation for crypto operations
- Review `unsafe` blocks carefully - document safety invariants

## Local Development Setup

### Prerequisites
```bash
# Install Rust (if not present)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Linux: Install TUN/TAP support (usually built-in)
sudo modprobe tun

# Generate test certificates
./gen_certs.sh
```

### Building
```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run clippy
cargo clippy

# Format code
cargo fmt
```

### Running the VPN

```bash
# Terminal 1: Start server (requires root for TUN)
sudo cargo run --bin server

# Terminal 2: Start client (requires root for TUN)
sudo cargo run --bin client
```

### Cleaning Up
```bash
# Clean up TUN interfaces and processes
./cleanup_vpn.sh

# Or manually
sudo ip link delete rustvpn0 2>/dev/null
sudo ip link delete rustvpn1 2>/dev/null
```

## Key Technical Decisions

### TUN vs TAP
- Using TUN (Layer 3) for IP packet handling
- TAP (Layer 2) would be needed for Ethernet frame handling
- TUN is simpler and sufficient for VPN use case

### rustls vs OpenSSL
- Using `rustls` for memory-safe TLS implementation
- No dependency on system OpenSSL
- Modern TLS 1.3 support out of the box

### Tokio Runtime
- Full tokio features enabled for comprehensive async support
- `AsyncFd` for integrating TUN file descriptors with async I/O
- Single-threaded runtime sufficient for current scope

### Protocol Design
- Simple length-prefixed protocol: 2-byte big-endian length + data
- Echo-based for development; will need proper routing later
- No protocol versioning yet (future improvement)

## Critical Paths

### TUN Interface Creation Flow
1. Check for existing interface and clean up
2. Create TUN device with `tun` crate
3. Configure IP address via `ip addr add`
4. Bring interface up via `ip link set up`
5. Set MTU to 1500
6. Wrap file descriptor in `AsyncFd` for async I/O

### TLS Connection Flow
1. Load certificates (server) or root CA store (client)
2. Create TLS config with rustls
3. Establish TCP connection
4. Perform TLS handshake
5. Return async TLS stream for packet I/O

### Packet Tunnel Flow
1. Read packet from TUN interface
2. Prepend 2-byte length header
3. Send over TLS connection
4. Receive response from peer
5. Write packet to TUN interface

## Common Issues & Solutions

### TUN Interface Creation Fails
- Verify root/CAP_NET_ADMIN permissions
- Check if TUN module is loaded: `lsmod | grep tun`
- Clean up stale interfaces: `./cleanup_vpn.sh`
- Check for conflicting VPN software

### TLS Connection Fails
- Verify certificates exist in `certs/`
- Regenerate if expired: `./gen_certs.sh`
- Check server is running and listening
- Verify firewall allows port 4433

### Build Errors
- Run `cargo clean` and rebuild
- Check Rust version: `rustc --version` (need 1.56+)
- Update dependencies: `cargo update`

### Permission Denied Errors
- TUN operations require root: use `sudo cargo run`
- Or add CAP_NET_ADMIN capability to binary

## Platform Support

### Currently Supported
- **Linux**: Full support with TUN/TAP

### Planned
- **Windows**: Dependencies prepared (`winapi`), not yet implemented
- **macOS**: Requires `utun` implementation

## Dependencies Overview

| Crate | Purpose | Notes |
|-------|---------|-------|
| `tokio` | Async runtime | Full features enabled |
| `rustls` | TLS implementation | Memory-safe, modern |
| `tokio-rustls` | Async TLS | Tokio integration |
| `tun` | TUN device creation | Linux-only currently |
| `anyhow` | Error handling | Application errors |
| `log` | Logging facade | Use with env_logger |
| `clap` | CLI parsing | Prepared, not used yet |
| `ratatui` | Terminal UI | Prepared, not used yet |

## Session Log Template

For documenting significant work sessions:

```markdown
### Session: [DATE TIME]
**Tasks Completed:**
- [Task description with file references]

**Files Modified:**
- path/to/file.rs: [Brief change description]

**Key Decisions:**
- [Technical choice and rationale]

**Next Steps:**
- [Proposed follow-up tasks]

**Notes:**
- [Any important observations or blockers]
```

## Additional Resources

- **PLANNING.md**: Technical architecture and design decisions
- **TASKS.md**: Implementation milestones and progress tracking
- **PRD.md**: Product requirements and success criteria
- **README.md**: Project overview and quick start guide
