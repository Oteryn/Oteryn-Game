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
# Exactly scope_assignment::valid_node_identity: 1..=256 bytes of ASCII alphanumerics and
# space . , = : _ - / @ +, with no leading or trailing space.
node_identity_ok() {
  [[ "$1" =~ ^[A-Za-z0-9\ .,=:_/@+-]{1,256}$ && "$1" != " "* && "$1" != *" " ]] || fail "invalid node identity"
}

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
    check "$uuid" "$world"; check "$uuid" "$channel"; node_identity_ok "$identity"
    [[ -f "$REPORT_CONFIG" && ! -L "$REPORT_CONFIG" && "$(stat -c %u "$REPORT_CONFIG")" = 0 ]] || fail "report config"
    # Report the new ownership generation to Platform with the assignment.
    args=(assignment "$1" --report-config "$REPORT_CONFIG" --node-config "$NODE_CONFIG" --node-identity "$identity"
      --request "assign-$2-$3.json" --world "$world" --channel "$channel" --node-id "$4" --revision "$5")
    ;;
  reconcile)
    # reconcile <run-id> <attempt>: reconcile the retained assignment request of that run before
    # any new assignment (the ops writer slot refuses new work while one is unreconciled).
    [[ $# -eq 2 ]] || fail "usage: reconcile <run-id> <attempt>"
    check "$number" "$1"; check "$number" "$2"
    [[ -f "$BASE/state/assign-$1-$2.json" ]] || { echo "deploy-ops: no retained request assign-$1-$2.json; nothing to reconcile"; exit 0; }
    [[ -f "$REPORT_CONFIG" && ! -L "$REPORT_CONFIG" && "$(stat -c %u "$REPORT_CONFIG")" = 0 ]] || fail "report config"
    # reconcile does not accept --node-identity (the retained request carries it).
    args=(assignment reconcile --report-config "$REPORT_CONFIG" --node-config "$NODE_CONFIG" --request "assign-$1-$2.json")
    ;;
  check-setup)
    # check-setup: verify the root-only files the runner cannot stat (ROOT_BASE/ops is 0700).
    [[ $# -eq 0 ]] || fail "usage: check-setup"
    for f in "$CONFIG" "$REPORT_CONFIG" "$SCOPE_FILE"; do
      [[ -f "$f" && ! -L "$f" && "$(stat -c %u "$f")" = 0 ]] || fail "missing or not root-owned: $f"
    done
    world="$(sed -n 's/^WORLD_ID=//p' "$SCOPE_FILE")"
    channel="$(sed -n 's/^CHANNEL_ID=//p' "$SCOPE_FILE")"
    identity="$(sed -n 's/^NODE_IDENTITY=//p' "$SCOPE_FILE")"
    check "$uuid" "$world"; check "$uuid" "$channel"; node_identity_ok "$identity"
    echo "deploy-ops: setup ok"
    exit 0
    ;;
  reconcile-launch)
    # reconcile-launch <run-id> <attempt>: the ops tool's own reconcile of an issued launch authorization.
    [[ $# -eq 2 ]] || fail "usage: reconcile-launch <run-id> <attempt>"
    check "$number" "$1"; check "$number" "$2"
    args=(authorization reconcile --file "launch-$1-$2.json")
    ;;
  *) fail "unknown command" ;;
esac
# Authorization issue and assignment do not read OTERYN_NATIVE_GAMEPLAY_MANIFEST (only content activate does).
exec "$OPS" --config "$CONFIG" "${args[@]}"
