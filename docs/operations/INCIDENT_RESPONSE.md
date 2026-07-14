---
title: "INCIDENT RESPONSE"
project: vpn-rust
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: vpn-rust
project_id: vpn-rust
file_kind: IncidentResponse
author: Derek Martinez
---

# Incident Response — VPN-Rust

> **Scope note:** This is a single-maintainer, learning-focused project. There is
> no 24/7 on-call, no paging, and no SLA. "Incident response" here is a practical
> troubleshooting runbook for the person running the VPN (Derek Martinez), plus a
> lightweight severity guide. Escalation goes to the maintainer; see
> `docs/process/ESCALATION_POLICY.md`.

## Severity Levels (right-sized for this project)

| Level | Description | Target Response | Example |
|-------|-------------|-----------------|---------|
| SEV-1 | Security exposure or total loss of function | Same day | Private key committed/leaked; kill switch not applying and traffic leaking |
| SEV-2 | Core tunnel broken | Best effort, days | TLS handshake fails for all clients; TUN cannot be created |
| SEV-3 | Degraded / intermittent | When convenient | Occasional reconnects, one client cannot connect |
| SEV-4 | Cosmetic / docs | Backlog | TUI layout glitch, log wording |

## Common Incidents & Responses

### TUN interface creation fails
Symptoms: error creating `rustvpn0`/`rustvpn1`, "Operation not permitted".
1. Confirm you are running with privilege: `sudo cargo run --bin server` (TUN
   requires root or `CAP_NET_ADMIN`).
2. Ensure the TUN module is loaded: `lsmod | grep tun`; if absent, `sudo modprobe tun`.
3. Clear stale interfaces/state: `./cleanup_vpn.sh`
   (or `sudo ip link delete rustvpn0` / `rustvpn1`).
4. Check for a conflicting VPN already holding the interface.

### TLS connection fails
Symptoms: "TLS handshake … failed", "Cannot open certs/…", cert parse errors.
1. Verify cert files exist under `certs/` (`server.crt`, `server.key`; for mTLS
   also `ca.crt`, `client.crt`, `client.key`).
2. Regenerate if missing/expired: `./gen_certs.sh` (self-signed, 365-day server
   cert; provision CA/client material for mTLS).
3. Confirm the server is up and listening on TCP **4433**
   (`ss -ltn 'sport = :4433'`).
4. Check firewall allows 4433 between client and server.
5. For mTLS: ensure the client cert is signed by the CA the server trusts
   (`certs/ca.crt`); "CA certificate required for client authentication" means
   the server was started in mTLS mode without a CA path.

### Build errors
1. `cargo clean` then rebuild.
2. Check the toolchain: `rustc --version` (Rust 2021, 1.56+); `cargo update` if
   dependencies are stale.
3. **Windows is not supported** — the build is Linux-only today; build/run on
   Linux (or WSL). This is a known limitation, not a defect to chase.

### Permission denied
- TUN and all `iptables`/`ip`/`sysctl` operations need root. Run under `sudo`,
  or grant `CAP_NET_ADMIN` to the binary.

### Connection drops / reconnects
- The client's keepalive/reconnect logic handles transient drops automatically;
  watch the logs (`RUST_LOG=info`/`debug`) for reconnect cadence.
- Persistent churn: check server-side idle reaping (`remove_idle_clients`) and
  network stability; correlate register/unregister log lines.

### Kill switch / DNS / IPv6 leak concerns
- If traffic appears to leak, verify the rules are actually installed:
  `sudo iptables -L VPN_KILLSWITCH -v`, `sudo iptables -L VPN_DNS`,
  `sudo ip6tables -L VPN_IPV6_BLOCK`. Missing chains usually mean the feature
  failed to enable (no root) — errors are logged, not fatal, so **check logs**.
- To restore normal networking, call the disable path or run cleanup; the
  `SecurityManager`/`KillSwitch`/`DnsLeakPrevention` types also restore backed-up
  rules and `/etc/resolv.conf` on `Drop`. If a process was killed mid-run,
  manually flush the leftover chains and restore `/etc/resolv.conf`.

## Recovery / Reset

`./cleanup_vpn.sh` is the primary reset: it removes the TUN interfaces and clears
VPN-related state so you can start fresh. After an abnormal exit also verify:
- No leftover `VPN_KILLSWITCH` / `VPN_DNS` / `VPN_IPV6_BLOCK` chains
  (`iptables -F <chain>; iptables -X <chain>`, same for `ip6tables`).
- NAT `MASQUERADE` rule removed if the server won't be restarted
  (`cleanup_nat`).
- `/etc/resolv.conf` restored to the system default.

## Escalation

Single maintainer: **Derek Martinez** (derekjm1203@gmail.com). File issues on
GitHub; use private contact for anything security-sensitive (leaked key,
suspected compromise) rather than a public issue. Follow
`docs/process/ESCALATION_POLICY.md`.

## Post-Incident (SEV-1 / SEV-2)

Keep it lightweight but do it: record what happened, root cause, and the fix in
`docs/state/DECISIONS.md` and `docs/state/CHANGELOG.md`. Per the BEACON incident
→ rule pipeline, a SEV-1/SEV-2 should produce a rule, checklist item, or
documented exception so the same class of failure is caught next time. For a
leaked-secret incident, rotate the CA and all certs immediately.
