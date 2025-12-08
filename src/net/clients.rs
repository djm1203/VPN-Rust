//! Client management system for the VPN server.
//!
//! This module provides multi-client support including:
//! - Client connection tracking
//! - IP address assignment (DHCP-like)
//! - Per-client traffic statistics
//! - Client-to-client routing

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use log::{debug, info, warn};
use tokio::sync::RwLock;

// =============================================================================
// IP Address Pool
// =============================================================================

/// IP address pool for client assignment.
#[derive(Debug)]
pub struct IpPool {
    /// Base network address (e.g., 10.8.0.0).
    base_addr: Ipv4Addr,
    /// Network prefix length (e.g., 24 for /24).
    prefix_len: u8,
    /// Set of available IP addresses.
    available: Vec<Ipv4Addr>,
    /// Set of assigned IP addresses (maps IP to client ID).
    assigned: HashMap<Ipv4Addr, String>,
}

impl IpPool {
    /// Create a new IP pool from a CIDR notation string.
    ///
    /// # Arguments
    ///
    /// * `cidr` - Network in CIDR notation (e.g., "10.8.0.0/24")
    /// * `gateway_ip` - The server's IP (excluded from pool)
    ///
    /// # Example
    ///
    /// ```
    /// use vpn_rust::net::clients::IpPool;
    ///
    /// let pool = IpPool::from_cidr("10.8.0.0/24", "10.8.0.1").unwrap();
    /// ```
    pub fn from_cidr(cidr: &str, gateway_ip: &str) -> Result<Self> {
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid CIDR notation: {}", cidr);
        }

        let base_addr: Ipv4Addr = parts[0]
            .parse()
            .with_context(|| format!("Invalid IP address: {}", parts[0]))?;
        let prefix_len: u8 = parts[1]
            .parse()
            .with_context(|| format!("Invalid prefix length: {}", parts[1]))?;

        let gateway: Ipv4Addr = gateway_ip
            .parse()
            .with_context(|| format!("Invalid gateway IP: {}", gateway_ip))?;

        // Calculate the number of host addresses
        let host_bits = 32 - prefix_len;
        if host_bits > 16 {
            anyhow::bail!("Network too large (max /16): {}", cidr);
        }

        let num_hosts = (1u32 << host_bits) - 2; // Exclude network and broadcast

        // Generate available addresses (skip network addr, gateway, and broadcast)
        let base_u32 = u32::from(base_addr);
        let mut available = Vec::with_capacity(num_hosts as usize);

        for i in 1..=(num_hosts) {
            let ip = Ipv4Addr::from(base_u32 + i);
            if ip != gateway {
                available.push(ip);
            }
        }

        debug!(
            "Created IP pool {} with {} available addresses",
            cidr,
            available.len()
        );

        Ok(Self {
            base_addr,
            prefix_len,
            available,
            assigned: HashMap::new(),
        })
    }

    /// Allocate an IP address for a client.
    ///
    /// Returns the assigned IP address or None if the pool is exhausted.
    pub fn allocate(&mut self, client_id: &str) -> Option<Ipv4Addr> {
        // Check if client already has an IP
        if let Some((&ip, _)) = self.assigned.iter().find(|(_, id)| *id == client_id) {
            debug!("Client {} already has IP {}", client_id, ip);
            return Some(ip);
        }

        // Allocate new IP
        let ip = self.available.pop()?;
        self.assigned.insert(ip, client_id.to_string());
        debug!("Allocated IP {} to client {}", ip, client_id);
        Some(ip)
    }

    /// Release an IP address back to the pool.
    pub fn release(&mut self, ip: Ipv4Addr) {
        if let Some(client_id) = self.assigned.remove(&ip) {
            self.available.push(ip);
            debug!("Released IP {} from client {}", ip, client_id);
        }
    }

    /// Release all IPs assigned to a client.
    pub fn release_by_client(&mut self, client_id: &str) {
        let ips_to_release: Vec<Ipv4Addr> = self
            .assigned
            .iter()
            .filter(|(_, id)| *id == client_id)
            .map(|(&ip, _)| ip)
            .collect();

        for ip in ips_to_release {
            self.release(ip);
        }
    }

    /// Get the IP assigned to a client.
    pub fn get_client_ip(&self, client_id: &str) -> Option<Ipv4Addr> {
        self.assigned
            .iter()
            .find(|(_, id)| *id == client_id)
            .map(|(&ip, _)| ip)
    }

    /// Get the client ID for an IP address.
    pub fn get_client_for_ip(&self, ip: Ipv4Addr) -> Option<&str> {
        self.assigned.get(&ip).map(|s| s.as_str())
    }

    /// Get the number of available addresses.
    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    /// Get the number of assigned addresses.
    pub fn assigned_count(&self) -> usize {
        self.assigned.len()
    }

    /// Get the network CIDR.
    pub fn cidr(&self) -> String {
        format!("{}/{}", self.base_addr, self.prefix_len)
    }
}

