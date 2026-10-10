#!/usr/bin/env bash
# Tests supervisor.sh registration-gap against logs of the node's own event lines
# (event=registering before the commit, event=registered after).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
export BASE="$tmp"; mkdir -p "$tmp/log" "$tmp/run"
a=01900000-0000-7000-8000-00000000000a
b=01900000-0000-7000-8000-00000000000b
gap() { "$here/supervisor.sh" registration-gap; }
expect() { # expect <name> <rc> <out>
  local out rc=0; out="$(gap)" || rc=$?
  [[ "$rc" = "$2" && "$out" = "$3" ]] || { echo "FAIL $1: rc=$rc out=$out" >&2; exit 1; }
}
rm -f "$tmp/log/node.log"; expect "no log" 0 ""
: > "$tmp/log/node.log"; expect "empty log (never reached registering)" 0 ""
echo "event=registering node_id=$a" > "$tmp/log/node.log"; expect "crash in the registering/registered gap" 1 "$a"
printf 'event=registering node_id=%s\nevent=registered node_id=%s registration_revision=7\n' "$a" "$a" > "$tmp/log/node.log"
expect "registered" 0 ""
printf 'event=registering node_id=%s\nevent=registered node_id=%s registration_revision=7\nevent=registering node_id=%s\n' "$a" "$a" "$b" > "$tmp/log/node.log"
expect "latest registering lacks its registered line" 1 "$b"
echo "supervisor registration-gap: ok"
