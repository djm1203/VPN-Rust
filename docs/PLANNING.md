# VPN-Rust - Technical Planning & Architecture

## System Architecture

### High-Level Overview
```
┌─────────────────────────────────────────────────────────────────────────┐
│                              VPN System                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   ┌──────────────────────┐              ┌──────────────────────┐       │
│   │      VPN Client      │    TLS       │      VPN Server      │       │
│   │                      │◀────────────▶│                      │       │
│   │  ┌────────────────┐  │   Tunnel     │  ┌────────────────┐  │       │
│   │  │  TUN Interface │  │              │  │  TUN Interface │  │       │
│   │  │  (rustvpn1)    │  │              │  │  (rustvpn0)    │  │       │
│   │  │  10.8.0.2/30   │  │              │  │  10.8.0.1/30   │  │       │
│   │  └────────────────┘  │              │  └────────────────┘  │       │
│   │         │            │              │         │            │       │
│   │         ▼            │              │         ▼            │       │
│   │  ┌────────────────┐  │              │  ┌────────────────┐  │       │
│   │  │ Packet Handler │  │              │  │ Packet Handler │  │       │
│   │  └────────────────┘  │              │  └────────────────┘  │       │
│   │         │            │              │         │            │       │
│   │         ▼            │              │         ▼            │       │
│   │  ┌────────────────┐  │              │  ┌────────────────┐  │       │
│   │  │  TLS Stream    │──┼──────────────┼──│  TLS Acceptor  │  │       │
│   │  └────────────────┘  │              │  └────────────────┘  │       │
│   └──────────────────────┘              └──────────────────────┘       │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Component Interaction Flow
```
┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐
│  App    │────▶│   TUN   │────▶│ Encrypt │────▶│   TLS   │────▶│ Server  │
│         │     │ Device  │     │ + Frame │     │ Stream  │     │         │
└─────────┘     └─────────┘     └─────────┘     └─────────┘     └─────────┘
                    │                                               │
                    │           ◀──────────────────────────────────┘
                    ▼                    Response Path
              Local Network
```

## Technology Stack

### Core Technologies
| Component | Technology | Version | Purpose |
|-----------|------------|---------|---------|
| Language | Rust | 2021 Edition | Systems programming |
| Async Runtime | Tokio | 1.38 | Async I/O and task scheduling |
| TLS | rustls | 0.21 | Memory-safe TLS implementation |
| TLS Async | tokio-rustls | 0.24 | Tokio + rustls integration |
| TUN/TAP | tun | 0.6 | Virtual network interface |
| Error Handling | anyhow | 1.0 | Application error handling |
| Logging | log + env_logger | 0.4 | Structured logging |
| CLI | clap | 4.5 | Command-line parsing (planned) |
| TUI | ratatui | 0.25 | Terminal UI (planned) |

### Cryptography Stack
- **TLS Library**: rustls (pure Rust, no OpenSSL dependency)
- **Certificate Format**: PEM (X.509)
- **Key Exchange**: TLS 1.3 default cipher suites
- **Root CAs**: webpki-roots for system trust store

## Data Models

### TUN Interface Configuration
```rust
pub struct TunInterface {
    name: String,           // Interface name (e.g., "rustvpn0")
    fd: AsyncFd<File>,      // Async file descriptor
    mtu: u32,               // Maximum transmission unit (1500)
}

// Server configuration
const SERVER_TUN_NAME: &str = "rustvpn0";
const SERVER_IP: &str = "10.8.0.1/30";

// Client configuration
const CLIENT_TUN_NAME: &str = "rustvpn1";
const CLIENT_IP: &str = "10.8.0.2/30";
```

### Packet Protocol (Current)
```rust
// Simple length-prefixed protocol
struct TunnelPacket {
    length: u16,        // Big-endian, max 65535 bytes
    data: Vec<u8>,      // Raw IP packet
}
```

### Configuration Model (Planned)
```rust
pub struct VpnConfig {
    // Connection settings
    server_address: String,
    server_port: u16,

    // TUN settings
    tun_name: String,
    local_ip: IpAddr,
    subnet_mask: u8,
    mtu: u32,

    // TLS settings
    cert_path: Option<PathBuf>,
    key_path: Option<PathBuf>,
    ca_path: Option<PathBuf>,
    verify_server: bool,

