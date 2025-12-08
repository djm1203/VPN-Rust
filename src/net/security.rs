//! Security features for the VPN client.
//!
//! This module provides security features to prevent traffic leaks:
//! - Kill switch: Blocks all non-VPN traffic when enabled
//! - DNS leak prevention: Ensures DNS queries go through the VPN
//! - IPv6 leak prevention: Blocks IPv6 traffic that could bypass the tunnel

use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::process::Command;

// =============================================================================
// Kill Switch
// =============================================================================

/// Kill switch state tracking.
#[derive(Debug, Default)]
pub struct KillSwitch {
    /// Whether the kill switch is currently active.
    active: bool,
    /// Original iptables rules backup (for restoration).
    backup_rules: Option<String>,
}

impl KillSwitch {
    /// Create a new kill switch instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the kill switch is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Enable the kill switch.
    ///
    /// This will:
    /// 1. Backup current iptables rules
    /// 2. Block all outgoing traffic except through the VPN interface
    /// 3. Allow traffic to the VPN server IP
    ///
    /// # Arguments
    ///
    /// * `vpn_interface` - The TUN interface name (e.g., "rustvpn1")
    /// * `vpn_server_ip` - The VPN server's public IP address
    /// * `vpn_server_port` - The VPN server's port
    pub fn enable(
        &mut self,
        vpn_interface: &str,
        vpn_server_ip: &str,
        vpn_server_port: u16,
    ) -> Result<()> {
        if self.active {
            warn!("Kill switch is already active");
            return Ok(());
        }

        info!("Enabling kill switch");

        // Backup current rules
        self.backup_rules = Some(backup_iptables_rules()?);
        debug!("Backed up iptables rules");

        // Create kill switch rules
        apply_kill_switch_rules(vpn_interface, vpn_server_ip, vpn_server_port)?;

        self.active = true;
        info!("Kill switch enabled - all non-VPN traffic blocked");

        Ok(())
    }

    /// Disable the kill switch and restore original rules.
    pub fn disable(&mut self) -> Result<()> {
        if !self.active {
            warn!("Kill switch is not active");
            return Ok(());
        }

        info!("Disabling kill switch");

        // Remove kill switch rules
        remove_kill_switch_rules()?;

        // Restore original rules if we have them
        if let Some(rules) = self.backup_rules.take() {
            restore_iptables_rules(&rules)?;
            debug!("Restored original iptables rules");
        }

        self.active = false;
        info!("Kill switch disabled - normal traffic restored");

        Ok(())
    }
}

impl Drop for KillSwitch {
    fn drop(&mut self) {
        if self.active {
            if let Err(e) = self.disable() {
                warn!("Failed to disable kill switch on drop: {}", e);
            }
        }
    }
}

