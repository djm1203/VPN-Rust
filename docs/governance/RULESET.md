---
title: "RULESET"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-06-06T03:40:25Z
product_id: project
project_id: project
file_kind: Ruleset
author: 
---

# BEACON Framework Ruleset — project

**Version:** 5.0.0

**Classification:** high

> **Core + packs.** This file is the language-agnostic **core** ruleset — it
> applies to every project regardless of stack. Stack-specific rules (Rust,
> database, etc.) live in `docs/governance/rules/` and are present only if their
> pack was installed (see `installed_rule_packs` in `.beacon-framework.toml`).
> When a pack is present, its rules apply in addition to the core.

---

## HallucinationControl

### R-1.1 — No invented APIs

**Severity:** HardBlock

Never reference an API endpoint, function signature, CLI flag, or library method unless you have verified it exists in the current codebase or official documentation.

### R-1.2 — No fabricated paths

**Severity:** HardBlock

Never reference a file path, directory, or module that does not exist. Verify existence before referencing.

### R-1.3 — Verify library versions

**Severity:** SoftBlock

When specifying a dependency version, confirm it exists in the package registry. Do not guess version numbers.

### R-1.4 — Match actual error messages

**Severity:** SoftBlock

When referencing or handling error messages, use the exact text from the codebase or runtime output.

### R-1.5 — Admit uncertainty

**Severity:** Advisory

When unsure about an API, version, or behavior, state the uncertainty rather than guessing.

## FaithfulnessDuringEdits

### R-2.1 — No behavior changes during refactor

**Severity:** HardBlock

A refactor commit must not change observable behavior. If behavior change is needed, it must be a separate commit.

### R-2.2 — No silent deletions

**Severity:** SoftBlock

Do not remove code, comments, or configuration without explicit instruction or confirmation.

### R-2.3 — Edit in place

**Severity:** SoftBlock

Modify existing files rather than creating new files and deleting old ones, unless restructuring is explicitly requested.

### R-2.4 — Show diffs for large changes

**Severity:** Advisory

For changes spanning more than 20 lines, describe what changed and why before applying.

### R-2.5 — Confirm destructive changes

**Severity:** SoftBlock

Deleting files, dropping tables, removing features, or resetting state requires explicit confirmation.

### R-2.6 — Preserve formatting conventions

**Severity:** Advisory

Match the existing formatting style (indentation, line length, bracket placement) of the file being edited.

### R-2.7 — No orphaned imports

**Severity:** Advisory

When removing code that was the only user of an import, remove the import too.

### R-2.8 — Destructive pattern soft-block

**Severity:** SoftBlock

Patterns like rm -rf, DROP TABLE, git reset --hard, and force-push require explicit approval.

## CodebaseRespect

### R-3.1 — Read before write

**Severity:** HardBlock

Read the target file and its surrounding context before making any edit. Never write blind.

### R-3.2 — Match existing patterns

**Severity:** Advisory

Follow the naming conventions, architectural patterns, and idioms already established in the codebase.

### R-3.3 — Respect module boundaries

**Severity:** Advisory

Place code in the module where similar code already lives. Do not scatter related logic across distant files.

### R-3.4 — Configuration file changes require review

**Severity:** SoftBlock

Edits to configuration files (Cargo.toml, package.json, tsconfig, CI configs) must be called out explicitly.

### R-3.5 — Honor existing abstractions

**Severity:** Advisory

Use existing utility functions, shared types, and helper modules rather than duplicating their logic.

### R-3.6 — Understand before changing

**Severity:** Advisory

Before modifying unfamiliar code, read enough context to understand its purpose and invariants.

### R-3.7 — Preserve test coverage

**Severity:** Advisory

When modifying code that has tests, update the tests to reflect the new behavior.

## DefensiveCodeDiscipline

### R-4.1 — No reflexive try/except

**Severity:** Advisory

Do not wrap code in broad exception handlers. Catch specific errors with specific recovery strategies.

