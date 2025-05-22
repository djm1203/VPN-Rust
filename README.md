# VPN-Rust

A custom VPN client and server written in Rust with TUN interface integration and secure TLS tunneling. This project is designed for learning systems-level networking using Rust, with inspiration from ProtonVPN and OpenVPN.

## 🔧 Features

- [x] Asynchronous TUN interface using `tokio::io::AsyncFd`
- [x] Secure TLS tunnel using `rustls` and `tokio-rustls`
- [x] Basic client-server echo over VPN
- [x] Manual IP assignment for TUN interfaces
- [ ] Ratatui-based CLI frontend (coming soon)
- [ ] OpenVPN protocol compatibility (in progress)

## 📦 Requirements

- Linux system (TUN/TAP support)
- Rust (latest stable)
- OpenSSL or compatible `libssl`
- Root access or `CAP_NET_ADMIN` to create TUN devices

## 🚀 Usage

### 1. Build the Project

```bash
cargo build --release

