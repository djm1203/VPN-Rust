---
title: "DEFINITION OF DONE"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: DefinitionOfDone
author: Derek Martinez
---

# Definition of Done — VPN-Rust

Every change must satisfy the core checklist. Feature-type-specific criteria add to it.

## Core (all changes)

- [ ] Code compiles on Linux (`cargo build` succeeds — the only supported build target).
- [ ] `cargo clippy` is clean (no warnings).
- [ ] `cargo fmt` applied (`cargo fmt --check` passes).
- [ ] Public items documented with `///`; modules have `//!` docs where appropriate.
- [ ] No `unwrap()` in production code paths (use `expect()` with a message or proper handling).
- [ ] Errors carry context via `anyhow` + `.with_context()`, and are logged before propagating.
- [ ] Relevant tests added and passing (`cargo test`).
- [ ] `docs/TASKS.md` and the BEACON state docs (`docs/state/*`) updated as needed.
- [ ] No secrets, certificates, or private keys committed.
- [ ] Commit message follows `type: desc` format (`feat`/`fix`/`docs`/`refactor`/`test`/`chore`),
      imperative mood, no AI/agent attribution trailers.

> **Windows caveat:** the build fails on Windows hosts (`from_raw_fd`/`AsyncFd` are Linux-only).
> "Compiles" and "tests pass" must be verified on Linux or WSL.

## New Feature

- [ ] Core checklist satisfied.
- [ ] New tests written for new functionality.
- [ ] User-facing behavior (CLI flags, config keys, TUI) documented.
- [ ] `config.example.toml` updated if new config options were added.

## Bug Fix

- [ ] Core checklist satisfied.
- [ ] Root cause identified and documented.
- [ ] Fix verified with a regression test where feasible.
- [ ] No unrelated changes included.

## Refactor

- [ ] Core checklist satisfied.
- [ ] No behavior change (verified by existing tests).
- [ ] Kept in a separate commit from behavior changes.

## Feature-Type Criteria

### Networking changes (TUN, TLS, routing, NAT, multi-client)

- [ ] Manually verified on Linux by running server + client and confirming end-to-end
      connectivity (e.g. `ping 10.8.0.1` from the client, `tcpdump -i rustvpn0`).
- [ ] TUN interfaces and routes/iptables rules are cleaned up on disconnect (no leaked state).
- [ ] Behavior confirmed with root/`CAP_NET_ADMIN` privileges as required.

### Security changes (mTLS, kill switch, leak prevention, key handling)

- [ ] Change reviewed for correctness against the threat it addresses.
- [ ] Certificate validation is not weakened (no dangerous verifier bypass in production paths).
- [ ] Kill switch / leak-prevention rules verified to block non-VPN traffic as intended.
- [ ] No sensitive data (keys, credentials, payloads) logged.

### Configuration changes

- [ ] Config parses correctly; invalid input produces a clear error (not a panic).
- [ ] `config.example.toml` reflects the new/changed schema.

## Deferred / Not Applicable

The framework's business-impact-assessment, data-at-scale, and background-job criteria do not
apply: VPN-Rust is a single-author, learning-focused project with no production data store,
user base, or rollout surface. Rollout risk is `None` for all backlog items.