### R-4.2 — No premature optimization

**Severity:** Advisory

Write clear, correct code first. Optimize only when measurements show a bottleneck.

### R-4.3 — No global mutable state

**Severity:** Advisory

Avoid global variables and singletons. Pass state explicitly through function parameters.

### R-4.4 — Fail explicitly

**Severity:** Advisory

When a function cannot fulfill its contract, return an error or raise an exception. Do not return partial or default results silently.

### R-4.5 — Validate at boundaries

**Severity:** Advisory

Validate input at system boundaries (user input, API endpoints, file reads). Trust internal function calls.

### R-4.6 — Minimize dependencies

**Severity:** Advisory

Prefer standard library features over third-party dependencies for simple tasks.

### R-4.7 — No magic numbers

**Severity:** Advisory

Use named constants for non-obvious numeric or string literals.

## TestingHonesty

### R-5.1 — Tests must assert

**Severity:** SoftBlock

Every test must contain at least one assertion. A test that only calls code without checking results proves nothing.

### R-5.2 — No self-fulfilling mocks

**Severity:** SoftBlock

Mocks must not mock the system under test. Mock dependencies, not the thing being verified.

### R-5.3 — Never disable tests without justification

**Severity:** SoftBlock

Do not comment out, skip, or ignore tests without documenting why and creating a follow-up to fix them.

## ExecutionState

### R-6.1 — Distinguish ran from wrote

**Severity:** SoftBlock

Explicitly state whether code was executed and verified versus written but not yet run.

### R-6.2 — Surface stubs and workarounds

**Severity:** SoftBlock

If you create a temporary stub, workaround, or placeholder, mark it clearly and note what it replaces. Stubs must be listed in HANDOFF.md.

### R-6.3 — No silent mocks in production code

**Severity:** SoftBlock

Mock objects and fake implementations must never appear in production code paths. Functions returning hardcoded placeholder values are mocks.

## ScopeDiscipline

### R-7.1 — Smallest unit of edit

**Severity:** SoftBlock

Make the smallest change that achieves the stated goal. Do not bundle unrelated improvements.

### R-7.2 — No unprompted boilerplate

**Severity:** Advisory

Do not generate template code, starter files, or scaffolding unless explicitly requested.

### R-7.3 — Scope expansion requires confirmation

**Severity:** SoftBlock

If the required change is larger than initially described, explain the expanded scope and get confirmation before proceeding.

### R-7.4 — One concern per commit

**Severity:** Advisory

Each commit should address a single logical change. Do not mix features, fixes, and refactors.

## SecurityAndSecrets

### R-8.1 — No hardcoded credentials

**Severity:** AuditedBlock

Never include API keys, passwords, tokens, or other secrets in source code, configuration files, or comments.

### R-8.2 — No logged secrets

**Severity:** AuditedBlock

Never log, print, or display secrets, tokens, or credentials in any output.

### R-8.3 — No committed env files

**Severity:** AuditedBlock

Never commit .env files, credential stores, or key files to version control.

### R-8.4 — Sanitize user input

**Severity:** HardBlock

All user-provided input must be sanitized before use in queries, commands, or rendered output.

## SideEffectsAndExternalState

### R-9.1 — No unprompted writes

**Severity:** SoftBlock

Do not write to the filesystem, database, or external service without explicit instruction.

### R-9.2 — No unprompted network calls

**Severity:** SoftBlock

Do not make HTTP requests, API calls, or other network operations unless explicitly requested.

### R-9.3 — Environment-dependent code warning

**Severity:** Advisory

Code that behaves differently based on environment variables, OS detection, or machine state should be called out.

## VersionControlHygiene

### R-10.1 — No commits without instruction

**Severity:** SoftBlock

Do not create git commits unless explicitly told to. Stage and commit are distinct, intentional actions.

### R-10.2 — No push without instruction

**Severity:** HardBlock

Never push to a remote repository without explicit instruction.

### R-10.3 — No force-push without instruction

