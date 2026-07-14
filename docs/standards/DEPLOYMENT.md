---
title: "DEPLOYMENT"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Deployment
author: Derek Martinez
---

# Deployment — VPN-Rust

VPN-Rust is a **Linux-only** VPN client and server. This document covers deploying
both roles on a Linux host. There is no container image, systemd unit, or managed
platform yet (tracked in the backlog); deployment today is a manual build-and-run.

> **Platform gate:** the codebase uses Linux-specific TUN plumbing
> (`AsyncFd`, `from_raw_fd`) and **does not compile on Windows or macOS**. Do not
> attempt to deploy on those platforms — cross-platform support is future work.

## Environments

| Environment | Host | Purpose |
|-------------|------|---------|
| Development | Local Linux box / VM | Build, run client+server on 127.0.0.1, manual testing |
| Production (personal) | Linux VPS or LAN host with a public/routable IP | Serve real tunnel traffic |

There is no staging tier and no CI-driven deploy pipeline. GitHub Actions currently
builds and tests only.

## Prerequisites

- **Rust stable** toolchain (install via rustup). Edition 2021.
- **Linux with the TUN module available.** Load it if needed:
  ```bash
  sudo modprobe tun
  lsmod | grep tun        # verify
  ```
- **Root or `CAP_NET_ADMIN`.** Creating TUN interfaces, running `ip`, `sysctl`, and
  `iptables` all require elevated privileges.
- **Certificates.** Generate self-signed dev certs into `certs/`:
  ```bash
  ./gen_certs.sh
  ```
  This produces `certs/server.crt`, `certs/server.key`, `certs/ca.crt`, and
  (for mTLS) `certs/client.crt` / `certs/client.key`.

## Build for Deployment

```bash
# Release build (both the unified vpn-rust binary and legacy server/client bins)
cargo build --release

# Artifacts land in:
#   target/release/vpn-rust        (unified: `vpn-rust server` / `vpn-rust client`)
#   target/release/server          (legacy)
#   target/release/client          (legacy)
```

## Networking Requirements

- **Open inbound TCP 4433** on the server host (or whatever `port` you configure).
  Adjust the host firewall / cloud security group accordingly.
- **IP forwarding** must be enabled for clients to reach the internet through the
  server. The server enables this automatically at startup
  (`src/net/route.rs::enable_ip_forwarding` runs `sysctl -w net.ipv4.ip_forward=1`),
  but you can pre-set it persistently:
  ```bash
  echo 'net.ipv4.ip_forward = 1' | sudo tee /etc/sysctl.d/99-vpn-rust.conf
  sudo sysctl --system
  ```
- **NAT / MASQUERADE.** When `enable_nat = true` (default), the server installs an
  `iptables` POSTROUTING MASQUERADE rule for the VPN subnet on the outbound
  interface (`src/net/route.rs`). If `nat_interface` is unset it auto-detects; set it
  explicitly (e.g. `eth0`) if detection is wrong.

## Server Startup

1. Prepare a config (see `config.example.toml`); at minimum set `[server] bind` to a
   routable address and confirm `subnet` / `server_ip`.
2. Run as root:
   ```bash
   sudo ./target/release/vpn-rust server --config /etc/vpn-rust/config.toml
   # add -v / RUST_LOG=debug for verbose logs
   ```
   On startup the server: loads TLS certs, binds TCP:4433, creates the `rustvpn0`
   TUN interface (`10.8.0.1/30`, MTU 1500), enables IP forwarding, and sets up NAT.

## Client Startup

1. Point `[client] server` / `port` / `hostname` at your server. `hostname` must
   match the server certificate name (dev certs use `localhost`).
2. Run as root:
   ```bash
   sudo ./target/release/vpn-rust client --config /etc/vpn-rust/config.toml
   ```
   The client connects over TLS, creates `rustvpn1` (`10.8.0.2/30`, MTU 1500), and
   begins tunneling. It sends keepalives every 10 s and auto-reconnects (backoff
   1 s → 30 s) unless `no_reconnect = true`.

### Verify

```bash
ping 10.8.0.1                 # from client → server tunnel IP
sudo tcpdump -i rustvpn0 -n   # observe tunneled traffic on the server
```

## Teardown / Rollback

```bash
# Stop the processes (Ctrl-C), then clean up interfaces and leftover rules:
./cleanup_vpn.sh              # deletes rustvpn0 / rustvpn1

# Manual fallback:
sudo ip link delete rustvpn0 2>/dev/null
sudo ip link delete rustvpn1 2>/dev/null
```

Because there is no orchestration layer, "rollback" is: stop the process, run
`cleanup_vpn.sh`, check out the previous known-good commit, `cargo build --release`,
and restart. Keep the previous release binary around for a fast revert.

## Security Notes (deployment-critical)

- **Replace the self-signed dev certificates.** `gen_certs.sh` output is for
  development only. Use a real CA / properly managed certs before exposing the
  server. Enable **mTLS** (client certificates) for any non-loopback deployment.
- **Restrict the bind address.** The code default is `127.0.0.1`
  (`DEFAULT_SERVER_ADDR`); only widen to `0.0.0.0` deliberately, and firewall
  port 4433 to known clients where possible.
- **Protect key files.** `certs/*.key` must be `chmod 600` and never committed
  (`.gitignore` already excludes keys). Rotate on a schedule.
- **Least privilege.** Prefer granting `CAP_NET_ADMIN` to the binary over running
  the whole process as root where your environment allows it.

## Not Yet Available (backlog)

- Container image / Dockerfile.
- systemd service unit for supervised, boot-time startup.
- Windows and macOS builds.
- Automated deploy pipeline / staging environment.
