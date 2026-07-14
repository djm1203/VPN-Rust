---
title: "PROMPT SAFETY"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-06-06T03:40:25Z
product_id: project
project_id: project
file_kind: PromptSafety
author: 
---

# Prompt Safety — project

## Principles

1. LLM agents are assistants, not autonomous actors — they follow governance rules
2. All LLM outputs must be reviewed before deployment
3. Prompts must not leak secrets, internal paths, or proprietary logic
4. User input must never be interpolated into prompts without sanitization

## Prompt Injection Defense

- Never pass raw user input directly into LLM prompts
- Sanitize and validate all user-provided content before prompt construction
- Use structured output formats (JSON schema) to constrain LLM responses
- Validate LLM outputs against expected schemas before acting on them

## Sensitive Data in Prompts

- Never include API keys, tokens, or credentials in prompts (Rule R-8.1)
- Never include PII in prompts unless required for the specific task
- Redact file paths that reveal infrastructure details
- Review prompt templates for information leakage

## Agent Governance

- Agents must follow the session protocol (SESSION_PROTOCOL.md)
- Agents must not self-approve changes (Rule R-11.3)
- Agents must read governance files before making changes (Rule R-11.1)
- Agents must not push code without explicit instruction (Rule R-10.2)

## Output Validation

- Parse LLM responses with strict schemas
- Reject responses that don't match expected format
- Log prompt/response pairs for audit (without secrets)
- Rate-limit LLM calls to prevent abuse