/// Backup current iptables rules.
fn backup_iptables_rules() -> Result<String> {
    let output = Command::new("iptables-save")
        .output()
        .context("Failed to run iptables-save")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("iptables-save failed: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Restore iptables rules from backup.
fn restore_iptables_rules(rules: &str) -> Result<()> {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new("iptables-restore")
        .stdin(Stdio::piped())
        .spawn()
        .context("Failed to run iptables-restore")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(rules.as_bytes())
            .context("Failed to write rules to iptables-restore")?;
    }

    let status = child.wait().context("Failed to wait for iptables-restore")?;
    if !status.success() {
        anyhow::bail!("iptables-restore failed");
    }

    Ok(())
}

/// Apply kill switch iptables rules.
fn apply_kill_switch_rules(
    vpn_interface: &str,
    vpn_server_ip: &str,
    vpn_server_port: u16,
) -> Result<()> {
    // Create a custom chain for kill switch rules
    run_iptables(&["-N", "VPN_KILLSWITCH"])?;

    // Allow loopback
    run_iptables(&["-A", "VPN_KILLSWITCH", "-o", "lo", "-j", "ACCEPT"])?;

    // Allow established connections
    run_iptables(&[
        "-A",
        "VPN_KILLSWITCH",
        "-m",
        "conntrack",
        "--ctstate",
        "ESTABLISHED,RELATED",
        "-j",
        "ACCEPT",
    ])?;

    // Allow traffic to VPN server (so we can establish the tunnel)
    run_iptables(&[
        "-A",
        "VPN_KILLSWITCH",
        "-d",
        vpn_server_ip,
        "-p",
        "tcp",
        "--dport",
        &vpn_server_port.to_string(),
        "-j",
        "ACCEPT",
    ])?;

    // Allow all traffic through VPN interface
    run_iptables(&[
        "-A",
        "VPN_KILLSWITCH",
        "-o",
        vpn_interface,
        "-j",
        "ACCEPT",
    ])?;

    // Allow DHCP (for local network)
    run_iptables(&[
        "-A",
        "VPN_KILLSWITCH",
        "-p",
        "udp",
        "--dport",
        "67:68",
        "-j",
        "ACCEPT",
    ])?;

    // Drop everything else
    run_iptables(&["-A", "VPN_KILLSWITCH", "-j", "DROP"])?;

    // Insert the chain into OUTPUT
    run_iptables(&["-I", "OUTPUT", "1", "-j", "VPN_KILLSWITCH"])?;

    debug!("Applied kill switch iptables rules");
    Ok(())
}

/// Remove kill switch iptables rules.
fn remove_kill_switch_rules() -> Result<()> {
    // Remove from OUTPUT chain
    let _ = run_iptables(&["-D", "OUTPUT", "-j", "VPN_KILLSWITCH"]);

    // Flush and delete the chain
    let _ = run_iptables(&["-F", "VPN_KILLSWITCH"]);
    let _ = run_iptables(&["-X", "VPN_KILLSWITCH"]);

    debug!("Removed kill switch iptables rules");
    Ok(())
}

/// Run an iptables command.
fn run_iptables(args: &[&str]) -> Result<()> {
    let output = Command::new("iptables")
        .args(args)
        .output()
        .with_context(|| format!("Failed to run iptables {:?}", args))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("iptables {:?} failed: {}", args, stderr);
    }

    Ok(())
}

// =============================================================================
// DNS Leak Prevention
// =============================================================================

/// DNS leak prevention state.
#[derive(Debug, Default)]
pub struct DnsLeakPrevention {
    /// Whether DNS leak prevention is active.
    active: bool,
    /// Original resolv.conf content.
    original_resolv_conf: Option<String>,
}

impl DnsLeakPrevention {
    /// Create a new DNS leak prevention instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if DNS leak prevention is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Enable DNS leak prevention.
    ///
    /// This will:
    /// 1. Backup /etc/resolv.conf
    /// 2. Configure DNS to use the VPN's DNS server
    /// 3. Block DNS traffic that doesn't go through the VPN
    ///
    /// # Arguments
    ///
    /// * `vpn_dns_server` - The DNS server IP to use (through the VPN)
    /// * `vpn_interface` - The VPN interface name
    pub fn enable(&mut self, vpn_dns_server: &str, vpn_interface: &str) -> Result<()> {
        if self.active {
            warn!("DNS leak prevention is already active");
            return Ok(());
        }

        info!("Enabling DNS leak prevention");

        // Backup resolv.conf
        self.original_resolv_conf = std::fs::read_to_string("/etc/resolv.conf").ok();

        // Write new resolv.conf with VPN DNS
        let new_resolv = format!(
            "# VPN-Rust DNS configuration\nnameserver {}\n",
            vpn_dns_server
        );
        std::fs::write("/etc/resolv.conf", new_resolv)
            .context("Failed to write /etc/resolv.conf")?;

        // Block DNS traffic not going through VPN interface
        apply_dns_leak_rules(vpn_interface)?;

        self.active = true;
        info!(
            "DNS leak prevention enabled - using DNS server {}",
            vpn_dns_server
        );

        Ok(())
    }

