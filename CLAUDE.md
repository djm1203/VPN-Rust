---
title: "CLAUDE"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-06-06T03:40:25Z
product_id: project
project_id: project
file_kind: ClaudeMd
author: 
---

# CLAUDE.md — project

> **Governance:** This project runs under the BEACON Framework. Classification: **high**.

---

## MANDATORY SESSION PROTOCOL — READ THIS FIRST

**HARD GATE: No code changes, file edits, or commits are permitted until the session resume protocol below is fully complete.** This is a binding requirement enforced by Rule R-11.1 (HardBlock severity). If the optional pre-commit hook is installed, it also blocks commits until a session is active (`.beacon-session.json` exists).

### Session start (resume protocol) — BLOCKING

Complete every step below **in order** before writing any code:

**Step 1.** Read this file (CLAUDE.md) — you are doing this now.
**Step 2.** Read `docs/state/STATUS.md` — current project state.
**Step 3.** Read `docs/state/HANDOFF.md` — where the last session stopped.
**Step 4.** Read `docs/state/OPEN_QUESTIONS.md` — unresolved questions.
**Step 5.** Read `docs/planning/BACKLOG.md` — prioritized work items.
**Step 6.** Run `git status` and verify the working tree is clean.
**Step 7.** Verify tests pass.
**Step 8.** State the session goal and first task.
**Step 9.** Start the session marker so the optional commit gate is satisfied:
  - macOS / Linux: `.beacon/session.sh start "your goal"`
  - Windows: `.beacon\session.ps1 start "your goal"`

  This writes `.beacon-session.json`. Run `.beacon/session.sh status` to check it.

After completing all 9 steps, output this exact line:

```
BEACON SESSION ACTIVE — Resume protocol complete. Session goal: [your goal here]
```

### Session close (before ending) — BLOCKING

1. Verify all tests pass
2. Verify working tree is clean (committed or stashed)
3. Update `docs/state/STATUS.md`
4. Update `docs/state/HANDOFF.md`
5. Append to `docs/state/CHANGELOG.md`
6. Update `docs/state/DECISIONS.md`
7. Update `docs/state/OPEN_QUESTIONS.md`
8. Update `docs/state/RISKS.md`
9. End the session marker:
   - macOS / Linux: `.beacon/session.sh end`
   - Windows: `.beacon\session.ps1 end`

After completing all 9 steps, output this exact line:

```
BEACON SESSION CLOSED — All state files updated. Close protocol complete.
```

### Enforcement mechanisms

1. **CLAUDE.md / AGENTS.md instructions** (this file) — behavioral gate for LLM agents. This is the primary mechanism.
2. **Pre-commit hook** (`.git/hooks/pre-commit`) — *optional*; installed only with the installer's `--hook` flag. Blocks commits unless a session is active (`.beacon-session.json` exists). Use the session helper to start/end a session.

---

## Commit policy (R-10.5)

- **Never commit on your own.** Create a commit only when the user explicitly tells you to.
- **No agent attribution.** When you do commit, do not add `Co-Authored-By` trailers, "Generated with"/"Created with" lines, "written by", or any tag, footer, or signature naming the AI agent, model, or tool.
- **Keep it concise.** A short subject and at most a sentence or two of body — only what changed and why. No lengthy descriptions or bullet-point changelogs. Detail belongs in the state docs, not the commit message.

---

## Working modes — ask before assuming

Before starting a non-trivial task, confirm how the operator wants to work, and stay in that mode until they change it:

- **Autonomous** — "just do it." Plan, execute, and verify the whole task end to end, reporting progress as you go.
- **Collaborative / teaching** — go slower. Explain the concepts, trade-offs, and approach, and let the operator write some or all of the code themselves.

The preferred mode can differ per task. If it is unclear which the operator wants, ask.

**Parallel agents.** When a task or set of subtasks is independent and could be run by parallel agents (authoring many files, auditing many modules, migrating many call sites), say so and ask whether to fan out. Only launch parallel agents after the operator agrees — unless they have already told you to proceed autonomously.

---

## Project overview

A multi-language project (0 files, 0 lines) using unknown.

**Primary language:** unknown
**Build system:** unknown
**Frameworks:** None detected
**Size:** 0 files, 0 lines

## Build commands

```bash
# TODO: Add build commands
```

## Testing

```bash
# TODO: Add test commands
```

## Key conventions

- Follow existing code patterns and style
- Match the project's formatting conventions
- Preserve test coverage when modifying code

## Monorepo structure

> **Fill this in when setting up for a monorepo project. Delete this section for single-solution repos.**

This repo contains multiple solutions. Each solution has its own PRD:

| Solution | PRD | Description |
|----------|-----|-------------|
| [SOL-A]  | `docs/product/PRD-sol-a.md` | TODO: describe |
| [SOL-B]  | `docs/product/PRD-sol-b.md` | TODO: describe |
| [SOL-C]  | `docs/product/PRD-sol-c.md` | TODO: describe |

Add more rows as needed. Rename `[SOL-A]` etc. to match your actual solution names.

All backlog items must be tagged with their solution (e.g., `[SOL-A]`). Cross-solution items get tagged with every solution they touch (e.g., `[SOL-A][SOL-B]`).

Before starting any ticket, identify which solution(s) it touches and read the relevant PRD(s).

## Legacy codebase protocol

> **Fill this in if working on a codebase with significant history (2+ years). Delete if greenfield.**

This codebase has years of history. Before modifying any existing code:

1. **Check all callers.** Grep the entire repo for every method, class, or constant you plan to change. A method that looks unused may be called from an old migration, a rake task, or a concern in a different solution.
2. **Read the git log.** Run `git log -p -- <file>` on any file you're about to change. Understand why past changes were made before adding to them.
3. **Look for hidden side effects.** Check for ActiveRecord callbacks (`before_save`, `after_commit`, etc.), concerns, observers, and service objects that may react to your change silently.
4. **Prefer additive changes.** When modifying behavior, add a new method alongside the old one rather than changing the old one in place. Deprecate the old method explicitly.
5. **Never remove a method** without confirming zero callers across every solution in the repo.
6. **Test against the full suite**, not just the tests closest to your change.

## Governance Documents

| Tier | Directory | Contents |
|------|-----------|----------|
| 1 | (this file) | Agent-specific instructions |
| 2 | `docs/product/`, `docs/architecture/`, `docs/planning/` | PRD, capabilities, use cases, glossary, architecture, backlog, execution plan |
| 3 | `docs/standards/` | API contracts, build/test, conventions, DoD, dependencies, deployment, performance, schema policy |
| 3 | `docs/operations/` | Security model, change management, incident response, monitoring |
| 3 | `docs/compliance/` | Compliance matrix, data governance, vulnerability policy |
| 4 | `docs/process/` | Release, feature lifecycle, escalation, handoff contract |
| 4 | `docs/governance/` | Ruleset, session protocol, LLM boundaries, prompt safety |

## Off-limits areas

None specified yet.
