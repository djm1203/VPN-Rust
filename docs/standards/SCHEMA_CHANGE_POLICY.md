---
title: "SCHEMA CHANGE POLICY"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: SchemaChangePolicy
author: Derek Martinez
---

# Schema Change Policy — VPN-Rust

VPN-Rust has **no database**. There are no tables, columns, or SQL migrations. The
"schemas" that evolve in this project — and that this policy governs — are the three
**interface formats** that a change can silently break:

1. The **tunnel wire protocol** (on-the-wire framing).
2. The **TOML configuration schema** (`src/config/toml_config.rs`).
3. The **OpenVPN `.ovpn` parsing subset** (`src/config/ovpn.rs`).

See API_CONTRACT.md for the full definitions.

## Principles

- **Prefer additive, backward-compatible changes.** New optional keys, new tolerated
  directives, new frame types behind a version gate.
- **Breaking changes require:** a version bump, a `docs/state/CHANGELOG.md` entry, and
  a migration note describing how existing configs/deployments move forward.
- **Deprecate before removal.** Announce, keep working for a transition period, then
  remove.
- Changes to any of the three schemas should be reviewed against the callers and
  documented in `docs/state/DECISIONS.md` when they alter compatibility.

## 1. Tunnel Wire Protocol

**Current state:** unversioned. The frame is a bare 2-byte big-endian length prefix +
IP packet, with `length == 0` reserved as the keepalive marker. There is no version,
type, or sequence field, so a receiver **cannot detect a mismatch** — a changed frame
format produces silent misframing.

**Policy:**

- Because the protocol is unversioned, **client and server must be built from the
  same commit.** Any framing change today is inherently breaking and requires a
  coordinated redeploy of both ends. State this in the CHANGELOG entry.
- **Introduce a version/type header before making further breaking changes.** The
  planned extended protocol (version + type + length + sequence, see API_CONTRACT.md
  §5) is the mechanism: once the first byte is a version discriminator, receivers can
  distinguish legacy framing from new framing and negotiate, ending the
  same-build-only constraint.
- Preserve the reserved meaning of `length == 0` (keepalive) unless the versioned
  header supersedes it; if it does, document the replacement.
- Do not shrink `MAX_PACKET_SIZE` semantics or change endianness without a version
  bump.

| Change | Compatible? | Process |
|--------|-------------|---------|
| Add a versioned header (first byte = version) | Bridging | Ship both ends; version byte enables future negotiation |
| Change length field size/endianness | No | Version bump + CHANGELOG + coordinated redeploy |
| Add a new frame type (under versioned header) | Yes | Additive once versioning exists |
| Repurpose the `length == 0` keepalive | No | Version bump + migration note |

## 2. TOML Configuration Schema

**Current state:** serde-parsed `[server]` / `[client]` tables where **every field
has a default** (`#[serde(default = ...)]`), both sections are `Option`, and an empty
file is valid. Load-time semantic **validation is currently deferred** — values are
accepted as parsed and only fail later at bind/interface/TLS time.

**Policy:**

- **Add new keys additively, always with a serde default,** so older config files
  keep parsing unchanged. This is the backward-compatibility guarantee.
- **Never remove or rename a key in place.** To remove: mark deprecated in
  `config.example.toml` and docs, keep honoring it for a transition period, then
  remove with a CHANGELOG entry and migration note. To rename: add the new key,
  accept both for a period, deprecate the old.
- **Changing a default value is a behavioral change** — call it out in the CHANGELOG,
  since existing files that relied on the old default will shift.
- When validation is eventually added, it must be **additive and non-surprising**:
  reject only clearly-invalid values, and prefer warnings over hard failures for
  previously-accepted inputs to avoid breaking working deployments.
- Keep `config.example.toml` in sync with the struct as the canonical reference.

| Change | Compatible? | Process |
|--------|-------------|---------|
| Add key with serde default | Yes | Update struct + `config.example.toml` |
| Add key without a default | No | Give it a default instead |
| Remove a key | No | Deprecate → transition period → remove |
| Rename a key | No | Add new, accept both, deprecate old |
| Change a default value | Behavioral | CHANGELOG note (existing files shift) |

## 3. OpenVPN `.ovpn` Parsing Subset

**Current state:** a compatibility-only parser that interprets **only the `remote`
directive** (host + optional port, default 1194). Comments (`#`, `;`) and blank lines
are skipped; **all other directives are ignored gracefully.**

**Policy:**

- **Support a documented subset and ignore unknown directives gracefully** — never
  hard-fail on a directive the parser does not understand. This lets real-world
  `.ovpn` files be dropped in without editing.
- When adding support for a new directive (e.g. `proto`, inline `<ca>`), do so
  additively and document it as newly-supported in API_CONTRACT.md.
- Do not tighten parsing in a way that rejects previously-accepted files. The one
  intentional hard error — **no `remote` directive found** — should remain the only
  fatal case unless a change is documented as breaking.

| Change | Compatible? | Process |
|--------|-------------|---------|
| Interpret a previously-ignored directive | Yes | Document as newly-supported |
| Start rejecting a previously-tolerated directive | No | Version bump + CHANGELOG + migration note |
| Change the default `remote` port (1194) | Behavioral | CHANGELOG note |

## Versioning & Rollback

- Format-affecting changes ride the crate version (`vpn-rust` SemVer). A breaking
  change to any schema warrants at least a minor bump pre-1.0 and a **major** bump
  once stable.
- Every breaking change gets a `docs/state/CHANGELOG.md` entry and a migration note
  (what to change in configs, whether both ends must be redeployed).
- "Rollback" for a wire-protocol change means redeploying the previous matched build
  on both client and server; for a config change it means reverting the config file.
  Keep the prior release binary available for fast revert (see DEPLOYMENT.md).
