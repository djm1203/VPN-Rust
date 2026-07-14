---
title: "CONVENTIONS"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Conventions
author: Derek Martinez
---

# Coding Conventions — VPN-Rust

Derived from `docs/CLAUDE.md`. These conventions apply to all Rust code in the repository.

## Language

Primary: **Rust**, 2021 edition, stable toolchain. Use 2021-edition idioms.

## Naming

- `snake_case` — files, functions, variables, and modules.
- `PascalCase` — structs, enums, and traits.
- `SCREAMING_SNAKE_CASE` — constants (centralized in `src/constants.rs`).
- Prefer descriptive names over abbreviations.
- Prefix async functions with action verbs (e.g. `read_packet`, `write_packet`, `connect_tls`).

## Module Organization

```
src/
├── lib.rs            # Library crate root — exports public modules
├── main.rs           # Unified binary entry point (server/client dispatch)
├── cli.rs            # clap CLI definitions (subcommands, flags)
├── constants.rs      # Centralized constants (IPs, ports, MTU, cert paths)
├── config/           # Configuration parsing
│   ├── mod.rs        #   module root
│   ├── ovpn.rs       #   OpenVPN (.ovpn) config parsing
│   └── toml_config.rs#   TOML config schema + parsing
├── net/              # Networking core
│   ├── mod.rs        #   module root
│   ├── tun.rs        #   TUN device abstraction (Linux/AsyncFd)
│   ├── tls.rs        #   rustls TLS + mTLS setup
│   ├── route.rs      #   route + NAT management
│   ├── security.rs   #   kill switch, DNS/IPv6 leak prevention
│   └── clients.rs    #   multi-client management (IP pool, stats)
├── tui/              # ratatui terminal UI
│   ├── mod.rs        #   public exports
│   ├── app.rs        #   app state, events, stats tracking
│   ├── ui.rs         #   rendering
│   └── runner.rs     #   terminal setup/teardown + event loop
└── bin/
    ├── server.rs     # server binary target
    └── client.rs     # client binary target
```

Guidelines:
- Networking code lives under `net/`; config parsing under `config/`; terminal UI under `tui/`.
- Binary targets under `bin/`; the unified entry point is `main.rs`.
- Constants (IP addresses, ports, MTU, certificate paths) are centralized in `constants.rs` —
  do not scatter magic values across modules.
- Never create files outside this structure without explicit approval.

## Error Handling

- Use `anyhow::Result` for fallible functions.
- Add context to every error path with `.with_context(|| "...")`.
- **No `unwrap()` in production code.** Use `expect()` with a clear message where a failure is
  truly unrecoverable, or propagate a proper error.
- Log errors before propagating with `log::error!`.
- Prefer `Result` over `panic!`.
- Custom error types are not currently used — `anyhow` is sufficient at this scope.

## Async Patterns

- Use the `tokio` runtime for all async I/O.
- Use `Arc<Mutex<T>>` for shared mutable state across tasks.
- Use `tokio::spawn` for concurrent tasks (e.g. bidirectional TUN↔TLS forwarding).
- Use `AsyncFd` to integrate raw TUN file descriptors with async I/O (Linux-specific).
- Avoid blocking operations inside async contexts.

## Logging

Use the `log` facade with `env_logger` (`RUST_LOG` controls level). Levels:

- `error!` — failures requiring immediate attention.
- `warn!` — recoverable issues or degraded performance.
- `info!` — general operational events (connection established, interface created).
- `debug!` — detailed diagnostics.
- `trace!` — very detailed tracing (packet contents, etc.).

Always include context in messages (interface name, address, byte counts). Never log sensitive
data (keys, passwords, or packet payloads in production paths).

```rust
info!("TUN interface created: {}", interface_name);
error!("Failed to connect to server {}: {}", addr, err);
// Avoid: info!("Done"); error!("Error occurred");
```

## Documentation

- Add `///` doc comments to all public items.
- Include examples in doc comments for non-trivial functions.
- Provide module-level docs (`//!`) describing each module's responsibility.
- Document safety invariants for any `unsafe` block.

## Formatting & Linting

- Run `cargo fmt` before committing — code must be `rustfmt`-clean.
- Run `cargo clippy` and resolve warnings before committing.

## Git / Commit Conventions

- Format: `<type>: <short description>`, imperative mood ("Add feature", not "Added feature").
- Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`.
- Atomic commits — one logical change each.
- Never commit certificates, keys, or secrets.
- Run `cargo fmt` and `cargo clippy` before every commit.

> **Governance note:** Per project `CLAUDE.md`, agents never commit unless explicitly told to,
> and commit messages carry no AI/agent attribution trailers.
