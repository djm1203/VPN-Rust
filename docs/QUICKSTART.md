---
title: "QUICKSTART"
project: vpn-rust
classification: high
created: 2026-07-14T00:00:00Z
updated: 2026-07-14T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Quickstart
author: Derek Martinez
---

# Quickstart — VPN-Rust

Stand up a **personal point-to-point QUIC VPN**: one Linux **server** and one
**client** (Linux/macOS/Windows), authenticated by a **pinned self-signed
certificate** — no CA, no PKI.

For the trust model see
[/docs/operations/THREAT_MODEL.md](/docs/operations/THREAT_MODEL.md); for the
on-the-wire details see
[/docs/standards/WIRE_PROTOCOL.md](/docs/standards/WIRE_PROTOCOL.md).

> **Matched build.** The protocol is pre-1.0 — build server and client from the
> **same release** (see WIRE_PROTOCOL §7 / backlog B-042).

---

## 1. Prerequisites

**All hosts:** a recent stable Rust toolchain (`cargo`, `rustc`) and the ability
to create a TUN device (requires elevated privileges).

Per OS:

- **Linux (server and client):** run as **root** or grant the binary
  `CAP_NET_ADMIN` to create the TUN interface and (on the server) apply NAT via
  `ip`/`iptables`/`sysctl`. The `tun` kernel module must be available
  (`lsmod | grep tun`).
- **Windows (client):** install **Wintun** — place `wintun.dll` next to the
  `vpn-rust.exe` binary — and run from an **Administrator** terminal.
- **macOS (client):** the OS provides `utun`; run with **root** (`sudo`).
  Note: automatic route/NAT setup is Linux-only today (D-19); on macOS/Windows
  the tunnel comes up but routing is a warn-noop, so add routes manually if
  needed.

The **server is Linux-only** (D-10). Ensure the server's UDP port (default
**4433**) is reachable through any firewall/NAT in front of it.

---

## 2. Build

On each host:

```bash
cargo build --release
# binary: ./target/release/vpn-rust  (vpn-rust.exe on Windows)
```

---

## 3. Generate the server identity

On the **server**, generate a self-signed certificate + private key. Pick a
`--server-name` the client will use to reach the server (this becomes the
certificate SAN; default `localhost`):

```bash
./target/release/vpn-rust keygen \
  --server-name vpn.example.net \
  --cert certs/server-cert.der \
  --key  certs/server-key.der
```

This writes DER files (private key `0600` on Unix) and prints the certificate
**fingerprint**, e.g.:

```
server certificate fingerprint: sha256:1a:2b:3c:...:ef
```

**Record that `sha256:` fingerprint** — you will verify it on the client. (If the
files already exist, the server reuses them; use `keygen --force` to regenerate
and re-pin.)

> `--server-name` must be a name the client can present. The generated SAN equals
> `--server-name`, and the client's `--server-name` must match it (D-19); a
> configurable SAN for arbitrary hostnames is future work. For a first local test,
> `localhost` works with the client connecting to `127.0.0.1`.

---

## 4. Run the server

```bash
sudo ./target/release/vpn-rust server \
  --bind 0.0.0.0 \
  --port 4433 \
  --server-name vpn.example.net \
  --cert certs/server-cert.der \
  --key  certs/server-key.der
```

Useful defaults (override as needed): `--tun-name rustvpn0`, `--tun-ip 10.8.0.1`,
`--prefix 30`, `--mtu 1300`, `--nat-interface <auto-detected>`. The server logs
its listening address and the certificate fingerprint again — confirm it matches
what you recorded.

---

## 5. Pin the server certificate on the client

Copy **only the public certificate** (`certs/server-cert.der`) to the client —
never the key. Then **verify the fingerprint out of band** (compare the server's
printed `sha256:…` against a value you trust — read it over the phone, an existing
SSH session, etc.). This out-of-band check is what makes trust-on-first-use safe.

```bash
# on the client, e.g.
scp server:~/VPN-Rust/certs/server-cert.der ./server-cert.der
```

---

## 6. Run the client

Point the client at the server host and the pinned certificate. `--server-name`
**must match** the certificate SAN from step 3:

```bash
sudo ./target/release/vpn-rust client \
  --server vpn.example.net \
  --port 4433 \
  --server-name vpn.example.net \
  --server-cert server-cert.der
```

(For a local loopback test: `--server 127.0.0.1 --server-name localhost`.)

The client logs the fingerprint of the certificate it pinned — confirm it equals
the server's. Useful defaults: `--tun-name rustvpn1`, `--tun-ip 10.8.0.2`,
`--prefix 30`, `--mtu 1300`. Reconnection is automatic with exponential backoff;
disable it with `--no-reconnect`, or cap attempts with `--max-reconnects N`.

On success the client performs the QUIC handshake, the versioned control
handshake (negotiating the smaller MTU/keepalive), and begins tunneling packets.

---

## 7. TUI dashboard (primary UX)

The TUI is the intended primary way to run and observe the VPN (D-17): a live
control dashboard with connection state, throughput/RTT, peer/route panels, and a
log viewer. A **`--tui` flag** is being added in milestone M4 to launch it
alongside `server`/`client`; until it lands, use the CLI commands above and watch
the `tracing` logs.

---

## 8. Troubleshooting

- **`Operation not permitted` / TUN creation fails** — you are not root and lack
  `CAP_NET_ADMIN`. Re-run with `sudo` (Linux/macOS) or an Administrator terminal
  (Windows). On Linux, confirm the `tun` module is loaded.
- **Windows: TUN fails to open** — ensure `wintun.dll` sits next to
  `vpn-rust.exe` and you are running as Administrator.
- **Fingerprint mismatch / TLS handshake rejected** — the client's pinned
  `--server-cert` doesn't match the certificate the server is presenting, or the
  fingerprints differ. Re-copy `server-cert.der` from the server and re-verify the
  `sha256:` value. A mismatch can also mean a genuine MITM — do not ignore it.
- **`InvalidCertificate` / name error** — the client's `--server-name` doesn't
  match the certificate SAN. They must be identical (the SAN is set from the
  server's `--server-name`/`keygen --server-name`).
- **Client connects then times out / no traffic** — a firewall or NAT is blocking
  **UDP** on the server port (default **4433**). QUIC is UDP, not TCP; open UDP
  4433 inbound on the server and any path in between.
- **Tunnel up but no internet on the client** — on Linux the server applies NAT
  automatically; verify IP forwarding and that `--nat-interface` is the right
  egress NIC. On macOS/Windows, routing/NAT is not applied automatically yet
  (D-19) — add routes manually.
