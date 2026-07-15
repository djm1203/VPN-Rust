# systemd unit — VPN-Rust server (Linux)

Installs `vpn-rust server` as a supervised system service using
[`vpn-rust-server.service`](./vpn-rust-server.service).

## Install

```bash
# 1. Install the binary (from a release archive or `cargo build --release`).
sudo install -m 0755 vpn-rust /usr/local/bin/vpn-rust

# 2. Create a dedicated, unprivileged service account (no login, no home).
#    Skip this if you choose to run the unit as root instead.
sudo useradd --system --no-create-home --shell /usr/sbin/nologin vpn-rust

# 3. Create the working/cert directory and hand it to the service user.
sudo mkdir -p /etc/vpn-rust/certs
sudo chown -R vpn-rust:vpn-rust /etc/vpn-rust

# 4. (Optional) Pre-generate the server identity. The server will also generate
#    these on first run if they are missing.
sudo -u vpn-rust /usr/local/bin/vpn-rust keygen \
    --server-name vpn.example.com \
    --cert /etc/vpn-rust/certs/server-cert.der \
    --key  /etc/vpn-rust/certs/server-key.der

# 5. Install the unit.
sudo cp vpn-rust-server.service /etc/systemd/system/vpn-rust-server.service

# 6. Edit ExecStart flags (bind/port, --server-name, cert paths) to match your setup.
sudo systemctl edit --full vpn-rust-server.service   # or edit the file directly

# 7. Reload systemd and enable + start the service.
sudo systemctl daemon-reload
sudo systemctl enable --now vpn-rust-server.service
```

## Operate

```bash
# Status
systemctl status vpn-rust-server

# Follow logs
journalctl -u vpn-rust-server -f

# Restart after editing flags
sudo systemctl restart vpn-rust-server
```

## Notes

- **Capabilities.** The unit runs as the unprivileged `vpn-rust` user and grants
  only `CAP_NET_ADMIN` (needed to create the TUN device and install NAT/routing
  rules) via ambient capabilities. `NoNewPrivileges` is deliberately unset because
  it would block that grant.
- **Running as root instead.** If the capability setup is troublesome on your
  distro, set `User=root` in the unit (and drop the `AmbientCapabilities` /
  `CapabilityBoundingSet` lines). Simpler, less isolated.
- **IP forwarding / NAT.** The server enables NAT itself, but the kernel must permit
  forwarding. Enable it persistently:
  ```bash
  echo 'net.ipv4.ip_forward=1' | sudo tee /etc/sysctl.d/99-vpn-rust.conf
  sudo sysctl --system
  ```
- **Firewall.** Open the UDP listen port (default `4433/udp`).
