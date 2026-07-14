---
title: "LLM BOUNDARIES"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-06-06T03:40:25Z
product_id: project
project_id: project
file_kind: LlmBoundaries
author: 
---

# LLM Boundaries — project

## Permitted Actions

- Read any project file for context
- Edit source code when instructed
- Run tests to verify changes
- Suggest approaches and flag issues

## Actions Requiring Permission

- Git commits, push, force-push
- File deletion
- Configuration changes
- Scope expansion beyond stated task
- External network calls

## Prohibited Actions

- Invent APIs or fabricate paths (R-1.1, R-1.2)
- Edit files without reading them first (R-3.1)
- Hardcode or log secrets (R-8.1, R-8.2)
- Self-approve changes (R-11.3)
- Change behavior during refactor (R-2.1)

## Uncertainty

When unsure about an API, version, or behavior, state the uncertainty rather than guessing. Verify before referencing.