**Severity:** HardBlock

Never force-push to any branch without explicit instruction and confirmation of the target branch.

### R-10.4 — Destructive git patterns require approval

**Severity:** SoftBlock

git reset --hard, git clean -f, git checkout -- (destructive), and branch deletion require explicit approval.

### R-10.5 — Commit message discipline and no agent attribution

**Severity:** HardBlock

Agents must not create commits on their own. Committing is permitted only when the user explicitly instructs it (see R-10.1). When the user does instruct a commit, the commit message must obey both of the following:

1. **No agent attribution of any kind.** Do not add `Co-Authored-By` trailers, "Generated with" / "Created with" lines, "written by", or any tag, footer, or signature that names the AI agent, model, or tool. The commit history must not advertise that an agent wrote it.
2. **Concise messages only.** Write a short subject line and, at most, a sentence or two of body — only what is needed to explain what changed and why. No lengthy descriptions, bullet-point changelogs, or marketing language. If the change genuinely needs more explanation, that belongs in the state docs, not the commit message.

## ApprovalAndToolUse

### R-11.1 — Read CLAUDE.md first

**Severity:** HardBlock

Read the project's CLAUDE.md before making any changes. It contains project-specific rules that override defaults.

### R-11.2 — Show commands before running

**Severity:** Advisory

Display shell commands before executing them so the user can review and approve.

### R-11.3 — No self-approval

**Severity:** HardBlock

Never approve your own changes, merge your own PRs, or bypass review gates.

### R-11.4 — Tool execution requires permission

**Severity:** HardBlock

Do not invoke tools, scripts, or external processes that are not on the approved list without permission.

## WhenStuck

### R-12.1 — Two-attempt limit

**Severity:** SoftBlock

If two attempts at solving a specific problem fail, stop and escalate to the human for guidance.

### R-12.2 — Escalate for scope clarification

**Severity:** Advisory

When the required scope is unclear or the task seems larger than initially described, ask for clarification rather than guessing.

## Performance

### R-13.1 — No unbounded collections

**Severity:** SoftBlock

Do not create Vec, HashMap, or other growable collections that accept unbounded external input without a size limit.

### R-13.2 — Pagination required for list endpoints

**Severity:** Advisory

All API endpoints and list-returning handlers (REST, GraphQL, RPC, IPC/command handlers) must support pagination via cursor or offset/limit.

## Observability

### R-14.1 — Structured logging required

**Severity:** Advisory

Use your logging library's structured key-value fields (spans/attributes). Do not use string interpolation for log context.

### R-14.2 — Error context required

**Severity:** Advisory

Errors must include enough context (operation, input, state) to diagnose without reproducing.

### R-14.3 — No silent error swallowing

**Severity:** SoftBlock

Do not silently discard errors — empty catch blocks, ignored return codes, or discard idioms (e.g. `.ok()`, `.unwrap_or_default()`, `_ = result`) — without logging or justification.

### R-14.4 — Every new feature must define its production health signal

**Severity:** SoftBlock

Before closing a ticket, state how you will know the feature is working correctly in production — a metric, a log line, a dashboard, or an alert threshold. A feature with no observable health signal cannot be monitored and cannot be diagnosed when it breaks.

## Dependencies

### R-15.1 — New dependency requires justification

**Severity:** SoftBlock

Adding a new third-party dependency requires documenting why the standard library or existing dependencies are insufficient.

### R-15.2 — License compatibility required

**Severity:** HardBlock

All dependencies must use approved licenses (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0). GPL, AGPL, and SSPL are prohibited.

### R-15.3 — No pinned git dependencies

**Severity:** SoftBlock

Use your package registry's published versions, not git repository references or path dependencies to external repos.

## ApiDesign

### R-17.1 — Breaking API changes require deprecation

**Severity:** SoftBlock

Breaking changes to public REST API endpoints must follow the deprecation process: announce in prior release, provide migration path, sunset with notice.

### R-17.2 — Standard error envelope

