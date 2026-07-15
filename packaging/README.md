# Packaging & Release

Scaffolding for building, packaging, and distributing the single `vpn-rust`
binary (subcommands: `server`, `client`, `keygen`).

## Release workflow

[`.github/workflows/release.yml`](../.github/workflows/release.yml) builds and
publishes release artifacts:

1. **Trigger** — a pushed tag matching `v*` (or manual `workflow_dispatch` for a
   dry run that builds/packages but does not publish).
2. **Build** — `cargo build --release --bin vpn-rust` on a matrix of
   `ubuntu-latest`, `windows-latest`, and `macos-latest`. Toolchain/cache actions
   (`actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`)
   match `.github/workflows/ci.yml`.
3. **Package** — the binary is placed in a per-OS archive named
   `vpn-rust-<os>-<arch>` (`.tar.gz` on Linux/macOS, `.zip` on Windows).
4. **Upload** — each archive is uploaded as a workflow artifact.
5. **Publish** — on a real tag push, archives are attached to a GitHub Release via
   `softprops/action-gh-release@v2` (auto-generated release notes).

### Cut a release

```bash
git tag v0.1.0
git push origin v0.1.0
```

The workflow builds all three platforms and creates the `v0.1.0` GitHub Release
with the archives attached. Artifacts also appear under the workflow run's
"Artifacts" section (useful for `workflow_dispatch` dry runs, which do not publish).

## Install notes

### Linux (server) — systemd

Use the packaged unit. See [`systemd/README.md`](./systemd/README.md): install the
binary to `/usr/local/bin`, create `/etc/vpn-rust/certs`, drop in the unit,
`systemctl daemon-reload`, then `enable --now`.

### Windows (client)

1. Extract `vpn-rust-windows-x86_64.zip`.
2. **Place `wintun.dll` beside `vpn-rust.exe`.** The Wintun driver is required for
   the TUN device and is **not bundled** in the release archive — download it from
   <https://www.wintun.net> and copy the arch-matching `wintun.dll` next to the exe.
3. Run from an **Administrator** terminal (creating the TUN adapter and routes needs
   elevation):
   ```powershell
   .\vpn-rust.exe client --server vpn.example.com --port 4433 --server-name vpn.example.com --server-cert server-cert.der
   ```

### macOS (client)

Run with `sudo` — creating a `utun` interface and installing routes requires root:

```bash
sudo ./vpn-rust client --server vpn.example.com --port 4433 --server-name vpn.example.com --server-cert server-cert.der
```

## Where artifacts land

- **GitHub Release** (tag pushes): attached to the `vX.Y.Z` release page.
- **Workflow artifacts** (every run, including `workflow_dispatch`): under the run's
  "Artifacts" section, one per platform.
