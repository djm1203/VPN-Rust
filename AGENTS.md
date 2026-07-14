---
title: "AGENTS"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-06-06T03:40:25Z
product_id: project
project_id: project
file_kind: AgentsMd
author: 
---

# AGENTS.md — Universal Agent Governance

> This file defines BEACON Framework governance rules for **all** LLM coding agents working on this project.
> Tool-specific instructions: `CLAUDE.md` (Claude Code), `.cursor/rules` (Cursor), `.github/copilot-instructions.md` (Copilot), `.windsurfrules` (Windsurf).

## Session Lifecycle

Follow the protocol in `docs/governance/SESSION_PROTOCOL.md` (if it exists) or `CLAUDE.md`.

Mark the session boundaries with the helper (this writes/clears
`.beacon-session.json`, which the optional pre-commit gate checks):

- Start: `.beacon/session.sh start "your goal"` (Windows: `.beacon\session.ps1 start "your goal"`)
- End:   `.beacon/session.sh end` (Windows: `.beacon\session.ps1 end`)

## Governance Rules

See `docs/governance/RULESET.md` for the full reference. The ruleset is a
language-agnostic core; stack-specific rules (e.g. Rust, database) live in
`docs/governance/rules/` and are present only if their pack was installed.

**Never violate** (HardBlock/AuditedBlock):
- R-1.1/R-1.2: No invented APIs or fabricated paths
- R-2.1: No behavior changes in refactor commits
- R-3.1: Read before write
- R-8.1/R-8.2/R-8.3: No hardcoded, logged, or committed secrets
- R-8.4: Sanitize all user input
- R-10.2/R-10.3: No push/force-push without instruction
- R-10.5: Never commit unless told to; when told, no agent attribution (no `Co-Authored-By`/"Generated with"/"written by" tags) and keep the message to a concise sentence or two
- R-11.1: Read governance files before changes
- R-11.3: No self-approval
- R-27.1: Tag backlog items with affected solution(s) before starting work
- R-27.2: Grep for all callers before modifying existing code
- R-29.1: Business impact assessment required before marking any status/flag/eligibility feature as done
- R-29.2: If >20% of records are affected on day one, a rollout plan is required before closing the ticket
- R-30.1: Background jobs must be idempotent — safe to run more than once without corrupting data
- R-31.2: High rollout risk features must be tested against production-scale data before closing
- R-33.1: New queries on large tables require EXPLAIN ANALYZE before merging
- R-34.1: SEV-1/SEV-2 incidents must produce a framework rule change before the post-mortem closes

## Working modes

Before a non-trivial task, ask whether the operator wants **autonomous** ("just
do it") or **collaborative/teaching** (explain concepts, let them code) mode. When
a task can be fanned out to parallel agents, ask before doing so unless they have
already said to proceed autonomously. See `CLAUDE.md` for details.

## Build Commands

TODO: Add build commands for this project.

## Key Conventions

- Follow existing code patterns and style
- Match formatting conventions
- Preserve test coverage
