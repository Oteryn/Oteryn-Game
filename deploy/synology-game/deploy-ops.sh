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
MANIFEST="$ROOT_BASE/ops/preprod-topology.toml" # root copy of deploy/synology-game/preprod-topology.toml
ROOT_UID=0 # plain variable, never read from the environment; only test-preprod-topology.sh overrides it

uuid='^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
number='^[0-9]{1,18}$'
fail() { echo "deploy-ops: $*" >&2; exit 2; }
check() { [[ "$2" =~ $1 ]] || fail "invalid argument"; }
# Exactly scope_assignment::valid_node_identity: 1..=256 bytes of ASCII alphanumerics and
# space . , = : _ - / @ +, with no leading or trailing space.
node_identity_ok() {
  [[ "$1" =~ ^[A-Za-z0-9\ .,=:_/@+-]{1,256}$ && "$1" != " "* && "$1" != *" " ]] || fail "invalid node identity"
}

# --- Topology manifest comparison (fail closed) ---------------------------------------------------
# toml_get <file> <section> <key>: the single value of key in [section] ("" is the top level),
# quoted or bare. Fails when the key is missing or repeated in that section.
toml_get() {
  awk -v sec="$2" -v key="$3" '
    /^[ \t]*\[/ { s = $0; gsub(/^[ \t]*\[+|\]+[ \t]*(#.*)?$/, "", s); cur = s; next }
    cur == sec && match($0, "^[ \t]*" key "[ \t]*=[ \t]*") {
      v = substr($0, RLENGTH + 1)
      if (v ~ /^"/) { sub(/^"/, "", v); sub(/".*$/, "", v) } else { sub(/[ \t]*(#.*)?$/, "", v) }
      val = v; n++
    }
    END { if (n != 1) exit 1; print val }' "$1"
}
m() { toml_get "$MANIFEST" "$1" "$2" || fail "manifest: missing or repeated [$1] $2"; }
not_placeholder() { # <label> <value>
  [[ -n "$2" && "$2" != *"<"* && "$2" != *">"* ]] || fail "manifest: placeholder or empty value for $1"
}
mv_() { local v; v="$(m "$1" "$2")"; not_placeholder "[$1] $2" "$v"; printf '%s' "$v"; } # manifest value, no placeholder
token_ok() { [[ "$2" =~ ^[A-Za-z0-9._:-]{1,64}$ ]] || fail "manifest: invalid token $1"; }
private_ipv4() { [[ "$1" =~ ^(10\.[0-9]{1,3}|192\.168|172\.(1[6-9]|2[0-9]|3[01]))\.[0-9]{1,3}(\.[0-9]{1,3})?$ ]] || fail "manifest: $2 is not a private IPv4 address"; }
expect_eq() { # <label> <file> <section> <key> <expected>
  local got
  got="$(toml_get "$2" "$3" "$4")" || fail "$1: missing or repeated [$3] $4"
  [[ "$got" = "$5" ]] || fail "$1: [$3] $4 differs from the manifest"
}
regular_root_file() { [[ -f "$1" && ! -L "$1" && "$(stat -c %u "$1")" = "$ROOT_UID" ]] || fail "missing or not root-owned: $1"; }
regular_file() { [[ -f "$1" && ! -L "$1" ]] || fail "missing or not a regular file: $1"; }

# topology_check: the manifest is valid and complete, and scope.env, node.toml and report.toml match it.
topology_check() {
  regular_root_file "$MANIFEST"; regular_file "$NODE_CONFIG"; regular_root_file "$REPORT_CONFIG"; regular_root_file "$SCOPE_FILE"
  local world channel node_id epoch lan port route_port peer endpoint src_r src_p k
  world="$(mv_ scope world_id)"; channel="$(mv_ scope channel_id)"
  check "$uuid" "$world"; check "$uuid" "$channel"
  [[ "$world" != "$channel" ]] || fail "manifest: WorldId equals ChannelId"
  node_id="$(mv_ node identity)"; node_identity_ok "$node_id"
  epoch="$(mv_ assignment epoch)"
  [[ "$epoch" =~ ^[1-9][0-9]{0,17}$ ]] || fail "manifest: assignment epoch must be a positive integer"
  lan="$(mv_ nas lan_address)"; private_ipv4 "$lan" "nas lan_address"
  [[ "$(mv_ route host)" = "$lan" ]] || fail "manifest: route host differs from nas lan_address"
  route_port="$(mv_ route port)"; [[ "$route_port" =~ ^[1-9][0-9]{0,4}$ && "$route_port" -le 65535 ]] || fail "manifest: invalid route port"
  port="$(mv_ platform internal_mtls_port)"; [[ "$port" =~ ^[1-9][0-9]{0,4}$ && "$port" -le 65535 ]] || fail "manifest: invalid platform port"
  peer="$(mv_ platform peer_name)"; endpoint="$lan:$port"
  src_r="$(mv_ readiness source_authority)"; src_p="$(mv_ platform source_authority)"
  [[ "$src_r" = "oteryn:runtime:synology-preprod" ]] || fail "manifest: unexpected readiness source_authority"
  [[ "$src_p" = "platform" ]] || fail "manifest: unexpected platform source_authority"
  [[ "$src_r" != "$src_p" ]] || fail "manifest: readiness and platform source_authority must differ"
  for k in route_revision runtime_observation_revision ruleset_revision content_revision map_revision world_policy_revision offer_revision; do
    token_ok "readiness $k" "$(mv_ readiness "$k")"
  done
  [[ "$(mv_ readiness route_revision)" =~ ^rt\. ]] || fail "manifest: route_revision must be a Platform rt.* revision"
  [[ "$(mv_ route route_revision)" = "$(mv_ readiness route_revision)" ]] || fail "manifest: route and readiness route_revision differ"
  mv_ route route_version >/dev/null; mv_ route tls_server_name >/dev/null
  for k in profile_revision ruleset_revision content_revision starter_template_revision; do token_ok "interpretation $k" "$(mv_ interpretation "$k")"; done
  [[ "$(mv_ interpretation ruleset_revision)" = "$(mv_ readiness ruleset_revision)" && "$(mv_ interpretation content_revision)" = "$(mv_ readiness content_revision)" ]] ||
    fail "manifest: interpretation and readiness ruleset/content revisions differ"

  [[ "$(sed -n 's/^WORLD_ID=//p' "$SCOPE_FILE")" = "$world" && "$(sed -n 's/^CHANNEL_ID=//p' "$SCOPE_FILE")" = "$channel" &&
    "$(sed -n 's/^NODE_IDENTITY=//p' "$SCOPE_FILE")" = "$node_id" ]] || fail "scope.env differs from the manifest"

  expect_eq node.toml "$NODE_CONFIG" scope world_id "$world"
  expect_eq node.toml "$NODE_CONFIG" scope channel_id "$channel"
  [[ "$(toml_get "$NODE_CONFIG" listener address || true)" == *":$route_port" ]] || fail "node.toml: [listener] address port differs from the manifest"
  for k in source_authority route_revision runtime_observation_revision ruleset_revision content_revision map_revision world_policy_revision offer_revision; do
    expect_eq node.toml "$NODE_CONFIG" readiness "$k" "$(m readiness "$k")"
  done
  expect_eq node.toml "$NODE_CONFIG" platform source_authority "$src_p"
  expect_eq node.toml "$NODE_CONFIG" platform endpoint "$endpoint"
  expect_eq node.toml "$NODE_CONFIG" platform peer_name "$peer"
  expect_eq node.toml "$NODE_CONFIG" platform.runtime_status assignment_epoch "$epoch"
  expect_eq report.toml "$REPORT_CONFIG" "" assignment_epoch "$epoch"
  expect_eq report.toml "$REPORT_CONFIG" "" endpoint "$endpoint"
  expect_eq report.toml "$REPORT_CONFIG" "" peer_name "$peer"
  expect_eq report.toml "$REPORT_CONFIG" scope world_id "$world"
  expect_eq report.toml "$REPORT_CONFIG" scope channel_id "$channel"
}

# Sourced only by test-preprod-topology.sh: define the functions and stop before the root-only commands.
if [[ "${BASH_SOURCE[0]}" != "$0" ]]; then return 0; fi

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
    topology_check
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
