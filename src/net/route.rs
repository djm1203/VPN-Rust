//! Route management for VPN traffic.
//!
//! This module provides functions for configuring IP routing, NAT, and
//! IP forwarding required for VPN operation.
//!
//! # Server Requirements
//!
//! The server needs to:
//! - Enable IP forwarding (`net.ipv4.ip_forward = 1`)
//! - Set up NAT (masquerading) for outbound traffic
//!
//! # Client Requirements
//!
//! The client needs to:
//! - Add a route to the VPN subnet through the TUN interface

use anyhow::{Context, Result};
use std::process::Command;
use tracing::{debug, info, warn};

/// Enables IPv4 forwarding on the system.
///
/// This is required on the VPN server to forward packets between
/// the TUN interface and the physical network interface.
///
/// # Note
///
/// This change is not persistent across reboots. For permanent
/// configuration, modify `/etc/sysctl.conf`.
pub fn enable_ip_forwarding() -> Result<()> {
    info!("Enabling IPv4 forwarding");

    let output = Command::new("sysctl")
        .args(["-w", "net.ipv4.ip_forward=1"])
        .output()
        .context("Failed to execute sysctl")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed to enable IP forwarding: {}", error));
    }

    info!("IPv4 forwarding enabled");
    Ok(())
}

/// Checks if IPv4 forwarding is currently enabled.
pub fn is_ip_forwarding_enabled() -> Result<bool> {
    let output = Command::new("sysctl")
        .args(["net.ipv4.ip_forward"])
        .output()
        .context("Failed to check IP forwarding status")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.contains("= 1"))
}

/// Sets up NAT (masquerading) for VPN traffic.
///
/// This allows VPN clients to access the internet through the server
/// by masquerading their traffic with the server's IP address.
///
/// # Arguments
///
/// * `vpn_subnet` - The VPN subnet in CIDR notation (e.g., "10.8.0.0/30")
/// * `outbound_interface` - The physical network interface for outbound traffic (e.g., "eth0")
pub fn setup_nat(vpn_subnet: &str, outbound_interface: &str) -> Result<()> {
    info!(
        "Setting up NAT for {} via {}",
        vpn_subnet, outbound_interface
    );

    // Add MASQUERADE rule for VPN traffic
    let output = Command::new("iptables")
        .args([
            "-t",
            "nat",
            "-A",
            "POSTROUTING",
            "-s",
            vpn_subnet,
            "-o",
            outbound_interface,
            "-j",
            "MASQUERADE",
        ])
        .output()
        .context("Failed to execute iptables")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        // Check if rule already exists
        if error.contains("already exists") {
            debug!("NAT rule already exists");
            return Ok(());
        }
        return Err(anyhow::anyhow!("Failed to add NAT rule: {}", error));
    }

    // Allow forwarding for established connections
    let output = Command::new("iptables")
        .args(["-A", "FORWARD", "-s", vpn_subnet, "-j", "ACCEPT"])
        .output()
        .context("Failed to add forward rule")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        warn!("Failed to add FORWARD rule (may already exist): {}", error);
    }

    // Allow return traffic
    let output = Command::new("iptables")
        .args([
            "-A",
            "FORWARD",
            "-d",
            vpn_subnet,
            "-m",
            "state",
            "--state",
            "RELATED,ESTABLISHED",
            "-j",
            "ACCEPT",
        ])
        .output()
        .context("Failed to add return forward rule")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        warn!(
            "Failed to add return FORWARD rule (may already exist): {}",
            error
        );
    }

    info!("NAT configured for VPN subnet {}", vpn_subnet);
    Ok(())
}

/// Removes NAT rules for VPN traffic.
///
/// # Arguments
///
/// * `vpn_subnet` - The VPN subnet in CIDR notation
/// * `outbound_interface` - The physical network interface
pub fn cleanup_nat(vpn_subnet: &str, outbound_interface: &str) -> Result<()> {
    info!("Cleaning up NAT rules for {}", vpn_subnet);

    // Remove MASQUERADE rule
    let _ = Command::new("iptables")
        .args([
            "-t",
            "nat",
            "-D",
            "POSTROUTING",
            "-s",
            vpn_subnet,
            "-o",
            outbound_interface,
            "-j",
            "MASQUERADE",
        ])
        .output();

    // Remove FORWARD rules
    let _ = Command::new("iptables")
        .args(["-D", "FORWARD", "-s", vpn_subnet, "-j", "ACCEPT"])
        .output();

    let _ = Command::new("iptables")
        .args([
            "-D",
            "FORWARD",
            "-d",
            vpn_subnet,
            "-m",
            "state",
            "--state",
            "RELATED,ESTABLISHED",
            "-j",
            "ACCEPT",
        ])
        .output();

    info!("NAT rules cleaned up");
    Ok(())
}

/// Adds a route to the VPN subnet through the specified interface.
///
/// # Arguments
///
/// * `subnet` - The destination subnet in CIDR notation
/// * `interface` - The interface to route through
pub fn add_route(subnet: &str, interface: &str) -> Result<()> {
    info!("Adding route to {} via {}", subnet, interface);

    let output = Command::new("ip")
        .args(["route", "add", subnet, "dev", interface])
        .output()
        .context("Failed to execute ip route add")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        // Ignore if route already exists
        if error.contains("File exists") {
            debug!("Route to {} already exists", subnet);
            return Ok(());
        }
        return Err(anyhow::anyhow!("Failed to add route: {}", error));
    }

    info!("Route to {} added via {}", subnet, interface);
    Ok(())
}

/// Adds a route to a specific host through the VPN gateway.
///
/// # Arguments
///
/// * `host` - The destination host IP
/// * `gateway` - The VPN gateway IP
/// * `interface` - The VPN interface name
pub fn add_host_route(host: &str, gateway: &str, interface: &str) -> Result<()> {
    debug!(
        "Adding host route: {} via {} dev {}",
        host, gateway, interface
    );

    let output = Command::new("ip")
        .args([
            "route",
            "add",
            &format!("{}/32", host),
            "via",
            gateway,
            "dev",
            interface,
        ])
        .output()
        .context("Failed to execute ip route add")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        if error.contains("File exists") {
            debug!("Route to {} already exists", host);
            return Ok(());
        }
        return Err(anyhow::anyhow!("Failed to add host route: {}", error));
    }

    Ok(())
}

/// Removes a route to the specified subnet.
///
/// # Arguments
///
/// * `subnet` - The destination subnet in CIDR notation
pub fn remove_route(subnet: &str) -> Result<()> {
    debug!("Removing route to {}", subnet);

    let _ = Command::new("ip").args(["route", "del", subnet]).output();

    Ok(())
}

/// Gets the default network interface (the one with the default route).
pub fn get_default_interface() -> Result<String> {
    let output = Command::new("ip")
        .args(["route", "show", "default"])
        .output()
        .context("Failed to get default route")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse "default via X.X.X.X dev eth0 ..."
    for part in stdout.split_whitespace() {
        if part.starts_with("dev") {
            continue;
        }
        // The interface name comes after "dev"
        if let Some(prev_idx) = stdout.find("dev ") {
            let rest = &stdout[prev_idx + 4..];
            if let Some(interface) = rest.split_whitespace().next() {
                return Ok(interface.to_string());
            }
        }
    }

    Err(anyhow::anyhow!("Could not determine default interface"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_interface() {
        // This test may fail in CI environments without network
        if let Ok(interface) = get_default_interface() {
            assert!(!interface.is_empty());
        }
    }
}