**Severity:** Advisory

API error responses must use the standard envelope: { error: { code, message, detail } }.

## LlmBehavior

### R-19.2 — Prompt safety boundaries required

**Severity:** SoftBlock

System instructions must be separated from user content with explicit delimiters. User content must never be interpolated into system prompts.

## PromptCompliance

### R-20.1 — Implementation must match prompt specification

**Severity:** HardBlock

When an implementation brief, prompt, or backlog acceptance criteria specifies component architecture, visual layout, naming, file structure, or behavioral contracts, the implementation must match exactly. Deviations are only permitted when the specification conflicts with an existing governance rule.

### R-20.2 — No merging or flattening specified structure

**Severity:** HardBlock

When a prompt specifies N separate components, files, modules, or architectural layers, the implementation must create all N artifacts. Combining multiple specified artifacts into a single file or flattening a specified hierarchy is a violation.

### R-20.3 — Document deviations from prompt

**Severity:** SoftBlock

If a specification cannot be implemented exactly as written due to technical constraints, the deviation must be documented inline with a comment referencing the prompt and explaining the constraint. Silent deviation is never acceptable.

### R-20.4 — Literal execution of numbered instructions

**Severity:** HardBlock

When given numbered steps, checklists, or explicitly sequenced instructions, execute each step exactly as written, in order, without skipping, reordering, combining, or reinterpreting. If a step appears wrong, contradictory, or suboptimal — STOP and ask the operator before proceeding. Do not silently substitute an alternative approach.

### R-20.5 — No unsolicited improvements to instructions

**Severity:** SoftBlock

Do not add features, refactor surrounding code, change naming conventions, alter file organization, introduce abstractions, or improve anything beyond what the instruction explicitly requests. If the operator's instruction is narrowly scoped, the execution must be equally narrow. Unsolicited scope expansion requires operator confirmation before proceeding.

### R-20.6 — Ambiguity must be resolved by asking, not by assuming

**Severity:** SoftBlock

When an instruction is ambiguous, underspecified, or could be interpreted multiple ways, the agent must stop and ask for clarification. The agent must not pick the interpretation it considers most likely and proceed silently. The question must state the ambiguity and present the competing interpretations.

### R-20.7 — Step completion reporting

**Severity:** HardBlock

After completing each numbered step or acceptance criterion in an execution instruction, the agent must report: step reference (which step or AC was just executed), action taken (one-sentence summary), files changed (list of files created, modified, or deleted), and expectation match (explicit confirmation that the result matches the instruction, or a flagged deviation with justification). This report must appear in the conversation before proceeding to the next step. Batch-completing multiple steps with a single summary at the end is a violation.

### R-20.8 — Diff-to-instruction traceability gate

**Severity:** HardBlock

Before marking any task, backlog item, or instruction set as complete, the agent must produce a traceability summary that maps every file change (addition, modification, deletion) back to the specific instruction step or acceptance criterion that required it. Any change that cannot be traced to an explicit instruction must be called out as untraced, justified with a specific reason, and accepted by the operator before the task is marked complete. If the traceability summary reveals changes that were not instructed and cannot be justified, those changes must be reverted before completion.

### R-20.9 — Mid-execution compliance checkpoints

**Severity:** SoftBlock

For instruction sets with 5 or more steps or acceptance criteria, the agent must pause at defined checkpoints to verify alignment with the specification before continuing. Checkpoints occur after completing step 3 (if total steps >= 5), at the midpoint of the instruction set, and after the final step before declaring completion. At each checkpoint the agent must list all steps completed so far and their status, confirm alignment with overall instruction intent, flag any accumulated drift or assumptions, and ask the operator to confirm continuation if deviations were flagged. The operator may waive checkpoints with 'skip checkpoints' but the final traceability gate (R-20.8) always applies.

## DeliveryIntegrity

### R-21.1 — Acceptance criteria verification before shipping

**Severity:** HardBlock

