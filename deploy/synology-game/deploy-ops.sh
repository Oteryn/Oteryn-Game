#!/usr/bin/env bash
# Root-owned wrapper (ROOT_BASE/bin/oteryn-game-deploy-ops) behind the deploy sudoers rule.
# It takes only fixed positional values, validates each with a strict regex, and calls
# oteryn-game-ops with fixed binary, config and scope. Extra arguments are rejected.
set -euo pipefail
ROOT_BASE="${ROOT_BASE:-/volume1/oteryn/game-preprod-root}"
OPS="$ROOT_BASE/bin/oteryn-game-ops"
BASE="${BASE:-/volume1/oteryn/game-preprod}"
CONFIG="$ROOT_BASE/ops/ops.toml"
REPORT_CONFIG="$ROOT_BASE/ops/report.toml"
NODE_CONFIG="$BASE/node/node.toml"
SCOPE_FILE="$ROOT_BASE/ops/scope.env" # root-owned: WORLD_ID=<uuid>, CHANNEL_ID=<uuid>, NODE_IDENTITY=<RFC 4514 subject>

uuid='^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
number='^[0-9]{1,18}$'
fail() { echo "deploy-ops: $*" >&2; exit 2; }
check() { [[ "$2" =~ $1 ]] || fail "invalid argument"; }

[[ "$(id -u)" = 0 ]] || fail "must run as root"
[[ $# -ge 1 ]] || fail "usage"
command="$1"
shift
case "$command" in
  issue)
    # issue <run-id> <attempt> [<supersedes-node-id>]
    [[ $# -eq 2 || $# -eq 3 ]] || fail "usage: issue <run-id> <attempt> [<node-id>]"
    check "$number" "$1"; check "$number" "$2"
    args=(authorization issue --file "launch-$1-$2.json" --binding "preprod-$1-$2")
    if [[ $# -eq 3 ]]; then check "$uuid" "$3"; args+=(--supersedes "$3"); fi
    ;;
  assign)
    # assign <assign|replace> <run-id> <attempt> <node-id> <revision>
    [[ $# -eq 5 ]] || fail "usage: assign <assign|replace> <run-id> <attempt> <node-id> <revision>"
    [[ "$1" = assign || "$1" = replace ]] || fail "invalid argument"
    check "$number" "$2"; check "$number" "$3"; check "$uuid" "$4"; check "$number" "$5"
    [[ -f "$SCOPE_FILE" && ! -L "$SCOPE_FILE" && "$(stat -c %u "$SCOPE_FILE")" = 0 ]] || fail "scope file"
    world="$(sed -n 's/^WORLD_ID=//p' "$SCOPE_FILE")"
    channel="$(sed -n 's/^CHANNEL_ID=//p' "$SCOPE_FILE")"
    identity="$(sed -n 's/^NODE_IDENTITY=//p' "$SCOPE_FILE")"
    check "$uuid" "$world"; check "$uuid" "$channel"; check '^[A-Za-z0-9=,._-]{1,128}$' "$identity"
    [[ -f "$REPORT_CONFIG" && ! -L "$REPORT_CONFIG" && "$(stat -c %u "$REPORT_CONFIG")" = 0 ]] || fail "report config"
    # Report the new ownership generation to Platform with the assignment.
    args=(assignment "$1" --report-config "$REPORT_CONFIG" --node-config "$NODE_CONFIG" --node-identity "$identity"
      --request "assign-$2-$3.json" --world "$world" --channel "$channel" --node-id "$4" --revision "$5")
    ;;
  *) fail "unknown command" ;;
esac
# Authorization issue and assignment do not read OTERYN_NATIVE_GAMEPLAY_MANIFEST (only content activate does).
exec "$OPS" --config "$CONFIG" "${args[@]}"