    /// Disable DNS leak prevention and restore original settings.
    pub fn disable(&mut self) -> Result<()> {
        if !self.active {
            warn!("DNS leak prevention is not active");
            return Ok(());
        }

        info!("Disabling DNS leak prevention");

        // Remove DNS blocking rules
        remove_dns_leak_rules()?;

        // Restore original resolv.conf
        if let Some(original) = self.original_resolv_conf.take() {
            std::fs::write("/etc/resolv.conf", original)
                .context("Failed to restore /etc/resolv.conf")?;
            debug!("Restored original resolv.conf");
        }

        self.active = false;
        info!("DNS leak prevention disabled");

        Ok(())
    }
}

impl Drop for DnsLeakPrevention {
    fn drop(&mut self) {
        if self.active {
            if let Err(e) = self.disable() {
                warn!("Failed to disable DNS leak prevention on drop: {}", e);
            }
        }
    }
}

/// Apply DNS leak prevention iptables rules.
fn apply_dns_leak_rules(vpn_interface: &str) -> Result<()> {
    // Create chain for DNS rules
    run_iptables(&["-N", "VPN_DNS"])?;

    // Allow DNS through VPN interface
    run_iptables(&[
        "-A",
        "VPN_DNS",
        "-o",
        vpn_interface,
        "-p",
        "udp",
        "--dport",
        "53",
        "-j",
        "ACCEPT",
    ])?;

    run_iptables(&[
        "-A",
        "VPN_DNS",
        "-o",
        vpn_interface,
        "-p",
        "tcp",
        "--dport",
        "53",
        "-j",
        "ACCEPT",
    ])?;

    // Allow DNS to localhost (for local DNS caching)
    run_iptables(&[
        "-A",
        "VPN_DNS",
        "-d",
        "127.0.0.1",
        "-p",
        "udp",
        "--dport",
        "53",
        "-j",
        "ACCEPT",
    ])?;

    // Block other DNS traffic
    run_iptables(&[
        "-A",
        "VPN_DNS",
        "-p",
        "udp",
        "--dport",
        "53",
        "-j",
        "DROP",
    ])?;

    run_iptables(&[
        "-A",
        "VPN_DNS",
        "-p",
        "tcp",
        "--dport",
        "53",
        "-j",
        "DROP",
    ])?;

    // Insert into OUTPUT chain
    run_iptables(&["-I", "OUTPUT", "1", "-j", "VPN_DNS"])?;

    debug!("Applied DNS leak prevention rules");
    Ok(())
}

/// Remove DNS leak prevention rules.
fn remove_dns_leak_rules() -> Result<()> {
    let _ = run_iptables(&["-D", "OUTPUT", "-j", "VPN_DNS"]);
    let _ = run_iptables(&["-F", "VPN_DNS"]);
    let _ = run_iptables(&["-X", "VPN_DNS"]);

    debug!("Removed DNS leak prevention rules");
    Ok(())
}

// =============================================================================
// IPv6 Leak Prevention
// =============================================================================

/// IPv6 leak prevention state.
#[derive(Debug, Default)]
pub struct Ipv6LeakPrevention {
    /// Whether IPv6 leak prevention is active.
    active: bool,
}

impl Ipv6LeakPrevention {
    /// Create a new IPv6 leak prevention instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if IPv6 leak prevention is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Enable IPv6 leak prevention by blocking all IPv6 traffic.
    pub fn enable(&mut self) -> Result<()> {
        if self.active {
            warn!("IPv6 leak prevention is already active");
            return Ok(());
        }

        info!("Enabling IPv6 leak prevention");

        apply_ipv6_block_rules()?;

        self.active = true;
        info!("IPv6 leak prevention enabled - all IPv6 traffic blocked");

        Ok(())
    }

