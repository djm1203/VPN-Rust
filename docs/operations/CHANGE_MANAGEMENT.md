---
title: "CHANGE MANAGEMENT"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: ChangeManagement
author: Derek Martinez
---

# Change Management — VPN-Rust

> **Scope note:** Single-maintainer, git-based workflow governed by the BEACON
> Framework session protocol. "Approval" is largely self-review (the maintainer),
> with GitHub PR review used where a second set of eyes exists. Right-sized to a
> personal learning project, not an enterprise change board.

## How Changes Flow

- **Version control:** Git, `main` as the trunk. Work happens on feature branches
  and merges via pull request (precedent: PR #1,
  `codex/find-and-fix-code-errors`, merged into `main`).
- **Session gate (BEACON):** No code changes until the session **resume
  protocol** is complete (read state docs, clean tree, tests pass, session
  marker started). On close, all `docs/state/*` files are updated and the session
  marker ended. The optional pre-commit hook (`.git/hooks/pre-commit`, installed
  with `--hook`) blocks commits unless a session is active
  (`.beacon-session.json` present).
- **Commits:** Convention `type: short description` where type ∈
  `feat` / `fix` / `docs` / `refactor` / `test` / `chore`; imperative mood,
  atomic. **Never commit on your own** — only when the maintainer explicitly asks
  (R-10.5). No AI-attribution trailers. Run `cargo fmt` and `cargo clippy` first.
- **CI:** GitHub Actions runs build + test on push (see `10db415 Github actions
  for rust`). A change is not "done" until CI is green.
- **Never commit secrets/certs:** `.key` files stay untracked; `.gitignore`
  covers session/local state (R-8.3).

## Change Classification

| Class | Description | Approval | Notes |
|-------|-------------|----------|-------|
| Standard | Docs, comments, formatting, non-behavioral | Self-review + green CI | Low risk |
| Normal | Feature or bug fix touching code behavior | Feature branch + PR review + CI | Update `docs/state/*` |
| Protocol / breaking | Wire-format, cert layout, or config schema change | PR review + version bump | Follow `docs/standards/SCHEMA_CHANGE_POLICY.md` |
| Emergency | Security fix (e.g., leaked key, leaking kill switch) | Do first, document immediately after | Rotate certs if key material involved |

### Change categories in practice
- **Code** — `src/**`; requires build + test to pass.
- **Config** — server address/port, cert paths, subnet/CIDR; validate before use.
- **Protocol** — length-prefixed framing, TLS/mTLS expectations, cert/CA layout.
  Breaking changes require a version bump and a note in `DECISIONS.md`.
- **Docs** — `docs/**`; the governance and state docs themselves.

## Review Requirements

- All code changes get at least self-review against the Definition of Done
  (`docs/standards/DEFINITION_OF_DONE.md`); use a PR for a second reviewer when
  available (R-11.3: no self-approval where review is expected).
- Security-sensitive changes (TLS/mTLS in `src/net/tls.rs`, kill switch/leak
  prevention in `src/net/security.rs`, NAT/forwarding in `src/net/route.rs`) get
  extra scrutiny and, ideally, a manual test on Linux.
- `unsafe` blocks (raw fd handling) must be reviewed and their invariants
  documented.

## Communication & Audit Trail

- Decisions recorded in `docs/state/DECISIONS.md`; changes logged in
  `docs/state/CHANGELOG.md`; status/handoff in `docs/state/STATUS.md` and
  `docs/state/HANDOFF.md`.
- Breaking changes: note the migration in the changelog and bump the version.

## Rollback

- Prefer `git revert <commit>` to undo a merged change while preserving history.
- For runtime state (interfaces, iptables, resolv.conf), roll back with
  `./cleanup_vpn.sh` and the `SecurityManager`/`route.rs` cleanup paths — see
  `INCIDENT_RESPONSE.md`.