Before marking any backlog item as Shipped, the agent MUST verify each acceptance criterion against the actual implementation. For each criterion: read the relevant source code, confirm the feature exists and functions as specified, and report the verdict (PASS/PARTIAL/FAIL). Any FAIL means the item cannot be marked shipped. Any PARTIAL must be explicitly acknowledged with a remediation plan.

### R-21.2 — No code changes without a backlog item

**Severity:** HardBlock

Every code change MUST trace to an active backlog item. The agent must identify the backlog item before starting work. If no item exists, one must be created first. Ad-hoc changes, drive-by fixes, and scope creep outside the stated backlog item are prohibited. Emergency hotfixes require a backlog item created retroactively in the same session.

### R-21.3 — No stub implementations marked as complete

**Severity:** HardBlock

A function, command, handler, or UI component that contains placeholder logic (hardcoded return values, not-yet-implemented messages, empty bodies, TODO comments as the only logic) MUST NOT be counted toward acceptance criteria. Stub implementations must be explicitly listed as incomplete in HANDOFF.md.

### R-21.4 — Phased build plan exit gates are binding

**Severity:** SoftBlock

When a backlog item specifies a phased build plan with exit gates, each gate must be verified before advancing to the next phase. Skipping phases or advancing with a failing gate is prohibited. If a gate cannot be met, the session must stop and document why in HANDOFF.md.

### R-21.5 — No execution with unsatisfied dependencies or unresolved decisions

**Severity:** HardBlock

A backlog item MUST NOT be started if: (1) any item listed in its Depends-on field has a status other than Shipped, (2) the item references an open question (OQ) that has not been resolved in OPEN_QUESTIONS.md, or (3) the item spec contains decision placeholders (TBD, to be decided, pending decision). Before beginning work, the agent must verify all dependencies are shipped and all referenced decisions are resolved. If a dependency is missing, the agent must either execute the dependency first or stop and document the blocker in HANDOFF.md.

### R-21.6 — Every backlog item must define acceptance criteria

**Severity:** HardBlock

A backlog item MUST NOT be executed unless it has explicit, numbered acceptance criteria. Each criterion must be verifiable against the implementation (testable, observable, or auditable). Vague criteria like 'works correctly' or 'is implemented' are not acceptable. If a backlog item lacks acceptance criteria, the agent must draft them and get operator approval before starting work.

Acceptance criteria must be written from the perspective of the user or business outcome — not the technical implementation. "The flag field is persisted to the database" is a technical detail, not an acceptance criterion. "A carrier whose insurance is unverified shows a warning status visible to brokers" is an acceptance criterion.

### R-21.7 — No session completion without full acceptance criteria review

**Severity:** HardBlock

A session that implements a backlog item MUST NOT complete (close protocol) until every acceptance criterion has been reviewed and verified against the actual implementation. The agent must read the relevant source code for each criterion and report PASS, PARTIAL, or FAIL. Sessions with any FAIL criteria must document the failures in HANDOFF.md and must NOT mark the backlog item as Shipped.

### R-21.8 — Session close must include compliance checklist

**Severity:** HardBlock

When the session close protocol runs, the agent MUST output a compliance checklist showing: (1) the backlog item ID, (2) each acceptance criterion with its PASS/PARTIAL/FAIL status, (3) the file or command used to verify each criterion, and (4) a summary count (N PASS, N PARTIAL, N FAIL). This checklist is part of the session close output and must appear before the FRAMEWORK SESSION CLOSED line.

### R-21.9 — Backlog status must be updated when work ships

**Severity:** HardBlock

When a session implements a backlog item and all acceptance criteria pass verification, the agent MUST update the item's **Status:** line in docs/planning/BACKLOG.md from Not started (or In Progress) to Shipped with the commit hash, session number, and date. This update is part of the session close protocol and must occur in the same commit as the state file updates.

## PlatformIntegrity

### R-22.1 — No hardcoded filesystem paths

**Severity:** HardBlock

