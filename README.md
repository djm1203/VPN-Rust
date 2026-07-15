# VPN-Rust

A personal, point-to-point VPN written in Rust. It tunnels IP packets between
machines you own — one Linux server and Linux/macOS/Windows clients — over
**QUIC/UDP** (via [`quinn`](https://crates.io/crates/quinn), TLS 1.3), with peers
authenticated by **pinned self-signed certificates** (no CA/PKI). A live terminal
dashboard is the primary way to run and observe it.

This started as a learning project and is being taken toward production-ready
personal software. See [docs/QUICKSTART.md](docs/QUICKSTART.md) to get running,
and the [architecture](docs/architecture/ARCHITECTURE.md),
[wire-protocol spec](docs/standards/WIRE_PROTOCOL.md), and
[threat model](docs/operations/THREAT_MODEL.md) for the design.

## Status

The full core is implemented: cross-platform TUN, QUIC transport with a versioned
control handshake, pinned-certificate auth, Linux NAT/routing, the TUI dashboard,
a Prometheus metrics endpoint, and validated config files. The crate builds on
Linux, macOS, and Windows, and the test suite (54 tests) is green.

**Not yet verified end-to-end on real hardware:** the full packet path requires
root for the TUN device, and the Windows (wintun) / macOS (utun) clients have not
been run on live hosts. Treat runtime behaviour as unproven until you test it.

## Features

- QUIC/UDP transport (unreliable datagrams for tunneled packets; a reliable
  control stream for the handshake) — no TCP-over-TCP meltdown.
- Cross-platform TUN via [`tun-rs`](https://crates.io/crates/tun-rs) (Linux,
  macOS `utun`, Windows `wintun`).
- Pinned self-signed keypairs with trust-on-first-use: the client pins the
  server certificate and both ends print a SHA-256 fingerprint for out-of-band
  verification. No CA, no PKI.
- Versioned control handshake negotiating MTU and keepalive.
- Automatic client reconnect with exponential backoff; QUIC keep-alive and idle
  timeout.
- Linux server NAT and client routing, reverted automatically on exit
  (macOS/Windows network config is manual for now).
- Live TUI control dashboard (`--tui`): connection state, TX/RX throughput
  sparklines, RTT gauge, byte/packet counters, and a filterable log viewer.
- Optional Prometheus metrics endpoint (`--metrics-addr`).
- TOML configuration files with actionable validation (`--config`).

## Requirements

- Rust (latest stable) and Cargo.
- Root / `CAP_NET_ADMIN` on Linux (and elevated privileges on macOS/Windows) to
  create the TUN device.
- Windows clients additionally need `wintun.dll` next to the binary
  (see [wintun.net](https://www.wintun.net)).

The Linux host is the server; Linux, macOS, and Windows machines can be clients.

## Build and test

```bash
cargo build --release        # binary at target/release/vpn-rust
cargo test                   # unit + loopback integration tests (no root needed)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Quick usage

The commands below are a summary; see [docs/QUICKSTART.md](docs/QUICKSTART.md)
for the full walkthrough including fingerprint verification and per-OS notes.

```bash
# 1. Generate the server identity (prints a sha256: fingerprint to verify later).
vpn-rust keygen --server-name vpn.example.com \
  --cert certs/server-cert.der --key certs/server-key.der

# 2. Run the server (Linux, needs root for the TUN device). It also generates
#    its identity on first run if the cert/key are absent.
sudo ./target/release/vpn-rust server --bind 0.0.0.0 --port 4433 --tui

# 3. Copy certs/server-cert.der to the client, then connect and pin it.
sudo ./target/release/vpn-rust client \
  --server vpn.example.com --server-cert server-cert.der \
  --server-name vpn.example.com --tui
```

Useful flags (run `vpn-rust <command> --help` for all of them):

- `--tui` — run the live dashboard (keys: `q` quit, `f` cycle log filter,
  arrows/PageUp/PageDown scroll, `?` help). Mutually exclusive with `--daemon`.
- `--daemon` — headless with plain (non-ANSI) logging, suited to systemd/journald.
- `--metrics-addr 127.0.0.1:9095` — serve Prometheus metrics (off by default;
  bind to loopback on a VPN host).
- `--config path.toml` — load and validate a TOML config; its values fill in the
  addressing and paths. See [config.example.toml](config.example.toml).

Server defaults: bind `0.0.0.0:4433`, TUN `rustvpn0` at `10.8.0.1/30`, inner MTU
`1300`. Client defaults: TUN `rustvpn1` at `10.8.0.2/30`.

## Deployment

A systemd unit and a tagged release workflow live under
[packaging/](packaging/README.md).

## Documentation

- [Quickstart](docs/QUICKSTART.md)
- [Architecture](docs/architecture/ARCHITECTURE.md)
- [Wire protocol](docs/standards/WIRE_PROTOCOL.md)
- [Threat model](docs/operations/THREAT_MODEL.md)
- [Versioning policy](docs/standards/VERSIONING.md)

## License

Personal project; see the repository for license details.
