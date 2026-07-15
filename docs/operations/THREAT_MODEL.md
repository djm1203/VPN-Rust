---
title: "THREAT MODEL"
project: vpn-rust
classification: high
created: 2026-07-14T00:00:00Z
updated: 2026-07-14T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: ThreatModel
author: Derek Martinez
---

# Threat Model — VPN-Rust

> **Companion document.** This threat model covers the **production QUIC/UDP
> point-to-point** design (decisions D-10…D-19). The
> [/docs/operations/SECURITY_MODEL.md](/docs/operations/SECURITY_MODEL.md)
> describes the *as-is prototype* (TLS-over-TCP, optional mTLS with a CA) and its
> known gaps; where the two differ, this document governs the current direction.
> Read them together — this one for the trust model and adversary analysis, that
> one for the implemented feature inventory (kill switch, DNS/IPv6 leak blocking,
> NAT).

---

## 1. Scope & system model

VPN-Rust is a **personal point-to-point VPN** (D-10): one operator-owned Linux
**server** and one operator-owned **client** (Linux, macOS, or Windows),
connected by a single QUIC/UDP tunnel. There are no third-party users, no
multi-tenancy, and no CA/PKI service. The operator controls both endpoints and
distributes trust material by hand.

The wire protocol is specified in
[/docs/standards/WIRE_PROTOCOL.md](/docs/standards/WIRE_PROTOCOL.md).

---

## 2. Assets

| Asset | Why it matters |
|-------|----------------|
| **Tunneled traffic confidentiality** | The tunnel exists to hide the client's traffic from the local/path network. Disclosure defeats the product's purpose. |
| **Tunneled traffic integrity** | Injected or modified packets could attack the client, server, or the hosts they reach. |
| **Node private keys** | The self-signed private key (`server-key.der`, and any client identity) *is* the node's identity. Theft lets an attacker impersonate the endpoint. |
| **Endpoint availability** | The tunnel should stay up / recover; loss of the tunnel is a (lesser) availability concern. |

---

## 3. Trust model

- **Pinned self-signed keypairs, no CA/PKI (D-13, D-19).** Each node has an
  `rcgen`-generated self-signed certificate. The client trusts the server by
  pinning **exactly one certificate** in a single-entry rustls root store
  (`client_config`, `src/transport/quic.rs`) — not a CA, not a chain.
- **Identity = SHA-256 fingerprint of the certificate DER** (`sha256:…`), printed
  by `keygen` and by the running server (`src/crypto/identity.rs`).
- **Trust on first use (TOFU) with out-of-band verification.** The operator copies
  the server certificate to the client and confirms the printed fingerprint over a
  channel they already trust (in person, existing SSH session, password manager).
  This is the pin that anchors all later authentication.
- **Both endpoints are trusted.** The operator owns and administers both machines;
  the design does not defend one endpoint against the other.

---

## 4. What the design protects against

| Threat | Mitigation |
|--------|------------|
| **Passive eavesdropping** on the path (ISP, café Wi-Fi, on-path observer) | All tunnel traffic — control stream and data datagrams — is inside QUIC's TLS 1.3 session. Contents are encrypted and authenticated. |
| **Tampering / packet injection** on the path | TLS 1.3 AEAD over QUIC authenticates every packet; forged or modified packets are rejected. QUIC packet-number encryption further frustrates injection. |
| **Active MITM / server impersonation** | Certificate **pinning** — the client accepts only the one certificate it pinned. An attacker cannot substitute their own self-signed cert (no CA to fool), and the TLS SAN check must also match `--server-name`. A MITM would need the server's private key. |
| **Downgrade to a weaker version** | The control handshake requires an **exact `PROTOCOL_VERSION` match**; mismatches are rejected, not silently downgraded. TLS 1.3 is the floor QUIC provides. |
| **Replayed / stale connections** | QUIC/TLS 1.3 handshake nonces and the max-idle-timeout bound connection lifetime. |

---

## 5. What the design does NOT protect against (out of scope)

- **Traffic analysis** — packet **timing, volume, and sizing** are not obscured.
  An observer can infer activity levels and correlate flows even though payloads
  are encrypted. No padding or cover traffic.
- **Endpoint compromise** — if the client or server host is compromised (malware,
  root attacker), the tunnel and its keys are exposed. VPN-Rust trusts both ends.
- **Key theft from disk** — an attacker who can read `server-key.der` (or a client
  key) can impersonate that node. On-disk protection is filesystem permissions
  only (`0600`, §6); there is no passphrase, HSM, or OS keystore integration.
- **Denial of service** — there is no rate limiting, connection throttling, or
  amplification protection. A network attacker can flood UDP/4433 or force
  reconnect churn. Single-peer scope limits blast radius but does not prevent DoS.
- **Metadata at the endpoints** — the fact that a VPN is in use, the server's IP,
  and connection times are all observable.
- **Malicious peer** — since both ends are operator-owned and mutually trusted,
  there is no defense of one endpoint against a hostile other endpoint.

---

## 6. Key handling

- **Generation:** self-signed via `rcgen` (`NodeIdentity::generate`), stored as
  **DER** files (D-19) — `server-cert.der` (public, shareable for pinning) and
  `server-key.der` (secret).
- **In memory:** the PKCS#8 private key bytes are held in a
  `Zeroizing<Vec<u8>>` so they are wiped from memory on drop
  (`src/crypto/identity.rs`).
- **On disk:** on Unix the key file is written `0600` (owner read/write only).
  On Windows, filesystem ACLs are the operator's responsibility.
- **Verification:** the SHA-256 certificate fingerprint is logged by `keygen` and
  at server start so the operator can pin and compare it out of band.
- **Rotation:** regenerate the identity (`keygen --force`) and **re-pin** on the
  client. Because trust is a direct pin (not a CA), rotating a key requires
  redistributing the new certificate — acceptable at P2P scale.

---

## 7. Residual risks & hardening backlog

These are known, accepted-for-now gaps with tracked follow-ups:

- **Payload logging at `trace`.** Diagnostic logging can emit packet bytes at
  `trace` level; release builds must not enable it. Enforcing no-payload logging
  in release and auditing trace paths is **B-028**.
- **Config validation.** Inputs (addresses, ports, MTU, paths) are not yet fully
  validated with actionable errors, and secrets must never be logged — **B-029**.
- **PMTU / fragmentation.** Inner MTU is a fixed default (1300, D-19) chosen to
  stay under the QUIC datagram limit; real path-MTU discovery is **B-016**.
  Oversized packets are dropped, not fragmented.
- **SPKI-fingerprint pinning.** Pinning is by exact certificate today; SPKI-based
  pinning (to allow key rotation without re-pin) is a possible refinement
  (B-025 family, D-19).
- **Native macOS/Windows network configuration.** `NetConfigurator` is a
  warn-noop off Linux (D-19); routes/NAT on those platforms are not yet applied
  automatically (B-022).
- **No security audit.** The codebase has not undergone an independent security
  review.