    /// Disable IPv6 leak prevention.
    pub fn disable(&mut self) -> Result<()> {
        if !self.active {
            warn!("IPv6 leak prevention is not active");
            return Ok(());
        }

        info!("Disabling IPv6 leak prevention");

        remove_ipv6_block_rules()?;

        self.active = false;
        info!("IPv6 leak prevention disabled");

        Ok(())
    }
}

impl Drop for Ipv6LeakPrevention {
    fn drop(&mut self) {
        if self.active {
            if let Err(e) = self.disable() {
                warn!("Failed to disable IPv6 leak prevention on drop: {}", e);
            }
        }
    }
}

/// Run an ip6tables command.
fn run_ip6tables(args: &[&str]) -> Result<()> {
    let output = Command::new("ip6tables")
        .args(args)
        .output()
        .with_context(|| format!("Failed to run ip6tables {:?}", args))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ip6tables {:?} failed: {}", args, stderr);
    }

    Ok(())
}

/// Apply IPv6 blocking rules.
fn apply_ipv6_block_rules() -> Result<()> {
    // Create chain for IPv6 blocking
    run_ip6tables(&["-N", "VPN_IPV6_BLOCK"])?;

    // Allow loopback
    run_ip6tables(&["-A", "VPN_IPV6_BLOCK", "-o", "lo", "-j", "ACCEPT"])?;

    // Block all other IPv6 traffic
    run_ip6tables(&["-A", "VPN_IPV6_BLOCK", "-j", "DROP"])?;

    // Insert into OUTPUT chain
    run_ip6tables(&["-I", "OUTPUT", "1", "-j", "VPN_IPV6_BLOCK"])?;

    debug!("Applied IPv6 blocking rules");
    Ok(())
}

/// Remove IPv6 blocking rules.
fn remove_ipv6_block_rules() -> Result<()> {
    let _ = run_ip6tables(&["-D", "OUTPUT", "-j", "VPN_IPV6_BLOCK"]);
    let _ = run_ip6tables(&["-F", "VPN_IPV6_BLOCK"]);
    let _ = run_ip6tables(&["-X", "VPN_IPV6_BLOCK"]);

    debug!("Removed IPv6 blocking rules");
    Ok(())
}

// =============================================================================
// Combined Security Manager
// =============================================================================

/// Combined security features manager.
///
/// This struct manages all security features together and ensures proper
/// cleanup when dropped.
#[derive(Debug, Default)]
pub struct SecurityManager {
    /// Kill switch instance.
    pub kill_switch: KillSwitch,
    /// DNS leak prevention instance.
    pub dns_leak_prevention: DnsLeakPrevention,
    /// IPv6 leak prevention instance.
    pub ipv6_leak_prevention: Ipv6LeakPrevention,
}

impl SecurityManager {
    /// Create a new security manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable all security features.
    ///
    /// # Arguments
    ///
    /// * `vpn_interface` - The VPN TUN interface name
    /// * `vpn_server_ip` - The VPN server's public IP
    /// * `vpn_server_port` - The VPN server's port
    /// * `vpn_dns_server` - The DNS server to use through the VPN
    pub fn enable_all(
        &mut self,
        vpn_interface: &str,
        vpn_server_ip: &str,
        vpn_server_port: u16,
        vpn_dns_server: &str,
    ) -> Result<()> {
        // Enable IPv6 blocking first
        self.ipv6_leak_prevention.enable()?;

        // Enable DNS leak prevention
        self.dns_leak_prevention
            .enable(vpn_dns_server, vpn_interface)?;

        // Enable kill switch last
        self.kill_switch
            .enable(vpn_interface, vpn_server_ip, vpn_server_port)?;

        info!("All security features enabled");
        Ok(())
    }

    /// Disable all security features.
    pub fn disable_all(&mut self) -> Result<()> {
        // Disable in reverse order
        let _ = self.kill_switch.disable();
        let _ = self.dns_leak_prevention.disable();
        let _ = self.ipv6_leak_prevention.disable();

        info!("All security features disabled");
        Ok(())
    }
}
