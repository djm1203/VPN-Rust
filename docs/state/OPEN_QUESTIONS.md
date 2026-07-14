---
title: "OPEN QUESTIONS"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: OpenQuestions
author: Derek Martinez
---

# Open Questions — VPN-Rust

## OQ-1: Application-level keepalive vs TCP keepalive

**Blocking:** No
**Target:** Derek Martinez
**Status:** Resolved

Whether to detect liveness at the application level or rely on TCP keepalive. **Resolved:**
application-level keepalive was chosen — a length=0 marker sent every 10s with a 30s inactivity
timeout and reconnect. Recorded here for history.

## OQ-2: Inner-MTU / PMTU strategy for QUIC datagrams

**Blocking:** No
**Target:** Derek Martinez
**Status:** Partially resolved

Inner MTU is now **configurable, default 1300** (chosen to stay under the QUIC datagram limit on
typical paths), and the control handshake negotiates the smaller of the two peers' MTUs. Still
open: proper **path-MTU discovery** for the QUIC datagram limit and clamping to it, so oversized
packets are handled rather than dropped-with-a-warning (tracked as B-016).

## OQ-3: UDP transport for lower latency

**Blocking:** No
**Target:** Derek Martinez
**Status:** Resolved

Should the tunnel move to UDP to avoid TCP-over-TCP head-of-line blocking? **Resolved (D-11):** the
transport moves to **QUIC over UDP** via `quinn` — tunneled IP packets ride unreliable QUIC
datagrams, a reliable QUIC control stream carries the handshake/keepalive/config, and TLS 1.3
(rustls under quinn) is retained. This supersedes TLS-over-TCP (implemented in M1).

## OQ-4: Full-tunnel default-route + kill-switch parity across OSes

**Blocking:** No
**Target:** Derek Martinez
**Status:** Open

Should the client support split tunneling as well as full default-route capture, and — the harder
question for the production direction — how do we achieve **parity** for full-tunnel default-route
plus kill-switch behavior across Linux, macOS, and Windows? Each OS configures routes/firewall
differently; the `NetConfigurator` trait (D-16) needs per-OS implementations with guaranteed
rollback. Resolve during M2.

## OQ-5: Compression (e.g., LZ4)

**Blocking:** No
**Target:** Derek Martinez
**Status:** Open

Should payloads be optionally compressed (LZ4) before encryption to improve throughput on
compressible traffic? Note the interaction between compression and encryption (CRIME-style risks).

## OQ-6: IPv6 tunneling

**Blocking:** No
**Target:** Derek Martinez
**Status:** Open

The tunnel carries IPv4 today and includes IPv6 *leak prevention* (blocking). Should it instead
carry IPv6 traffic through the tunnel, and how should dual-stack addressing be assigned?

## OQ-7: DNS handling through the tunnel

**Blocking:** No
**Target:** Derek Martinez
**Status:** Open

How should DNS be handled through the tunnel across all three client OSes — captured/forwarded
through the tunnel, assigned to the client, and restored on disconnect — rather than merely
*prevented* from leaking? This ties into the per-OS `NetConfigurator` (D-16). Resolve during
M2/M3.

## OQ-8: Cross-platform clients vs Linux-only

**Blocking:** No
**Target:** Derek Martinez
**Status:** Resolved

Should the project stay Linux-only or go cross-platform? **Resolved (D-10, D-12):** the topology is
one operator-hosted **Linux server** plus **Linux/macOS/Windows clients** the operator owns. The
Linux-only `from_raw_fd`/`AsyncFd` TUN path is replaced by a `TunDevice` trait backed by `tun-rs`
(Linux/utun/wintun). Delivered in M2.

## OQ-10: Product scope — personal point-to-point vs multi-user

**Blocking:** No
**Target:** Derek Martinez
**Status:** Resolved

What is the product scope for "production-ready"? **Resolved (D-10):** **personal point-to-point** —
a secure tunnel between machines the operator owns, no multi-tenant service and no PKI. The
prototype's multi-client `ClientManager`/`IpPool` is re-scoped to a single-peer session; addressing
is point-to-point (`/30` or `/31`).

## OQ-11: Role of the TUI

**Blocking:** No
**Target:** Derek Martinez
**Status:** Resolved

Is the TUI a monitoring afterthought or the main interface? **Resolved (D-17):** the TUI is the
**primary control dashboard** — connect/disconnect/reconnect controls, live throughput sparklines
and RTT gauge, peer/route panels, filterable log viewer, keybindings, help, and theming, fed by an
engine→UI event/stats channel. Built out in M4 (ratatui 0.29).

## OQ-12: Cargo workspace split (B-009)

**Blocking:** No
**Target:** Derek Martinez
**Status:** Open

Should the single `vpn-rust` crate be split into a Cargo workspace (e.g. a reusable `vpn-core`
library plus thin server/client/TUI binaries) to sharpen the `thiserror` library boundary (D-15)
and the trait seams (D-16)? Tracked as backlog item B-009; decide before the seams calcify.

## OQ-13: Linux `NetConfigurator` — netlink vs wrapping `ip`

**Blocking:** No
**Target:** Derek Martinez
**Status:** Resolved (for now)

**Resolved (D-19):** the Linux `NetConfigurator` wraps `ip`/`iptables`/`sysctl` (via
`net::route`), with rollback on drop. Netlink (`rtnetlink`) remains an optional robustness
refinement, not a blocker. macOS/Windows configurators are still a warn-noop (B-022).

## OQ-9: Non-root operation via capabilities

**Blocking:** No
**Target:** Derek Martinez
**Status:** Open

TUN creation currently requires root. Should the binary instead be granted CAP_NET_ADMIN (and
related capabilities) to run unprivileged, and how should that be packaged/documented?