    // Operational settings
    reconnect_attempts: u32,
    keepalive_interval: Duration,
    log_level: LogLevel,
}
```

### Connection State Model (Planned)
```rust
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Authenticating,
    Connected {
        since: Instant,
        bytes_sent: u64,
        bytes_received: u64,
    },
    Reconnecting {
        attempt: u32,
        last_error: String,
    },
    Error(String),
}
```

## Module Architecture

### Current Structure
```
src/
├── lib.rs                 # Library crate root
│   └── Exports: config, net
│
├── config.rs              # Configuration parsing
│   └── OVPNConfig        # Parse .ovpn files
│
├── net/
│   ├── mod.rs            # Network module
│   │   └── Exports: tls, tun
│   │
│   ├── tun.rs            # TUN device management
│   │   └── TunInterface  # Create, configure, read/write
│   │
│   └── tls.rs            # TLS connections
│       ├── connect_tls() # Client TLS connection
│       └── start_tls_server() # Server TLS acceptor
│
├── main.rs               # Debug/test binary
│
└── bin/
    ├── server.rs         # VPN server binary
    └── client.rs         # VPN client binary
```

### Planned Structure
```
src/
├── lib.rs
├── config/
│   ├── mod.rs
│   ├── parser.rs         # Config file parsing
│   ├── validator.rs      # Config validation
│   └── types.rs          # Config data types
│
├── net/
│   ├── mod.rs
│   ├── tun.rs
│   ├── tls.rs
│   └── routing.rs        # Route management (new)
│
├── tunnel/
│   ├── mod.rs
│   ├── client.rs         # Client tunnel logic
│   ├── server.rs         # Server tunnel logic
│   └── protocol.rs       # Wire protocol
│
├── cli/
│   ├── mod.rs
│   ├── commands.rs       # CLI commands
│   └── tui.rs            # Terminal UI
│
└── bin/
    ├── vpn-server.rs
    └── vpn-client.rs
```

## Processing Pipelines

### Packet Send Pipeline (Client → Server)
```rust
// 1. Application sends packet
let packet = app_socket.read().await?;

// 2. TUN device captures packet
let mut buf = [0u8; 1500];
let len = tun.read_packet(&mut buf).await?;

// 3. Frame packet with length prefix
let frame = frame_packet(&buf[..len]);

// 4. Send over TLS
tls_stream.write_all(&frame).await?;

// 5. Server receives and processes
// (Reverse on server side)
```

### TUN Interface Lifecycle
```rust
// 1. Create interface
let config = tun::Configuration::default();
config.name(interface_name);
config.layer(tun::Layer::L3);  // TUN mode (Layer 3)
config.mtu(1500);
let device = tun::create(&config)?;

// 2. Configure IP
Command::new("ip")
    .args(["addr", "add", ip_addr, "dev", interface_name])
    .status()?;

// 3. Bring up interface
Command::new("ip")
    .args(["link", "set", "dev", interface_name, "up"])
    .status()?;

// 4. Wrap in AsyncFd for async I/O
let async_fd = AsyncFd::new(device)?;

// 5. Use for packet I/O
// ... read_packet / write_packet ...

// 6. Cleanup on drop (automatic via Drop trait)
Command::new("ip")
    .args(["link", "delete", interface_name])
    .status()?;
```

### TLS Connection Pipeline
```rust
// Client side
async fn connect_tls(addr: &str) -> Result<TlsStream<TcpStream>> {
    // 1. Create TCP connection
    let stream = TcpStream::connect(addr).await?;

    // 2. Configure TLS with root certificates
    let root_store = RootCertStore::from(webpki_roots::TLS_SERVER_ROOTS);
    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    // 3. Create TLS connector
    let connector = TlsConnector::from(Arc::new(config));

    // 4. Perform TLS handshake
    let tls_stream = connector.connect(server_name, stream).await?;

    Ok(tls_stream)
}

// Server side
async fn start_tls_server(addr: &str) -> Result<(TcpListener, TlsAcceptor)> {
    // 1. Load certificate and key
    let cert = load_certs("certs/server.crt")?;
    let key = load_key("certs/server.key")?;

    // 2. Configure TLS
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert, key)?;

    // 3. Create acceptor
    let acceptor = TlsAcceptor::from(Arc::new(config));

    // 4. Bind TCP listener
    let listener = TcpListener::bind(addr).await?;

    Ok((listener, acceptor))
}
```

## API Design

### Current Endpoints (Binary Protocols)
```
Server: 127.0.0.1:4433
├── TLS Handshake
├── Accept: Tunnel packets (length-prefixed)
└── Response: Echo packets back

