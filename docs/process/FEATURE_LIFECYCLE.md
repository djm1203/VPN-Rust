---
title: "FEATURE LIFECYCLE"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: FeatureLifecycle
author: Derek Martinez
---

# Feature Lifecycle — VPN-Rust

## Stages

1. **Idea** — Proposed, not yet evaluated
2. **Backlog** — Accepted, prioritized in BACKLOG.md
3. **Design** — Architecture documented in DECISIONS.md
4. **Implementation** — Code written following governance rules
5. **Review** — Code reviewed, tests passing
6. **Ship** — Merged and deployed
7. **Monitor** — Post-ship observation period

## Feature Removal

1. Deprecate in prior release
2. Provide migration path
3. Remove in next major version

## Project Context (VPN-Rust)

This is a single-maintainer, learning-focused VPN client+server in Rust. The
generic stages above map onto this repo's concrete workflow as follows:

1. **Idea** → capture it. Rough thoughts go to `docs/state/OPEN_QUESTIONS.md`
   if unresolved, or straight to the backlog if actionable.
2. **Backlog item** → add a tagged entry in `docs/planning/BACKLOG.md`, aligned
   with the phased plan in `docs/planning/EXECUTION_PLAN.md` (e.g. Phase 1 —
   Core Infrastructure). Also reflected in `TASKS.md`.
3. **Design note** → for anything non-trivial (protocol changes, new modules,
   crypto/security-affecting work), write a short design note in `docs/` and
   record the decision + rationale in `docs/state/DECISIONS.md` before coding.
4. **Implement on a feature branch** → branch off `main`, keep commits atomic
   and in `type: desc` form (`feat`/`fix`/`docs`/`refactor`/`test`/`chore`,
   imperative). Follow the Rust conventions in `docs/CLAUDE.md` (anyhow errors,
   tokio async, no `unwrap()` in production paths).
5. **Tests + lint** → add/adjust unit and (where feasible) integration tests;
   `cargo test`, `cargo clippy`, and `cargo fmt` must all be clean. Note the
   build/test is **Linux-only** today.
6. **PR review** → open a PR into `main`; CI (GitHub Actions: `cargo build` +
   `cargo test`) must pass. Even as sole maintainer, use the PR for a
   self-review checkpoint.
7. **Merge to `main`** → after green CI and review.
8. **Update tracking + state docs** → mark the item done in `TASKS.md` and
   update the relevant `docs/state/*` files (STATUS, CHANGELOG, DECISIONS,
   HANDOFF as applicable) per the BEACON session close protocol.

### Working modes

Per the project `CLAUDE.md`, work proceeds in one of two modes, confirmed per
task:

- **Autonomous** — plan, implement, and verify the feature end to end,
  reporting progress.
- **Collaborative / teaching** — go slower, explain trade-offs, and let the
  maintainer write some or all of the code.

The mode can differ per feature; confirm it before starting non-trivial work.

### Protocol-affecting features

Because the wire protocol is unversioned, any feature that changes framing or
message format is breaking and must coordinate a matching client+server release
(see RELEASE_PROCESS.md). Prefer designing in a protocol version handshake as
part of such work.
