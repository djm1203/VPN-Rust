//! Network modules for VPN communication.
//!
//! - [`device`] - Cross-platform async TUN device abstraction (`TunDevice`)
//! - [`route`] - Linux route + NAT configuration (server-side; Unix only)
//! - [`security`] - Linux kill switch / DNS & IPv6 leak prevention (Unix only)
//!
//! The `route` and `security` modules shell out to `ip`/`iptables`/`sysctl` and
//! are Linux-specific; they will be wrapped by a cross-platform
//! `NetConfigurator` abstraction (backlog B-020).

pub mod device;
pub mod netcfg;

#[cfg(unix)]
pub mod route;
#[cfg(unix)]
pub mod security;
