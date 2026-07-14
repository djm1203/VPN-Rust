---
title: "HANDOFF CONTRACT"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: HandoffContract
author: Derek Martinez
---

# Handoff Contract — VPN-Rust

## Session Handoff Requirements

Every session must update the following files before closing:

| File | Required Content |
|------|-----------------|
| `docs/state/STATUS.md` | Current status, what's done, what's in flight, what's next |
| `docs/state/HANDOFF.md` | Where we stopped, what's next, what to watch |
| `docs/state/CHANGELOG.md` | What changed this session |
| `docs/state/DECISIONS.md` | Any decisions made with context and rationale |
| `docs/state/OPEN_QUESTIONS.md` | Unresolved questions with blocking status |
| `docs/state/RISKS.md` | New or changed risks |

## Handoff Quality Criteria

A handoff is complete when the next session can start without asking "what happened?":

- [ ] Working tree is clean (all changes committed or stashed)
- [ ] All tests pass
- [ ] HANDOFF.md explains where work stopped and why
- [ ] HANDOFF.md lists the next logical step
- [ ] Any "gotchas" or surprises are documented in What to Watch
- [ ] New files or significant changes are mentioned in CHANGELOG.md
- [ ] If monorepo: every solution touched this session is listed with a one-line summary of what changed in it
- [ ] If cross-solution change: callers checked, side effects documented (R-27.2, R-27.4)

## Context Transfer

- Never assume the next session has context from this session
- Be specific: "I stopped mid-way through implementing the store trait for PluginResult" > "work in progress"
- Include file paths and function names, not just descriptions
- If a workaround was applied, document it and the intended permanent fix

## Emergency Handoff

If a session must end abruptly:

1. Commit or stash all changes
2. Write a minimal HANDOFF.md entry with what's in flight
3. Note any broken state (failing tests, partial implementation)

## Project Context (VPN-Rust)

VPN-Rust runs the **BEACON session protocol** defined in the project
`CLAUDE.md`. The generic handoff requirements above are enacted through that
protocol's **session close** steps. `docs/state/HANDOFF.md` is the **living
handoff document** — the single source of truth for where the last session
stopped; read it at session start and update it at session close.

### Session close checklist (this project)

Before ending a session:

1. **Tests pass** — run `cargo test` (and ideally `cargo clippy` /
   `cargo fmt --check`). Note that the build and tests are **Linux-only** today;
   they fail on Windows. If closing from a Windows environment where tests
   cannot run, say so explicitly in HANDOFF.md rather than claiming green.
2. **Working tree clean** — all changes committed or stashed
   (`git status` clean). Do not commit certificates, keys, or secrets.
3. **Update the state docs** (all six):
   - `docs/state/STATUS.md`
   - `docs/state/HANDOFF.md`
   - `docs/state/CHANGELOG.md`
   - `docs/state/DECISIONS.md`
   - `docs/state/OPEN_QUESTIONS.md`
   - `docs/state/RISKS.md`
4. **End the session marker** so the optional commit gate is released:
   - Windows: `.beacon\session.ps1 end`
   - macOS / Linux: `.beacon/session.sh end`

After the state files are updated and the marker is ended, the close protocol
emits its `BEACON SESSION CLOSED` confirmation line (see `CLAUDE.md`).

### What to carry forward

Given the project's known state, handoffs should keep the next session aware of:
the Linux-only build constraint, minimal test coverage, self-signed certs, and
the unversioned wire protocol (protocol changes require coordinated
client+server releases). Record any of these that changed this session in
`RISKS.md` / `HANDOFF.md`.
