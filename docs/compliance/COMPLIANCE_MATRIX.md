---
title: "COMPLIANCE MATRIX"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: ComplianceMatrix
author: Derek Martinez
---

# Compliance Matrix — VPN-Rust

## Governance Rule Compliance

| Rule | Description | Status | Evidence |
|------|-------------|--------|----------|
| R-1.1 | No invented APIs | Enforced | Code review, guardrails |
| R-1.2 | No fabricated paths | Enforced | Code review, guardrails |
| R-2.1 | No behavior change in refactor | Enforced | Test suite, code review |
| R-3.1 | Read before write | Enforced | Guardrails engine |
| R-8.1 | No hardcoded secrets | Enforced | Pre-commit hook, code review |
| R-8.2 | No logged secrets | Enforced | Code review |
| R-8.3 | No committed secrets | Enforced | Pre-commit hook, .gitignore |
| R-8.4 | Sanitize user input | Enforced | Code review |
| R-10.2 | No push without instruction | Enforced | Guardrails engine |
| R-11.1 | Read governance files first | Enforced | Session protocol |
| R-11.3 | No self-approval | Enforced | Code review policy |

## Project-Specific Compliance

Security and quality controls mapped to their **actual** status in this codebase.
This project is a learning-focused prototype; several controls are honestly
marked **Partial**, **Gap**, or **Not done**.

| Control | Status | Evidence / Notes |
|---------|--------|------------------|
| TLS encryption in transit | Implemented | `rustls 0.21` + `tokio-rustls`, `with_safe_defaults()` (TLS 1.2/1.3) in `src/net/tls.rs`; tunnel over TCP:4433 |
| mTLS client authentication | Implemented (opt-in) | `ServerTlsConfig::with_mtls()` → `AllowAnyAuthenticatedClient`; client CN via `get_client_cert_cn` (`x509-parser`). Default server path uses `with_no_client_auth()` unless mTLS is selected |
| Server authentication (cert verify) | Implemented | Client verifies server via `webpki-roots` and/or project CA (`certs/ca.crt`) |
| No committed secrets | Enforced | `.gitignore`; only `.crt` tracked, `.key` untracked; certs produced locally by `gen_certs.sh` |
| No hardcoded secrets in code | Enforced | Keys referenced by path constants, not embedded |
| No logged secrets/payloads | Partial | Payloads only at `trace` level; must be disabled in real use — enforced by convention, not by a guard |
| Certificate revocation (CRL/OCSP) | Gap | Not implemented; compromised client cert can't be revoked short of CA rotation |
| Input sanitization on config parsing | Partial | CIDR/IP parsing validates format (`IpPool::from_cidr`), but broader config validation is deferred (R-8.4) |
| Key protection at rest | Partial | Plaintext PEM on disk; relies on operator `chmod 600`; no zeroize in memory |
| Dependency vetting | Manual | `rustls` chosen over OpenSSL for memory safety; `Cargo.lock` pins versions; **no `cargo audit`/`cargo deny` in CI** |
| Automated vulnerability scanning | Gap | Not wired into GitHub Actions yet |
| Test coverage | Gap | Minimal — unit tests exist for `IpPool`/`ClientStats`; network/security paths largely untested |
| Security audit | Not done | No formal audit; learning project, not production-hardened |
| Cross-platform build | Gap | Linux-only; Windows build does not currently succeed |

## Regulatory Compliance

**No regulatory scope claimed.** VPN-Rust is a personal/educational project that
processes no third-party user data and makes no GDPR / SOC 2 / HIPAA / PCI-DSS
claims. The data-minimization and no-payload-logging principles are applied on
their own merit (see `DATA_GOVERNANCE.md`).

## Audit Schedule

| Audit Type | Frequency | Last Completed | Next Due |
|------------|-----------|----------------|----------|
| Dependency review (`cargo update` + advisories) | Ad hoc | — | On dependency change |
| Security self-review | Ad hoc (per security-touching change) | — | Next mTLS/security change |
| Formal external audit | Not planned | — | — |

## Evidence Collection

- Governance compliance verified against the BEACON ruleset and session
  protocol.
- All changes tracked in git; CI (GitHub Actions) runs build + test.
- Session lifecycle enforced by the optional pre-commit hook.
