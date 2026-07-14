#!/usr/bin/env bash
#
# BEACON session helper (macOS / Linux).
# Manages .beacon-session.json, the marker the resume protocol and the optional
# pre-commit hook look for.
#
# Usage:
#   .beacon/session.sh start ["session goal"]   # begin a session
#   .beacon/session.sh end                       # end the session
#   .beacon/session.sh status                    # show current session
#
set -euo pipefail

cmd="${1:-status}"
root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
file="$root/.beacon-session.json"

case "$cmd" in
  start)
    goal="${2:-}"
    goal="${goal//\\/\\\\}"; goal="${goal//\"/\\\"}"   # escape for JSON
    ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
    printf '{\n  "started_at": "%s",\n  "branch": "%s",\n  "goal": "%s"\n}\n' \
      "$ts" "$branch" "$goal" > "$file"
    echo "BEACON session started ($ts). Goal: ${2:-<none>}"
    ;;
  end|stop)
    rm -f "$file"
    echo "BEACON session ended."
    ;;
  status)
    if [ -f "$file" ]; then echo "Active BEACON session:"; cat "$file"
    else echo "No active BEACON session."; fi
    ;;
  *)
    echo "usage: session.sh {start [\"goal\"] | end | status}" >&2
    exit 2
    ;;
esac
