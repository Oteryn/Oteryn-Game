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
WORLD_UID=1001 # owner of the staged World Bundle: the service user (same rule: plain variable, tests only)
WORLD_GID=1001
WORLD_MODE=644
WORLD_BUNDLE_FORMAT="OTERYN_WORLD_BUNDLE/v4" # crates/world-bundle FORMAT; the digest covers FORMAT, a NUL byte, then the body
WORLD_BUNDLE_MAX=1073741824 # bytes; a larger incoming file is refused
RUNNER_UID=1001 # the runner user that calls this wrapper; stage-world refuses directories it could write (tests only)

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
private_ipv4() { # exactly four decimal octets 0-255 (no leading zeros) in 10/8, 172.16/12 or 192.168/16
  local o
  [[ "$1" =~ ^(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,2})$ ]] || fail "manifest: $2 is not a dotted IPv4 address"
  for o in "${BASH_REMATCH[@]:1}"; do [[ "$o" -le 255 ]] || fail "manifest: $2 has an octet above 255"; done
  [[ "${BASH_REMATCH[1]}" = 10 || ( "${BASH_REMATCH[1]}" = 192 && "${BASH_REMATCH[2]}" = 168 ) ||
    ( "${BASH_REMATCH[1]}" = 172 && "${BASH_REMATCH[2]}" -ge 16 && "${BASH_REMATCH[2]}" -le 31 ) ]] || fail "manifest: $2 is not a private IPv4 address"
}
# report_identity_ok <section> <key|node_identities> <node identity>: the report.toml identity location holds exactly the node identity.
report_identity_ok() {
  awk -v sec="$1" -v want="$3" -v mode="$2" '
    /^[ \t]*\[/ { s = $0; gsub(/^[ \t]*\[+|\]+[ \t]*(#.*)?$/, "", s); cur = s; next }
    cur != sec { next }
    mode == "key" && $0 ~ /^[ \t]*"/ { k = $0; sub(/^[ \t]*"/, "", k); sub(/".*$/, "", k); n++; if (k == want) ok++ }
    mode == "list" && $0 ~ /^[ \t]*node_identities[ \t]*=/ { v = $0; sub(/^[^=]*=[ \t]*/, "", v); sub(/[ \t]*(#.*)?$/, "", v); n++; if (v == "[\"" want "\"]") ok++ }
    END { exit (n == ok && n >= 1) ? 0 : 1 }' "$REPORT_CONFIG"
}
expect_eq() { # <label> <file> <section> <key> <expected>
  local got
  got="$(toml_get "$2" "$3" "$4")" || fail "$1: missing or repeated [$3] $4"
  [[ "$got" = "$5" ]] || fail "$1: [$3] $4 differs from the manifest"
}
regular_root_file() { [[ -f "$1" && ! -L "$1" && "$(stat -c %u "$1")" = "$ROOT_UID" ]] || fail "missing or not root-owned: $1"; }
regular_file() { [[ -f "$1" && ! -L "$1" ]] || fail "missing or not a regular file: $1"; }

# bundle_digest <file>: prints the bundle's own digest, sha256(FORMAT 0x00 body) over every byte but the
# 32-byte trailer, and fails unless that equals the trailer (format document section 6).
bundle_digest() {
  local f="$1" got trailer
  [[ "$(stat -c %s "$f")" -ge 33 ]] || return 1
  got="$({ printf '%s\0' "$WORLD_BUNDLE_FORMAT"; head -c -32 "$f"; } | sha256sum | cut -d ' ' -f 1)"
  trailer="$(tail -c 32 "$f" | od -An -v -tx1 | tr -d ' \n')"
  [[ "$got" = "$trailer" ]] || return 1
  printf '%s' "$got"
}
# world_bundle_pin: the pinned bare 64-hex digest and the bundle path of the manifest, as "digest path".
world_bundle_pin() {
  local digest path
  digest="$(mv_ world_bundle digest)"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || fail "manifest: invalid [world_bundle] digest"
  path="$(m world_bundle path)"; path="${path//<BASE>/$BASE}"
  [[ "$path" = "$BASE"/* && "$path" != *..* ]] || fail "manifest: [world_bundle] path must be under BASE"
  printf '%s %s' "$digest" "$path"
}

# world_bundle_check: [world_bundle] is opt-in. Enabled in the manifest, node.toml must carry exactly the
# manifest's pinned table, map_revision must be the bare 64-hex digest, and the staged file must exist with
# exactly that digest; disabled, node.toml must carry none.
world_bundle_check() {
  local enabled k want pin digest
  enabled="$(m world_bundle enabled)" || fail "manifest: missing [world_bundle] enabled"
  [[ "$enabled" = true || "$enabled" = false ]] || fail "manifest: [world_bundle] enabled must be true or false"
  if [[ "$enabled" = false ]]; then
    ! grep -q '^[[:space:]]*\[world_bundle\]' "$NODE_CONFIG" || fail "node.toml: [world_bundle] present but the manifest does not enable it"
    return 0
  fi
  [[ "$(mv_ world_bundle production)" = false ]] || fail "manifest: [world_bundle] production must be false"
  pin="$(world_bundle_pin)"; digest="${pin%% *}"; want="${pin#* }"
  [[ "$(mv_ readiness map_revision)" = "$digest" ]] || fail "manifest: map_revision must be the world_bundle digest"
  expect_eq node.toml "$NODE_CONFIG" world_bundle path "$want"
  for k in digest project_format_version world_schema_version content_revision production start_x start_y start_floor; do
    expect_eq node.toml "$NODE_CONFIG" world_bundle "$k" "$(mv_ world_bundle "$k")"
  done
  # The file itself: present, a regular file, readable by the service user, and the pinned bytes.
  [[ -f "$want" && ! -L "$want" ]] || fail "world_bundle_missing: $want is not a staged regular file (run the stage-world step)"
  [[ "$(stat -c '%u:%g %a' "$want")" = "$WORLD_UID:$WORLD_GID $WORLD_MODE" ]] ||
    fail "world_bundle_wrong_owner_or_mode: $want must be $WORLD_UID:$WORLD_GID $WORLD_MODE"
  [[ "$(bundle_digest "$want" || true)" = "$digest" ]] || fail "world_bundle_digest_mismatch: $want is not the pinned bundle $digest"
}

# runner_can_write <path>: true unless a probe run as RUNNER_UID reports that the runner cannot write path.
# access(2) as that user covers mode bits, POSIX ACLs and DSM ACLs alike; a probe that cannot run counts as writable.
runner_can_write() {
  local out
  out="$(sudo -n -u "#$RUNNER_UID" -- /bin/sh -c 'if [ -w "$1" ]; then echo writable; else echo safe; fi' sh "$1" 2>/dev/null)" || true
  [[ "$out" = safe ]] && return 1
  [[ "$out" = writable ]] || echo "deploy-ops: world_bundle_runner_probe_failed: cannot check runner access to $1 as uid $RUNNER_UID" >&2
  return 0
}
# root_dir <dir>: creates dir root-owned 0755 if it is missing (its parent is already verified), then requires
# a real directory (no symlink), owned by root, without group or other write and not writable by the runner.
root_dir() {
  local d="$1" uid mode
  [[ -e "$d" || -L "$d" ]] || mkdir -m 0755 -- "$d" || fail "world_bundle_unsafe_dir: cannot create $d"
  [[ -d "$d" && ! -L "$d" ]] || fail "world_bundle_unsafe_dir: $d is a symlink or not a directory"
  read -r uid mode < <(stat -c '%u %a' -- "$d")
  [[ "$uid" = "$ROOT_UID" ]] || fail "world_bundle_unsafe_dir: $d is not root-owned (uid $uid)"
  (( (8#$mode & 8#022) == 0 )) || fail "world_bundle_unsafe_dir: $d is group or other writable (mode $mode)"
  ! runner_can_write "$d" || fail "world_bundle_unsafe_dir: $d is writable by the runner uid $RUNNER_UID (owner, group or ACL)"
}
# root_dir_chain <dir>: root_dir for BASE and then every directory below it down to dir, top-down, so each
# directory is created or checked only inside a parent the runner cannot change.
root_dir_chain() {
  local rest="${1#"$BASE"}" p="$BASE" c
  local -a parts=()
  [[ "$1" = "$BASE" || "$1" = "$BASE"/* ]] || fail "world_bundle_unsafe_dir: $1 is not under BASE"
  root_dir "$p"
  [[ -z "$rest" ]] || IFS=/ read -r -a parts <<< "${rest#/}"
  for c in ${parts[@]+"${parts[@]}"}; do
    [[ -n "$c" && "$c" != . && "$c" != .. ]] || fail "world_bundle_unsafe_dir: bad path component in $1"
    p="$p/$c"; root_dir "$p"
  done
}

# stage_world: copy the runner-provided $BASE/run/world-bundle.incoming into the manifest's bundle path.
# First the target directory and every directory from BASE down to it are created or verified as root-owned
# and not writable by the runner (no symlink, no group/other write, no ACL write), so the runner cannot swap
# any path the steps below use. The incoming file is then read once without following a symlink into a
# root-owned temp file in that directory, and only that copy is verified, chowned and renamed over the target,
# so the runner cannot change the bytes after the check. A digest mismatch, an oversize, a non-regular file
# or an unsafe directory is refused and leaves the current target untouched.
stage_world() {
  local pin digest target dir incoming tmp
  regular_root_file "$MANIFEST"
  pin="$(world_bundle_pin)"; digest="${pin%% *}"; target="${pin#* }"
  dir="${target%/*}"
  root_dir_chain "$dir"
  incoming="$BASE/run/world-bundle.incoming"
  [[ -f "$incoming" && ! -L "$incoming" ]] || fail "world_bundle_incoming_missing: $incoming"
  [[ ! -d "$target" || -L "$target" ]] || fail "world_bundle_target_is_directory: $target"
  tmp="$(mktemp -- "$dir/.world-bundle.XXXXXX")"
  # shellcheck disable=SC2064
  trap "rm -f -- '$tmp'" EXIT
  # nofollow: a symlink swapped in after the check above is not opened; nonblock: a FIFO does not hang.
  dd if="$incoming" iflag=nofollow,nonblock,count_bytes bs=1M count="$((WORLD_BUNDLE_MAX + 1))" status=none > "$tmp" ||
    fail "world_bundle_incoming_unreadable: $incoming"
  [[ "$(stat -c %s -- "$tmp")" -le "$WORLD_BUNDLE_MAX" ]] || fail "world_bundle_too_large"
  [[ "$(bundle_digest "$tmp" || true)" = "$digest" ]] || fail "world_bundle_digest_mismatch: incoming file is not the pinned bundle $digest"
  chown -h -- "$WORLD_UID:$WORLD_GID" "$tmp"; chmod -- "$WORLD_MODE" "$tmp"
  mv -f -T -- "$tmp" "$target"
  trap - EXIT
  echo "deploy-ops: world bundle staged ($digest)"
}

# topology_check: the manifest is valid and complete, and scope.env, node.toml and report.toml match it.
topology_check() {
  regular_root_file "$MANIFEST"; regular_file "$NODE_CONFIG"; regular_root_file "$REPORT_CONFIG"; regular_root_file "$SCOPE_FILE"
  local world channel node_id epoch lan port route_port peer endpoint src_r src_p k
  world="$(mv_ scope world_id)"; channel="$(mv_ scope channel_id)"
  check "$uuid" "$world"; check "$uuid" "$channel"
  [[ "$world" != "$channel" ]] || fail "manifest: WorldId equals ChannelId"
  node_id="$(mv_ node identity)"; node_identity_ok "$node_id"
  [[ "$(mv_ scope channel_key)" =~ ^[a-z0-9][a-z0-9_-]{0,31}$ ]] || fail "manifest: invalid channel_key"
  for k in node_native_evidence node_runtime_status game_ops_authority character_projection; do node_identity_ok "$(mv_ certificates "$k")"; done
  [[ "$(mv_ certificates node_runtime_status)" = "$node_id" ]] || fail "manifest: node_runtime_status differs from [node] identity"
  [[ "$(mv_ certificates node_native_evidence)" != "$node_id" && "$(mv_ certificates game_ops_authority)" != "$node_id" &&
    "$(mv_ certificates character_projection)" != "$node_id" &&
    "$(mv_ certificates node_native_evidence)" != "$(mv_ certificates game_ops_authority)" &&
    "$(mv_ certificates node_native_evidence)" != "$(mv_ certificates character_projection)" &&
    "$(mv_ certificates game_ops_authority)" != "$(mv_ certificates character_projection)" ]] || fail "manifest: certificate subjects must be distinct"
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
  mv_ route route_version >/dev/null
  [[ "$(mv_ certificates platform_internal_san)" = "$peer" ]] || fail "manifest: platform_internal_san differs from platform peer_name"
  [[ "$(mv_ certificates node_gameplay_san)" = "$(mv_ route tls_server_name)" ]] || fail "manifest: node_gameplay_san differs from route tls_server_name"
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
  world_bundle_check
  expect_eq node.toml "$NODE_CONFIG" platform source_authority "$src_p"
  expect_eq node.toml "$NODE_CONFIG" platform endpoint "$endpoint"
  expect_eq node.toml "$NODE_CONFIG" platform peer_name "$peer"
  expect_eq node.toml "$NODE_CONFIG" platform.runtime_status assignment_epoch "$epoch"
  expect_eq report.toml "$REPORT_CONFIG" "" assignment_epoch "$epoch"
  expect_eq report.toml "$REPORT_CONFIG" "" endpoint "$endpoint"
  expect_eq report.toml "$REPORT_CONFIG" "" peer_name "$peer"
  expect_eq report.toml "$REPORT_CONFIG" scope world_id "$world"
  expect_eq report.toml "$REPORT_CONFIG" scope channel_id "$channel"
  report_identity_ok node_certificate_files key "$node_id" || fail "report.toml: [node_certificate_files] key differs from the manifest node identity"
  report_identity_ok scope list "$node_id" || fail "report.toml: [[scope]] node_identities differs from the manifest node identity"
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
  stage-world)
    # stage-world: install the runner-provided BASE/run/world-bundle.incoming as the pinned World Bundle.
    [[ $# -eq 0 ]] || fail "usage: stage-world"
    stage_world
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