// =============================================================================
// Per-Client Statistics
// =============================================================================

/// Traffic statistics for a single client.
#[derive(Debug, Default)]
pub struct ClientStats {
    /// Bytes sent to this client.
    pub bytes_sent: AtomicU64,
    /// Bytes received from this client.
    pub bytes_received: AtomicU64,
    /// Packets sent to this client.
    pub packets_sent: AtomicU64,
    /// Packets received from this client.
    pub packets_received: AtomicU64,
}

impl ClientStats {
    /// Create new client statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record bytes/packet sent to client.
    pub fn record_sent(&self, bytes: u64) {
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Record bytes/packet received from client.
    pub fn record_received(&self, bytes: u64) {
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
        self.packets_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current statistics snapshot.
    pub fn snapshot(&self) -> ClientStatsSnapshot {
        ClientStatsSnapshot {
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            packets_sent: self.packets_sent.load(Ordering::Relaxed),
            packets_received: self.packets_received.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of client statistics (non-atomic copy).
#[derive(Debug, Clone, Copy, Default)]
pub struct ClientStatsSnapshot {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
}

// =============================================================================
// Client Connection
// =============================================================================

/// Information about a connected client.
#[derive(Debug)]
pub struct ClientConnection {
    /// Unique client identifier (from certificate CN or generated).
    pub id: String,
    /// Socket address of the client.
    pub socket_addr: SocketAddr,
    /// Assigned VPN IP address.
    pub vpn_ip: Ipv4Addr,
    /// Connection timestamp.
    pub connected_at: Instant,
    /// Last activity timestamp.
    pub last_activity: Instant,
    /// Traffic statistics.
    pub stats: Arc<ClientStats>,
    /// Whether the client is authenticated (mTLS).
    pub authenticated: bool,
    /// Client certificate CN (if using mTLS).
    pub cert_cn: Option<String>,
}

impl ClientConnection {
    /// Create a new client connection.
    pub fn new(
        id: String,
        socket_addr: SocketAddr,
        vpn_ip: Ipv4Addr,
        authenticated: bool,
        cert_cn: Option<String>,
    ) -> Self {
        let now = Instant::now();
        Self {
            id,
            socket_addr,
            vpn_ip,
            connected_at: now,
            last_activity: now,
            stats: Arc::new(ClientStats::new()),
            authenticated,
            cert_cn,
        }
    }

    /// Update the last activity timestamp.
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Get connection duration.
    pub fn duration(&self) -> std::time::Duration {
        self.connected_at.elapsed()
    }

    /// Get time since last activity.
    pub fn idle_time(&self) -> std::time::Duration {
        self.last_activity.elapsed()
    }
}

// =============================================================================
// Client Manager
// =============================================================================

/// Manages all connected clients.
pub struct ClientManager {
    /// IP address pool for allocation.
    ip_pool: RwLock<IpPool>,
    /// Connected clients (keyed by socket address).
    clients: RwLock<HashMap<SocketAddr, ClientConnection>>,
    /// VPN IP to socket address mapping (for routing).
    ip_to_socket: RwLock<HashMap<Ipv4Addr, SocketAddr>>,
    /// Whether to allow client-to-client routing.
    allow_client_routing: bool,
}

impl ClientManager {
    /// Create a new client manager.
    ///
    /// # Arguments
    ///
    /// * `subnet` - The VPN subnet in CIDR notation
    /// * `gateway_ip` - The server's VPN IP address
    /// * `allow_client_routing` - Whether to allow client-to-client traffic
    pub fn new(subnet: &str, gateway_ip: &str, allow_client_routing: bool) -> Result<Self> {
        let ip_pool = IpPool::from_cidr(subnet, gateway_ip)?;

        info!(
            "Client manager initialized with subnet {} ({} available IPs)",
            subnet,
            ip_pool.available_count()
        );

        Ok(Self {
            ip_pool: RwLock::new(ip_pool),
            clients: RwLock::new(HashMap::new()),
            ip_to_socket: RwLock::new(HashMap::new()),
            allow_client_routing,
        })
    }

    /// Register a new client connection.
    ///
    /// Returns the assigned VPN IP address.
    pub async fn register_client(
        &self,
        socket_addr: SocketAddr,
        client_id: Option<&str>,
        authenticated: bool,
        cert_cn: Option<String>,
    ) -> Result<ClientConnection> {
        // Generate client ID if not provided
        let id = client_id
            .map(String::from)
            .or_else(|| cert_cn.clone())
            .unwrap_or_else(|| format!("client-{}", socket_addr));

        // Allocate IP address
        let vpn_ip = {
            let mut pool = self.ip_pool.write().await;
            pool.allocate(&id)
                .ok_or_else(|| anyhow::anyhow!("IP pool exhausted"))?
        };

        // Create client connection
        let connection = ClientConnection::new(id.clone(), socket_addr, vpn_ip, authenticated, cert_cn);

        // Store in maps
        {
            let mut clients = self.clients.write().await;
            clients.insert(socket_addr, connection);
        }

        {
            let mut ip_map = self.ip_to_socket.write().await;
            ip_map.insert(vpn_ip, socket_addr);
        }

        info!(
            "Registered client {} ({}) with VPN IP {}",
            id, socket_addr, vpn_ip
        );

        // Return a copy of the connection info
        let clients = self.clients.read().await;
        Ok(ClientConnection::new(
            id,
            socket_addr,
            vpn_ip,
            authenticated,
            clients.get(&socket_addr).and_then(|c| c.cert_cn.clone()),
        ))
    }

    /// Unregister a client connection.
    pub async fn unregister_client(&self, socket_addr: SocketAddr) {
        let client = {
            let mut clients = self.clients.write().await;
            clients.remove(&socket_addr)
        };

        if let Some(client) = client {
            // Release IP
            {
                let mut pool = self.ip_pool.write().await;
                pool.release(client.vpn_ip);
            }

            // Remove from IP map
            {
                let mut ip_map = self.ip_to_socket.write().await;
                ip_map.remove(&client.vpn_ip);
            }

            let stats = client.stats.snapshot();
            info!(
                "Unregistered client {} ({}): sent {} bytes, received {} bytes",
                client.id, socket_addr, stats.bytes_sent, stats.bytes_received
            );
        }
    }

    /// Get a client by socket address.
    pub async fn get_client(&self, socket_addr: &SocketAddr) -> Option<ClientConnection> {
        let clients = self.clients.read().await;
        clients.get(socket_addr).map(|c| {
            ClientConnection::new(
                c.id.clone(),
                c.socket_addr,
                c.vpn_ip,
                c.authenticated,
                c.cert_cn.clone(),
            )
        })
    }

    /// Get a client by VPN IP address.
    pub async fn get_client_by_ip(&self, vpn_ip: Ipv4Addr) -> Option<SocketAddr> {
        let ip_map = self.ip_to_socket.read().await;
        ip_map.get(&vpn_ip).copied()
    }

    /// Update client activity timestamp.
    pub async fn touch_client(&self, socket_addr: &SocketAddr) {
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.get_mut(socket_addr) {
            client.touch();
        }
    }

    /// Record traffic for a client.
    pub async fn record_traffic(
        &self,
        socket_addr: &SocketAddr,
        bytes_sent: u64,
        bytes_received: u64,
    ) {
        let clients = self.clients.read().await;
        if let Some(client) = clients.get(socket_addr) {
            if bytes_sent > 0 {
                client.stats.record_sent(bytes_sent);
            }
            if bytes_received > 0 {
                client.stats.record_received(bytes_received);
            }
        }
    }

    /// Get statistics for all clients.
    pub async fn get_all_stats(&self) -> Vec<(String, Ipv4Addr, ClientStatsSnapshot)> {
        let clients = self.clients.read().await;
        clients
            .values()
            .map(|c| (c.id.clone(), c.vpn_ip, c.stats.snapshot()))
            .collect()
    }

    /// Get the number of connected clients.
    pub async fn client_count(&self) -> usize {
        let clients = self.clients.read().await;
        clients.len()
    }

    /// Check if client-to-client routing is allowed.
    pub fn allows_client_routing(&self) -> bool {
        self.allow_client_routing
    }

    /// Get list of all connected client IPs (for routing).
    pub async fn get_client_ips(&self) -> Vec<Ipv4Addr> {
        let ip_map = self.ip_to_socket.read().await;
        ip_map.keys().copied().collect()
    }

    /// Remove idle clients (no activity for specified duration).
    pub async fn remove_idle_clients(&self, max_idle: std::time::Duration) -> Vec<SocketAddr> {
        let idle_clients: Vec<SocketAddr> = {
            let clients = self.clients.read().await;
            clients
                .iter()
                .filter(|(_, c)| c.idle_time() > max_idle)
                .map(|(&addr, _)| addr)
                .collect()
        };

        for addr in &idle_clients {
            warn!("Removing idle client: {}", addr);
            self.unregister_client(*addr).await;
        }

        idle_clients
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_pool_creation() {
        let pool = IpPool::from_cidr("10.8.0.0/24", "10.8.0.1").unwrap();
        assert_eq!(pool.available_count(), 253); // 254 hosts - 1 gateway
        assert_eq!(pool.assigned_count(), 0);
    }

    #[test]
    fn test_ip_pool_allocation() {
        let mut pool = IpPool::from_cidr("10.8.0.0/30", "10.8.0.1").unwrap();
        // /30 has 2 usable hosts, minus gateway = 1
        assert_eq!(pool.available_count(), 1);

        let ip = pool.allocate("client1").unwrap();
        assert_eq!(ip, Ipv4Addr::new(10, 8, 0, 2));
        assert_eq!(pool.available_count(), 0);
        assert_eq!(pool.assigned_count(), 1);

        // Pool should be exhausted
        assert!(pool.allocate("client2").is_none());

        // Release and reallocate
        pool.release(ip);
        assert_eq!(pool.available_count(), 1);

        let ip2 = pool.allocate("client2").unwrap();
        assert_eq!(ip2, Ipv4Addr::new(10, 8, 0, 2));
    }

    #[test]
    fn test_ip_pool_same_client() {
        let mut pool = IpPool::from_cidr("10.8.0.0/28", "10.8.0.1").unwrap();

        let ip1 = pool.allocate("client1").unwrap();
        let ip2 = pool.allocate("client1").unwrap();

        // Same client should get same IP
        assert_eq!(ip1, ip2);
        assert_eq!(pool.assigned_count(), 1);
    }

    #[test]
    fn test_client_stats() {
        let stats = ClientStats::new();

        stats.record_sent(100);
        stats.record_sent(50);
        stats.record_received(200);

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.bytes_sent, 150);
        assert_eq!(snapshot.packets_sent, 2);
        assert_eq!(snapshot.bytes_received, 200);
        assert_eq!(snapshot.packets_received, 1);
    }
}
