---
title: "RELEASE PROCESS"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: ReleaseProcess
author: Derek Martinez
---

# Release Process — VPN-Rust

## Versioning

Follow Semantic Versioning (SemVer): MAJOR.MINOR.PATCH.

## Pre-Release Checklist

- [ ] All tests pass
- [ ] No known critical bugs
- [ ] CHANGELOG updated
- [ ] Version numbers bumped
- [ ] Documentation current

## Release Steps

1. Create release branch from main
2. Run full test suite
3. Update version numbers
4. Update CHANGELOG
5. Create tag
6. Build release artifacts
7. Publish

## Hotfix Process

1. Branch from release tag
2. Fix the issue
3. Create PATCH release
4. Cherry-pick fix to main

## Project Context (VPN-Rust)

`vpn-rust` is currently at **v0.1.0** (pre-1.0). Under SemVer's pre-1.0 rules,
breaking changes are permitted between minor versions, so consumers should
expect churn until a 1.0 line is declared.

### What a release is here

A release is a **git tag on `main`** created after the following pass:

- `cargo build --release` succeeds (on **Linux** — the build is Linux-only today
  and fails on Windows/macOS).
- `cargo test` passes (on Linux).
- `cargo clippy` is clean (no warnings).
- `cargo fmt --check` is clean.
- `docs/state/CHANGELOG.md` is updated for the release.
- Version bumped in `Cargo.toml`.

Tag naming follows SemVer, e.g. `v0.1.0`. Commits leading up to the release use
the `type: desc` convention (`feat`/`fix`/`docs`/`refactor`/`test`/`chore`),
imperative mood.

### Artifacts

The release artifact is the compiled `target/release/vpn-rust` binary
(Linux x86_64). There is **no published crate or registry release** yet — the
project is not on crates.io, and no binaries are distributed. Building from
source is the only supported path.

### Protocol coupling — ship client + server together

The wire protocol is a simple length-prefixed framing and is currently
**unversioned**. Any protocol change is therefore a **breaking change** with no
negotiation or backward compatibility. A release that touches the protocol MUST
ship a **matching client and server** built from the same tag; mixing versions
across the protocol boundary is unsupported and will fail or misbehave. Adding a
protocol version handshake is a tracked future improvement (see the state docs /
BACKLOG.md).

### CI

GitHub Actions runs `cargo build` + `cargo test` on push/PR. Green CI on `main`
is a precondition for tagging a release.
