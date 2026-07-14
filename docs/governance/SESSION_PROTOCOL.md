---
title: "SESSION PROTOCOL"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-06-06T03:40:25Z
product_id: project
project_id: project
file_kind: SessionProtocol
author: 
---

# Session Protocol — project

> Managed by the BEACON Framework. Classification: **high**.

---

## Resume Protocol (before coding)

Complete these steps in order before making any code changes:

1. Read `CLAUDE.md` (or `AGENTS.md` for non-Claude tools)
2. Read state files: `docs/state/STATUS.md`, `docs/state/HANDOFF.md`, `docs/state/OPEN_QUESTIONS.md`
3. Read `docs/planning/BACKLOG.md`
4. Run `git status` — working tree must be clean
5. Run the project's test command — all tests must pass
6. State your session goal, then output:
   ```
   BEACON SESSION ACTIVE — Resume protocol complete. Session goal: [goal]
   ```

**Do not skip steps.** If the working tree is dirty or tests fail, resolve before proceeding.

---

## Close Protocol (before ending)

1. All tests pass, working tree clean
2. Update state files:
   - `docs/state/STATUS.md` — current status and blockers
   - `docs/state/HANDOFF.md` — what the next session needs to know
   - `docs/state/CHANGELOG.md` — what changed this session
   - `docs/state/DECISIONS.md` — any decisions made
   - `docs/state/OPEN_QUESTIONS.md` — unresolved questions
   - `docs/state/RISKS.md` — new or changed risks
3. If any incident occurred this session: confirm a rule change or DoD update was made (R-34.1)
4. If a feature flag was introduced: confirm owner and removal criteria are documented (R-32.1)
5. If a high rollout risk item shipped: confirm activation plan is documented separately from the code merge (R-29.3)
6. Output:
   ```
   BEACON SESSION CLOSED — All state files updated. Close protocol complete.
   ```

---

## Critical Rules During Session

| Rule | Description |
|------|-------------|
| R-1.1 | No invented APIs — verify before referencing |
| R-1.2 | No fabricated paths — verify files exist |
| R-2.1 | No behavior change in refactors |
| R-3.1 | Read before write — read a file before editing it |
| R-8.1 | No hardcoded secrets |
| R-8.2 | No logged secrets |
| R-8.3 | No committed secrets |
| R-8.4 | Sanitize user input |
| R-10.2 | No push without explicit instruction |
| R-10.3 | No force-push without explicit instruction |
| R-10.5 | Never commit unless told; no agent attribution tags; concise messages |
| R-11.1 | Read CLAUDE.md/AGENTS.md first |
| R-11.3 | No self-approval of changes |

See `docs/governance/RULESET.md` for the full 73-rule reference.

---

*BEACON Framework — customize as needed.*
