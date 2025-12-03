//! Constants used throughout the VPN application.
//!
//! This module centralizes all hardcoded values to make configuration
//! and maintenance easier.

/// Default MTU (Maximum Transmission Unit) for TUN interfaces.
pub const DEFAULT_MTU: i32 = 1500;

/// Buffer size for reading packets (MTU + 4 bytes for TUN header).
pub const PACKET_BUFFER_SIZE: usize = 1504;

/// Maximum packet size allowed (IP maximum).
pub const MAX_PACKET_SIZE: usize = 65535;

// --- Network Configuration ---

/// Server TUN interface name.
pub const SERVER_TUN_NAME: &str = "rustvpn0";

/// Client TUN interface name.
pub const CLIENT_TUN_NAME: &str = "rustvpn1";

/// Server IP address with CIDR notation.
pub const SERVER_IP_CIDR: &str = "10.8.0.1/30";

/// Client IP address with CIDR notation.
pub const CLIENT_IP_CIDR: &str = "10.8.0.2/30";

/// Server IP address (without CIDR).
pub const SERVER_IP: &str = "10.8.0.1";

/// Client IP address (without CIDR).
pub const CLIENT_IP: &str = "10.8.0.2";

// --- TLS Configuration ---

/// Default server bind address.
pub const DEFAULT_SERVER_ADDR: &str = "127.0.0.1";

/// Default server port.
pub const DEFAULT_SERVER_PORT: u16 = 4433;

/// Path to server certificate.
pub const SERVER_CERT_PATH: &str = "certs/server.crt";

/// Path to server private key.
pub const SERVER_KEY_PATH: &str = "certs/server.key";

// --- OpenVPN Compatibility ---

/// Default OpenVPN port.
pub const DEFAULT_OVPN_PORT: u16 = 1194;

// --- Timeouts ---

/// Time to wait after cleaning up an interface (milliseconds).
pub const INTERFACE_CLEANUP_DELAY_MS: u64 = 200;