Absolute filesystem paths to external tools, scan roots, project directories, or user-specific locations MUST NOT be embedded in source code. Paths must come from configuration (environment variables, app settings, or user input). Use relative paths and your platform's path-join API for path construction. Constants like DEFAULT_PORT are acceptable; constants like DEFAULT_PROJECT_DIR = "C:\\Projects" are not.

### R-22.2 — Magic strings must be constants or registry entries

**Severity:** SoftBlock

Provider names, model IDs, API endpoint paths, and pricing data must be defined as named constants, enum variants, or registry entries — not scattered as string literals across multiple files. If the same string appears in more than one file, it must be extracted to a shared constant.

### R-22.3 — External tool output schemas must be verified

**Severity:** SoftBlock

Before writing deserialization code for external tool output (CLI stdout, API responses, webhook payloads), verify the actual schema against real output. Silently swallowing parse failures is prohibited for external data — log or surface the mismatch. Types used to deserialize external tools must include a comment citing the source of the schema definition.

### R-22.4 — Cross-platform path construction

**Severity:** Advisory

Build filesystem paths with your language's path-joining API (e.g. path.join / Path.join), not string concatenation. Forward-slash literal construction breaks on Windows; backslash construction breaks on Unix.

### R-22.6 — Resource cleanup for spawned tasks and listeners

**Severity:** Advisory

Event listeners, spawned async tasks, and timers must have a cleanup path. Fire-and-forget handles must be justified with a comment explaining why cleanup is not needed, or replaced with proper lifecycle management (unlisten/removeListener, cancellation, abort handle, drop guard).

### R-22.7 — No absolute paths in persisted data

**Severity:** HardBlock

Project paths stored in the database or config files must be workspace-root-relative, never absolute. Absolute paths break silently when the workspace moves to a different drive or machine. Use Project::make_relative() before persisting and Project::resolve_path() when reading.

## BackgroundJobSafety

### R-30.1 — Background jobs must be idempotent

**Severity:** HardBlock

Every background job, worker, or async task must be safe to run more than once on the same input without corrupting data. If a job fails halfway through and retries, the end state must be identical to a clean run. Non-idempotent jobs are prohibited in production code paths.

### R-30.2 — Jobs must handle partial failure explicitly

**Severity:** SoftBlock

A job that performs multiple steps must define what happens if it fails mid-way. Options: wrap in a transaction and roll back, use a checkpoint pattern to resume, or document why partial execution is acceptable. Silent partial success is never acceptable.

### R-30.3 — Dead letter queue or failure logging required

**Severity:** SoftBlock

Every job queue must have a failure destination — a dead letter queue, error log, or alert — so failed jobs are visible and recoverable. Jobs that fail silently and disappear are prohibited.

### R-30.4 — Long-running jobs must not block the queue

**Severity:** Advisory

Jobs that may run for more than 30 seconds must use a dedicated queue and not share capacity with short-running jobs. A slow job that starves the main queue is a production incident waiting to happen.

## StagingFidelity

### R-31.1 — State the data distribution when citing staging tests

**Severity:** SoftBlock

When referencing staging test results as evidence that a feature works, you must state the approximate data volume and distribution used. "Tested on staging" with 500 records is not equivalent to testing against 500,000 production records. If staging does not reflect production data distribution, call it out explicitly.

### R-31.2 — High rollout risk features must be tested against production-scale data

**Severity:** HardBlock

Any feature tagged Rollout Risk: High must be tested — or have its impact queried — against production or a production-scale data snapshot before closing the ticket. A staging test on sparse data does not satisfy the business impact assessment requirement (R-29.1).

### R-31.3 — Document the environment gap when it exists

**Severity:** Advisory

If there is a known difference between staging and production (data volume, third-party integrations, configuration) that is relevant to the feature being shipped, document it in HANDOFF.md so the next session or reviewer knows the validation was limited.

## FeatureFlagLifecycle

### R-32.1 — Feature flags must have a defined owner and removal criteria

**Severity:** SoftBlock