Protocol:
┌──────────┬────────────────┐
│ 2 bytes  │    N bytes     │
│ Length   │  Packet Data   │
└──────────┴────────────────┘
```

### Planned CLI Interface
```bash
# Server commands
vpn-server start [--config <path>] [--port <port>]
vpn-server stop
vpn-server status
vpn-server clients      # List connected clients

# Client commands
vpn-client connect <server> [--config <path>]
vpn-client disconnect
vpn-client status
vpn-client stats        # Show traffic statistics
```

### Planned Control Protocol (Future)
```rust
enum ControlMessage {
    // Client → Server
    Authenticate { token: Vec<u8> },
    Keepalive,
    Disconnect,

    // Server → Client
    AuthResult { success: bool, message: String },
    ServerInfo { version: String, features: Vec<String> },
    KeepaliveAck,

    // Bidirectional
    Error { code: u32, message: String },
}
```

## Configuration Management

### Environment Variables
```bash
# Logging
RUST_LOG=debug              # Log level (trace, debug, info, warn, error)

# Server settings
VPN_SERVER_ADDR=0.0.0.0     # Bind address
VPN_SERVER_PORT=4433        # Listen port
VPN_CERT_PATH=certs/server.crt
VPN_KEY_PATH=certs/server.key

# Client settings
VPN_REMOTE_HOST=vpn.example.com
VPN_REMOTE_PORT=4433

# TUN settings
VPN_TUN_NAME=rustvpn0
VPN_TUN_MTU=1500
VPN_SUBNET=10.8.0.0/24
```

### Configuration File Format (Planned)
```toml
# vpn.toml

[server]
bind_address = "0.0.0.0"
port = 4433
max_clients = 10

[tls]
cert = "certs/server.crt"
key = "certs/server.key"
# ca = "certs/ca.crt"  # For client auth

[tunnel]
subnet = "10.8.0.0/24"
mtu = 1500
dns = ["8.8.8.8", "8.8.4.4"]

[logging]
level = "info"
file = "/var/log/vpn-rust.log"
```

## Security Considerations

### Current Security Model
- TLS 1.2/1.3 for transport encryption
- Self-signed certificates (development only)
- No client authentication
- No packet validation beyond TLS

### Planned Security Enhancements
1. **Certificate Validation**: Proper CA chain verification
2. **Client Authentication**: Mutual TLS with client certificates
3. **Packet Validation**: Verify IP packet integrity
4. **Rate Limiting**: Prevent DoS attacks
5. **Logging**: Security event logging

### Security Best Practices
```rust
// DO: Use secure defaults
let config = ClientConfig::builder()
    .with_safe_defaults()  // TLS 1.2+, secure cipher suites
    .with_root_certificates(root_store)
    .with_no_client_auth();

// DON'T: Skip certificate verification in production
// let config = config.dangerous().set_certificate_verifier(...);

// DO: Clear sensitive data
fn drop_secret(mut secret: Vec<u8>) {
    secret.zeroize();  // Use zeroize crate
}

// DO: Validate input lengths
fn read_packet(buf: &mut [u8]) -> Result<usize> {
    let len = stream.read_u16().await? as usize;
    if len > MAX_PACKET_SIZE {
        return Err(anyhow!("Packet too large: {}", len));
    }
    // ...
}
```

## Performance Targets

### Latency
| Operation | Target | Notes |
|-----------|--------|-------|
| TUN read/write | < 1ms | Kernel operation |
| Packet framing | < 0.1ms | Memory copy |
| TLS encrypt/decrypt | < 1ms | Per packet |
| Total tunnel overhead | < 5ms | End-to-end |

### Throughput
| Metric | Target | Notes |
|--------|--------|-------|
| Single connection | > 100 Mbps | Local network |
| Packets per second | > 10,000 | Small packets |
| Memory usage | < 50 MB | Client |
| Memory usage | < 100 MB | Server with 10 clients |

### Resource Limits
```rust
// Recommended limits
const MAX_PACKET_SIZE: usize = 65535;  // IP max
const MAX_MTU: u32 = 1500;             // Ethernet standard
const READ_BUFFER_SIZE: usize = 4096;  // Per connection
const MAX_CLIENTS: usize = 100;        // Server limit
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(60);
```

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_framing() {
        let data = b"hello";
        let framed = frame_packet(data);
        assert_eq!(framed[0..2], [0, 5]);  // Length
        assert_eq!(&framed[2..], data);
    }

    #[test]
    fn test_config_parsing() {
        let config = OVPNConfig::from_file("test.ovpn").unwrap();
        assert_eq!(config.remote, "vpn.example.com");
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_tls_connection() {
    // Start test server
    let (listener, acceptor) = start_tls_server("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Connect client
    let client = connect_tls(&addr.to_string()).await.unwrap();

    // Verify connection
    assert!(client.get_ref().1.is_handshaking() == false);
}
```

