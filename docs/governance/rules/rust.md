---
title: "RULESET — Rust pack"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-06-26T00:00:00Z
product_id: project
project_id: project
file_kind: Ruleset
author:
---

# BEACON Ruleset — Rust pack

> Stack-specific rules for Rust / Tauri projects. These **add to** the core
> ruleset in `docs/governance/RULESET.md`. Installed only when the `rust` pack
> is selected. Rule IDs are preserved from the core's original numbering.

## ConcurrencyAndSafety

### R-18.1 — No unwrap in production code

**Severity:** SoftBlock

Do not use .unwrap() or .expect() in non-test code. Use proper error handling with ? or match.

### R-18.2 — Lock scope minimization

**Severity:** Advisory

Hold RwLock/Mutex guards for the minimum duration. Clone data out of the lock scope before performing I/O or computation.

## AsyncRuntime

### R-13.3 — No blocking in async context

**Severity:** SoftBlock

Do not perform synchronous I/O, CPU-intensive computation, or thread-blocking operations inside async functions. Use spawn_blocking for blocking work.

## ApiDesign (Tauri)

### R-17.3 — Tauri commands must return Result

**Severity:** Advisory

All Tauri commands must return Result<T, String> and convert errors via .map_err(|e| e.to_string()).

## LlmBehavior (typed output)

### R-19.1 — LLM output must be typed

**Severity:** Advisory

Parse LLM responses through typed serde deserialization, not raw string processing. Use extract_json_object() + serde.
