---
title: "ESCALATION POLICY"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: EscalationPolicy
author: Derek Martinez
---

# Escalation Policy — VPN-Rust

## Escalation Triggers

| Trigger | Action |
|---------|--------|
| Blocked for > 4 hours | Escalate to tech lead |
| Blocked for > 1 day | Escalate to engineering manager |
| Security incident | Escalate immediately to security lead |
| Data breach | Escalate immediately to CTO + legal |
| Production outage (SEV-1) | Page on-call, escalate per incident response |

## Escalation Levels

| Level | Role | Response Time |
|-------|------|---------------|
| L1 | Team lead / Senior engineer | 30 minutes |
| L2 | Engineering manager | 1 hour |
| L3 | CTO / VP Engineering | 2 hours |

## Technical Escalation

1. Document what you've tried and what failed
2. Provide reproduction steps or error logs
3. Tag the appropriate escalation contact
4. Update the issue/ticket with escalation status

## Decision Escalation

When a technical decision has broader impact:

1. Document options with trade-offs in DECISIONS.md
2. If no consensus after 1 day, escalate to tech lead
3. If architectural impact, escalate to architecture review
4. Document final decision and rationale

## Project Context (VPN-Rust)

VPN-Rust is a **single-maintainer** project (Derek Martinez,
derekjm1203@gmail.com). The multi-role escalation ladder above is aspirational
boilerplate — there is no on-call rotation, tech lead, or manager. In practice,
"escalation" means **surfacing a blocker to the maintainer** and tracking it in
the right state doc rather than paging anyone.

### Where blockers go

- Actionable blockers and bugs → **GitHub issues** (github.com/djm1203/VPN-Rust)
  and/or `docs/planning/BACKLOG.md`.
- Unresolved questions blocking progress → `docs/state/OPEN_QUESTIONS.md`.
- Risks (known limitations, things that could bite later) →
  `docs/state/RISKS.md`.

### Severity levels (personal-project scale)

| Severity | Meaning | Tracked in |
|----------|---------|------------|
| **Blocker** | Work cannot proceed; build/test broken on Linux, or an open question must be answered first | `OPEN_QUESTIONS.md` (question) + `RISKS.md` (if it's a standing risk); flag prominently in `HANDOFF.md` |
| **Major** | Significant limitation or bug that doesn't fully stop work (e.g. Linux-only build, minimal test coverage) | `RISKS.md` and/or a `BACKLOG.md` item |
| **Minor** | Nice-to-have, cleanup, or low-impact issue | `BACKLOG.md` |

### Security issues

Security-relevant findings (e.g. the current self-signed certificate setup,
unvalidated cert chains, unversioned/unauthenticated protocol concerns) are
handled per `docs/compliance/VULNERABILITY_POLICY.md`, not through the generic
escalation ladder above. Never commit certificates, keys, or secrets while
documenting or reproducing a security issue.
