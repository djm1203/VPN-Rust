---
title: "VERSIONING"
project: vpn-rust
classification: high
created: 2026-07-14T00:00:00Z
updated: 2026-07-14T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: Standard
author: Derek Martinez
---

# Versioning & Compatibility Policy — VPN-Rust

This document defines how VPN-Rust versions the **crate** and the **on-the-wire
protocol**, and what compatibility guarantees each version carries. It applies
to `vpn-rust` (the crate/binary) and cross-references the on-the-wire contract
in [WIRE_PROTOCOL.md](/docs/standards/WIRE_PROTOCOL.md).

---

## 1. Crate versioning (SemVer)

The crate version in `Cargo.toml` follows [Semantic Versioning
2.0.0](https://semver.org/): `MAJOR.MINOR.PATCH`.

- **MAJOR** — incompatible public-API changes.
- **MINOR** — backwards-compatible functionality added.
- **PATCH** — backwards-compatible bug fixes.

### Pre-1.0 caveat (current phase)

The crate is currently **pre-1.0** (`0.y.z`). Under SemVer, the leading `0`
means the public API is **not yet stable**:

- **Minor** bumps (`0.1.z → 0.2.0`) MAY contain breaking changes — to the Rust
  API *and* to the wire protocol.
- **Patch** bumps (`0.1.0 → 0.1.1`) are reserved for backwards-compatible fixes
  that do not change the wire protocol.

Do not treat any `0.y` release as a stability promise. Stability guarantees
begin at `1.0.0`.

---

## 2. Wire-protocol compatibility

The protocol spoken between peers is versioned independently of the crate by
the constant [`PROTOCOL_VERSION`](/docs/standards/WIRE_PROTOCOL.md) (defined in
`src/transport/control.rs`, **currently `1`**). It is exchanged in the control
handshake (`ClientHello` / `ServerHello`).

### Exact-match requirement (pre-1.0)

Until the crate reaches `1.0.0`, `PROTOCOL_VERSION` **must match exactly**
between the two peers. The handshake enforces this: a client and server that
report different `PROTOCOL_VERSION` values abort the connection rather than
attempt a downgrade or partial negotiation. There is **no backwards or forwards
compatibility window** in this phase.

### Matched-build guarantee

Because the protocol is not yet stable, the only supported configuration is a
**matched build on both ends**: run the *same released version* of VPN-Rust on
the client and the server. Mixing versions — even two versions that happen to
share the same `PROTOCOL_VERSION` — is not supported before `1.0.0`, since
framing and message details may shift within a protocol version during the
pre-stable phase. This mirrors the "matched build" status note at the top of
[WIRE_PROTOCOL.md](/docs/standards/WIRE_PROTOCOL.md).

At `1.0.0` the protocol stabilizes and this policy will be replaced with an
explicit compatibility range (documented in WIRE_PROTOCOL.md).

---

## 3. What constitutes a breaking wire change

Any of the following is a **breaking wire change** and **MUST** bump
`PROTOCOL_VERSION`:

- **Control messages** — adding, removing, reordering, retyping, or changing the
  encoding of any field in `ClientHello`, `ServerHello`, or any other control
  message, or adding/removing a control message.
- **Framing** — any change to how datagrams or control data are delimited,
  length-prefixed, or laid out on the wire.
- **Datagram semantics** — any change to what a data-plane datagram carries or
  how its contents are interpreted (e.g. the one-IP-packet-per-datagram
  invariant, MTU handling, or the meaning of negotiated `SessionParams`).

A change that a peer running the previous `PROTOCOL_VERSION` cannot correctly
parse or interpret is, by definition, breaking. When in doubt, bump.

Purely internal changes that do not alter bytes on the wire (refactors,
logging, performance, new metrics) do **not** bump `PROTOCOL_VERSION`.

---

## 4. Releases & tagging

- Releases are tagged `vX.Y.Z` (e.g. `v0.1.0`), matching the crate version in
  `Cargo.toml`.
- Pushing a `vX.Y.Z` tag drives the release/packaging pipeline and produces the
  corresponding GitHub Release; see
  [/packaging/README.md](/packaging/README.md) for the build/publish flow.
- A release whose changes include a **breaking wire change** (Section 3) must
  land the `PROTOCOL_VERSION` bump in the same release, and the release notes
  must call out that peers require a matched upgrade.

---

## 5. Checklist for a wire-affecting change

1. Determine whether the change is breaking per Section 3.
2. If breaking, bump `PROTOCOL_VERSION` in `src/transport/control.rs`.
3. Update [WIRE_PROTOCOL.md](/docs/standards/WIRE_PROTOCOL.md) to reflect the new
   protocol shape and version.
4. Bump the crate version (minor while pre-1.0) in `Cargo.toml`.
5. Note the matched-build requirement in the release notes.
6. Tag `vX.Y.Z` to release.
