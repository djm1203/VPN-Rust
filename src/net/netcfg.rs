//! Cross-platform host network configuration for a VPN session.
//!
//! [`NetConfigurator`] is the seam (decision D-16) that configures host routing
//! and NAT for a session and reverts it on teardown/drop. The Linux
//! implementation wraps [`crate::net::route`] (which shells out to
//! `ip`/`iptables`/`sysctl`); other platforms currently get a no-op that asks
//! the operator to configure routing manually (macOS/Windows impls are backlog
//! items B-022).

use anyhow::Result;
use tracing::warn;

/// Configures host networking (routes / NAT / forwarding) for a VPN session and
/// reverts all changes on [`NetConfigurator::teardown`] (also on drop).
pub trait NetConfigurator {
    /// Server-side: enable forwarding and NAT so peers can reach the internet
    /// through `out_interface` (auto-detected when `None`).
    fn setup_server(&mut self, vpn_subnet: &str, out_interface: Option<&str>) -> Result<()>;

    /// Client-side: route `vpn_subnet` through the tunnel interface.
    fn setup_client(&mut self, vpn_subnet: &str, tun_interface: &str) -> Result<()>;

    /// Revert everything this configurator applied. Idempotent.
    fn teardown(&mut self);
}

/// Return the platform's default network configurator.
pub fn platform_default() -> Box<dyn NetConfigurator + Send> {
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxNetConfigurator::default())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Box::new(NoopNetConfigurator)
    }
}

/// A configurator that does nothing but warn — used on platforms without an
/// implementation yet.
#[cfg_attr(target_os = "linux", allow(dead_code))]
struct NoopNetConfigurator;

impl NetConfigurator for NoopNetConfigurator {
    fn setup_server(&mut self, _vpn_subnet: &str, _out_interface: Option<&str>) -> Result<()> {
        warn!("automatic server NAT/forwarding is not implemented on this platform — configure it manually");
        Ok(())
    }

    fn setup_client(&mut self, _vpn_subnet: &str, _tun_interface: &str) -> Result<()> {
        warn!(
            "automatic client routing is not implemented on this platform — configure it manually"
        );
        Ok(())
    }

    fn teardown(&mut self) {}
}

#[cfg(target_os = "linux")]
mod linux {
    use super::NetConfigurator;
    use crate::net::route;
    use anyhow::Result;
    use tracing::info;

    /// Linux configurator backed by `ip`/`iptables`/`sysctl` (via
    /// [`crate::net::route`]). Reverts NAT rules and routes on teardown/drop.
    #[derive(Default)]
    pub struct LinuxNetConfigurator {
        /// `(vpn_subnet, out_interface)` if server NAT was applied.
        nat: Option<(String, String)>,
        /// `vpn_subnet` if a client route was added.
        client_route: Option<String>,
    }

    impl NetConfigurator for LinuxNetConfigurator {
        fn setup_server(&mut self, vpn_subnet: &str, out_interface: Option<&str>) -> Result<()> {
            let out = match out_interface {
                Some(i) => i.to_string(),
                None => route::get_default_interface()?,
            };
            route::enable_ip_forwarding()?;
            route::setup_nat(vpn_subnet, &out)?;
            info!("server networking configured (NAT for {vpn_subnet} via {out})");
            self.nat = Some((vpn_subnet.to_string(), out));
            Ok(())
        }

        fn setup_client(&mut self, vpn_subnet: &str, tun_interface: &str) -> Result<()> {
            route::add_route(vpn_subnet, tun_interface)?;
            info!("client route to {vpn_subnet} via {tun_interface} configured");
            self.client_route = Some(vpn_subnet.to_string());
            Ok(())
        }

        fn teardown(&mut self) {
            if let Some((subnet, out)) = self.nat.take() {
                let _ = route::cleanup_nat(&subnet, &out);
            }
            if let Some(subnet) = self.client_route.take() {
                let _ = route::remove_route(&subnet);
            }
        }
    }

    impl Drop for LinuxNetConfigurator {
        fn drop(&mut self) {
            self.teardown();
        }
    }
}