### Manual Testing
```bash
# Terminal 1: Start server
sudo RUST_LOG=debug cargo run --bin server

# Terminal 2: Start client
sudo RUST_LOG=debug cargo run --bin client

# Terminal 3: Test connectivity
ping 10.8.0.1  # From client, should reach server

# Terminal 4: Monitor traffic
sudo tcpdump -i rustvpn0 -n
```

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2024-01 | Use TUN over TAP | Layer 3 sufficient for IP tunneling, simpler |
| 2024-01 | Use rustls over OpenSSL | Memory safety, no system dependency |
| 2024-01 | Use tokio for async | Industry standard, full-featured |
| 2024-01 | Simple length-prefix protocol | Easy to implement, debug; extend later |
| 2024-01 | Single-threaded runtime | Sufficient for learning/personal use |
| 2024-01 | Linux-first development | Primary use case, extend to other platforms later |

## Open Questions

- [ ] Should we implement keepalive at application level or rely on TCP keepalive?
- [ ] How to handle MTU discovery and fragmentation?
- [ ] Should we support UDP transport for lower latency?
- [ ] How to implement split tunneling (route only some traffic)?
- [ ] Should we add compression (e.g., LZ4)?
- [ ] How to handle IPv6?

## Future Architecture Considerations

### Multi-Client Server
```rust
// Connection manager for multiple clients
struct ConnectionManager {
    clients: HashMap<ClientId, ClientConnection>,
    routing_table: HashMap<IpAddr, ClientId>,
}

// Per-client state
struct ClientConnection {
    id: ClientId,
    stream: TlsStream<TcpStream>,
    assigned_ip: IpAddr,
    connected_at: Instant,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
}
```

### Traffic Routing (Server)
```rust
// Forward packets between clients and internet
async fn route_packet(packet: &[u8], routing: &RoutingTable) -> Result<()> {
    let ip_header = parse_ip_header(packet)?;

    match routing.lookup(ip_header.dst_addr) {
        Destination::Client(client_id) => {
            // Forward to another VPN client
            send_to_client(client_id, packet).await
        }
        Destination::Internet => {
            // NAT and forward to internet
            nat_and_forward(packet).await
        }
        Destination::Local => {
            // Deliver locally
            Ok(())
        }
    }
}
```

## Platform Considerations

### Linux
- Full TUN/TAP support via `/dev/net/tun`
- Route management via `ip` command or netlink
- Capabilities: CAP_NET_ADMIN for non-root operation

### macOS (Planned)
- Use `utun` devices instead of tun/tap
- Different ioctl interface
- Route management via `route` command

### Windows (Planned)
- Requires TAP-Windows or Wintun driver
- Different API surface
- Route management via netsh or Windows API

## References

### Documentation
- [Linux TUN/TAP](https://www.kernel.org/doc/Documentation/networking/tuntap.txt)
- [rustls Guide](https://docs.rs/rustls/latest/rustls/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [tun crate](https://docs.rs/tun/latest/tun/)

### Protocols
- [OpenVPN Protocol](https://openvpn.net/community-resources/reference-manual-for-openvpn-2-6/)
- [WireGuard Protocol](https://www.wireguard.com/protocol/)
- [TLS 1.3 RFC 8446](https://tools.ietf.org/html/rfc8446)

### Similar Projects
- [OpenVPN](https://github.com/OpenVPN/openvpn)
- [WireGuard](https://www.wireguard.com/)
- [boringtun](https://github.com/cloudflare/boringtun) (Rust WireGuard)