Every feature flag introduced must document: who owns enabling it, what criteria must be met before it is enabled, and what the target date or trigger is for removing it. A flag with no removal plan becomes permanent technical debt.

### R-32.2 — Flags older than 90 days without activation must be reviewed

**Severity:** Advisory

A feature flag that has existed for more than 90 days without being enabled in production requires a review. Either activate it, update the timeline, or remove it. Stale flags obscure code paths and create confusion for future developers.

### R-32.3 — Removing a flag requires confirming all code paths are reachable

**Severity:** SoftBlock

Before deleting a feature flag, confirm that all code paths previously behind the flag are now active and reachable. Do not remove the flag and the inactive code path in the same commit — remove the flag first, verify, then clean up the dead code.

## IncidentLearning

### R-34.1 — SEV-1 and SEV-2 incidents must produce a rule change

**Severity:** HardBlock

Every SEV-1 or SEV-2 post-incident review must result in at least one change to the BEACON framework — a new rule, a tightened existing rule, or a new DoD checklist item. An incident that produces only action items but no framework update is not closed. See `docs/operations/INCIDENT_RESPONSE.md` for the pipeline format.

### R-34.2 — Rule changes from incidents must reference the incident

**Severity:** SoftBlock

Any rule added or modified as a result of an incident must include a note in `docs/state/DECISIONS.md` referencing the incident date, severity, and root cause. This creates a traceable history of why rules exist.

### R-34.3 — Repeated incidents of the same class indicate a missing rule

**Severity:** Advisory

If the same class of incident occurs more than once, the existing rule addressing it is insufficient. Escalate the severity of the rule, add a more specific constraint, or add a pre-ship checklist item that directly tests for the failure mode.

## RolloutSafety

### R-29.1 — Business impact assessment required before activation

**Severity:** HardBlock

Any feature that changes the status, flag, score, visibility, or eligibility of existing records must include a business impact assessment before it can be marked done. The assessment must answer: what percentage of existing records are affected on day one, and what does the user-facing change look like for those records. A feature without this assessment is incomplete regardless of technical correctness.

### R-29.2 — High-impact activations require explicit rollout plan

**Severity:** HardBlock

If a business impact assessment shows that more than 20% of active records would be affected on day one, the feature must not be activated immediately. A rollout plan is required — options include a grace period, phased activation, feature flag with manual trigger, or stakeholder communication before go-live. Document the plan in the backlog item before closing the ticket.

### R-29.3 — Code complete and activation are separate decisions

**Severity:** SoftBlock

Marking a backlog item as Shipped means the code is correct and merged — it does not mean the feature is active in production. Features with rollout risk must track activation as a separate step with its own acceptance criteria and stakeholder sign-off.

### R-29.4 — No assumptions about acceptable impact percentage

**Severity:** Advisory

Do not assume that a low percentage of affected records makes immediate activation safe. The acceptable threshold depends on the business context — a 5% impact on carrier compliance status may be just as disruptive as 90% depending on which carriers are affected and how brokers respond.

## MonorepoAndLegacySafety

### R-27.1 — Identify affected solutions before starting work

**Severity:** HardBlock

Before starting any ticket or backlog item, identify which solution(s) in the monorepo are affected. Tag the backlog item accordingly. Do not write code until the scope is solution-tagged and confirmed.

### R-27.2 — Grep for all callers before modifying existing code

**Severity:** HardBlock

Before changing any method, class, constant, or module that already exists, search the entire repo for all references to it. A caller may exist in a different solution, a rake task, a migration, or a background job. Document what you found before making the change.

### R-27.3 — Read git history before touching old files

**Severity:** SoftBlock

For any file with meaningful history (last changed more than 3 months ago, or with more than 5 commits), run `git log -p -- <file>` and understand why past changes were made before adding to them.

### R-27.4 — Check for framework side effects

**Severity:** SoftBlock

Before modifying a model, controller, or service that may have callbacks, concerns, observers, or hooks, explicitly check for them. In Rails: `before_save`, `after_commit`, `include`, `extend`, `prepend`, and any concern that reopens the class.

### R-27.5 — Prefer additive changes over in-place modification

**Severity:** Advisory

When changing existing behavior, add alongside rather than replace. Introduce a new method or code path, deprecate the old one explicitly, and remove only after confirming zero callers. Never silently change what an existing method does.

### R-27.6 — No method removal without confirmed zero callers

**Severity:** SoftBlock

Never delete a method, class, or constant without first grepping the entire repo and confirming zero references. In a monorepo, references may exist in solutions far from where the code is defined.

### R-27.7 — Cross-solution changes require explicit documentation

**Severity:** SoftBlock

Any change that touches more than one solution must be called out explicitly in the session HANDOFF.md with a description of what changed in each solution and why. Silent cross-solution changes are prohibited.

## BrandCompliance

### R-23.1 — Classification footers required on classified documents

**Severity:** SoftBlock

Documents marked with a classification level (Restricted, Confidential, Public) must include the correct classification footer text from `docs/governance/classification-footers.md`. Missing or incorrect footers are rejected.

## DocumentGovernance

### R-24.1 — Canonical directory taxonomy

**Severity:** SoftBlock

Files under docs/ must reside in one of the 14 canonical subdirectories (architecture, compliance, design, governance, integration, operations, planning, process, product, prompts, reports, requirements, standards, state). Files at docs/ root level are prohibited.

### R-24.2 — Design directory naming convention

**Severity:** SoftBlock

Files under docs/design/ must be organized in B-{NNN}-{slug}/ subdirectories with a MANIFEST.md. Loose files directly in docs/design/ are prohibited.

### R-24.3 — Markdown branding header required

**Severity:** SoftBlock

Markdown files under docs/ must include YAML frontmatter with at minimum: title, project, classification, created, and updated fields.

### R-24.4 — HTML branding header required

**Severity:** SoftBlock

HTML files under docs/ must include a comment header containing at minimum a Classification field: <!-- ... Classification: ... -->.

### R-24.5 — No version suffixes on filenames

**Severity:** SoftBlock

Document filenames must not contain version suffixes matching patterns like -v2, _v3, (2), or _V1. Versioning is managed through git history, not filename conventions.

### R-24.6 — No cross-project document references

**Severity:** Advisory

Document content must not contain absolute references to other project directories. Use relative paths within the project or reference external projects by name only.

## UserDocumentation

### R-25.1 — Every governed project must have a User Manual

**Severity:** SoftBlock

Every project onboarded to the BEACON Framework must contain a docs/user-guide/manual.html file. The session close protocol must verify the file exists and is not empty. The user manual documents end-user functionality and value. It does not replace developer-facing docs (CLAUDE.md, README, architecture docs, API specs). If the project keeps a primary manual elsewhere (e.g., portal/manual.html), the governance copy must be synced at session close.

### R-25.2 — User Manual must be updated when code changes

**Severity:** SoftBlock

If any source files were modified during the session (detected via git diff against session start), docs/user-guide/manual.html must also appear in the changeset. Exempt sessions that only modify governance docs, state files, or non-functional files (.md in docs/state/, docs/governance/, docs/planning/).

### R-25.3 — User Manual format must be consistent

**Severity:** Advisory

The user manual must follow a standard HTML template structure (header, TOC, overview, installation, configuration, usage, feature reference, troubleshooting, FAQ, release notes appendix, footer) so that format stays consistent across releases and across projects. All CSS must be inline or in a <style> block — no external stylesheets, no external JS, no CDN links. This is required for Teams compatibility.

## ReleaseProcess

### R-26.1 — Version bump commits must include a release report

**Severity:** HardBlock

When the workspace version is bumped, a release report must exist at docs/reports/Release_{version}.html covering all backlog items shipped since the previous version. The report must include commit references, session numbers, and a totals summary.

---

*BEACON Framework — customize as needed.*
