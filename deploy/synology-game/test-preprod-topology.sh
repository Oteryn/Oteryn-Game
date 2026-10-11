#!/usr/bin/env bash
# Tests deploy-ops.sh topology_check: a rendered node.toml, report.toml and scope.env must match
# preprod-topology.toml; every mismatch, placeholder, epoch 0 and WorldId == ChannelId is refused.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
umask 022 # directories the test creates must not be group-writable, as stage-world requires
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
export ROOT_BASE="$tmp/root" BASE="$tmp/base"
mkdir -p "$ROOT_BASE/ops" "$BASE/node"
# shellcheck source=deploy-ops.sh
source "$here/deploy-ops.sh"
ROOT_UID="$(id -u)" # the test user stands in for root
WORLD_UID="$(id -u)"; WORLD_GID="$(id -g)" # and for the service user that owns the staged bundle
# The runner-access probe needs root to switch users; the tests keep the real one as real_runner_can_write and
# simulate a runner (owner, group or ACL) write grant on the one directory named by RUNNER_WRITABLE.
eval "real_$(declare -f runner_can_write)"
RUNNER_WRITABLE=""
runner_can_write() { [[ -n "$RUNNER_WRITABLE" && "$1" = "$RUNNER_WRITABLE" ]]; }
W=01900000-0000-7000-8000-000000000001
C=01900000-0000-7000-8000-000000000002
NODE="CN=oteryn-preprod-node-1-runtime-status"
RR=rt.1.0123456789abcdef

fill_manifest() {
  sed -e "s|<NAS_LAN_ADDRESS>|192.168.10.5|" -e "s|<WORLD_ID>|$W|" -e "s|<CHANNEL_ID>|$C|" -e "s|<GAME_PORT>|7172|" \
    -e "s|<ROUTE_VERSION>|1|" -e "s|<ROUTE_REVISION>|$RR|" -e "s|<GAME_COMMIT_12HEX>|0123456789ab|" \
    -e "s|<MAP_SHA256_16HEX>|00112233445566ff|" "$here/preprod-topology.toml" > "$MANIFEST"
}
render() { # rebuild node.toml, report.toml and scope.env from the manifest values
  r() { toml_get "$MANIFEST" "$1" "$2"; }
  sed -e "s|<LISTEN_ADDRESS>:<GAME_PORT>|0.0.0.0:7172|" -e "s|<WORLD_ID>|$W|" -e "s|<CHANNEL_ID>|$C|" \
    -e "s|<READINESS_SOURCE_AUTHORITY>|$(r readiness source_authority)|" -e "s|<ROUTE_REVISION>|$(r readiness route_revision)|" \
    -e "s|<RUNTIME_OBSERVATION_REVISION>|obs.1|" -e "s|<RULESET_REVISION>|$(r readiness ruleset_revision)|" \
    -e "s|<CONTENT_REVISION>|$(r readiness content_revision)|" -e "s|<MAP_REVISION>|$(r readiness map_revision)|" \
    -e "s|<WORLD_POLICY_REVISION>|wp.1|" -e "s|<OFFER_REVISION>|offer.1|" \
    -e "s|<PLATFORM_ENDPOINT>|192.168.10.5:8543|" -e "s|<PLATFORM_PEER_NAME>|platform-internal.preprod.oteryn.internal|" \
    -e "s|<ASSIGNMENT_EPOCH>|1|" "$here/node.toml.template" > "$NODE_CONFIG"
  sed -e "s|<PLATFORM_ENDPOINT>|192.168.10.5:8543|" -e "s|<PLATFORM_PEER_NAME>|platform-internal.preprod.oteryn.internal|" \
    -e "s|<ASSIGNMENT_EPOCH>|1|" -e "s|<WORLD_ID>|$W|" -e "s|<CHANNEL_ID>|$C|" -e "s|<NODE_IDENTITY>|$NODE|" \
    "$here/report.toml.template" > "$REPORT_CONFIG"
  printf 'WORLD_ID=%s\nCHANNEL_ID=%s\nNODE_IDENTITY=%s\n' "$W" "$C" "$NODE" > "$SCOPE_FILE"
}
run() { (topology_check) >/dev/null 2>"$tmp/err"; }
expect_ok() { render; run || { echo "FAIL $1: $(cat "$tmp/err")" >&2; exit 1; }; }
expect_refused() { # <name> (state already mutated)
  if run; then echo "FAIL $1: accepted" >&2; exit 1; fi
  [[ -z "${VERBOSE:-}" ]] || echo "refused $1: $(cat "$tmp/err")"
}
sub() { sed -i "$2" "$1"; } # sub <file> <sed expr>

fill_manifest; expect_ok "matching configuration"

# The shipped manifest still holds placeholders and must be refused.
render; cp "$here/preprod-topology.toml" "$MANIFEST"; expect_refused "placeholder manifest"
fill_manifest; render
sub "$MANIFEST" "s|^epoch = 1|epoch = 0|"; expect_refused "epoch 0"
fill_manifest; sub "$MANIFEST" "s|^channel_id = .*|channel_id = \"$W\"|"; expect_refused "WorldId equals ChannelId"
fill_manifest; sub "$MANIFEST" "s|^lan_address = .*|lan_address = \"8.8.8.8\"|"; expect_refused "non-private address"
fill_manifest; sub "$MANIFEST" "0,/^source_authority = \"platform\"/s||source_authority = \"oteryn:runtime:synology-preprod\"|"; expect_refused "equal source authorities"
fill_manifest; sub "$MANIFEST" "s|^ruleset_revision = \"game.0123456789ab\"|ruleset_revision = \"bad token\"|"; expect_refused "token grammar"
fill_manifest; sub "$MANIFEST" "s|^route_revision = \"$RR\"|route_revision = \"<ROUTE_REVISION>\"|"; expect_refused "placeholder route_revision"

for pair in "readiness route_revision" "readiness map_revision" "readiness ruleset_revision" "readiness content_revision" \
  "readiness runtime_observation_revision" "readiness world_policy_revision" "readiness offer_revision" "readiness source_authority"; do
  fill_manifest; render
  read -r -a kv <<< "$pair"; set -- "${kv[@]}"
  sub "$NODE_CONFIG" "/^\[readiness\]/,/^\[platform\]/s|^$2 = \"[^\"]*\"|$2 = \"other.1\"|"; expect_refused "node.toml $2"
done
fill_manifest; render; sub "$NODE_CONFIG" "s|^assignment_epoch = 1|assignment_epoch = 2|"; expect_refused "node.toml epoch"
fill_manifest; render; sub "$REPORT_CONFIG" "s|^assignment_epoch = 1|assignment_epoch = 2|"; expect_refused "report.toml epoch"
fill_manifest; render; sub "$REPORT_CONFIG" "s|^assignment_epoch = 1|assignment_epoch = 0|"; expect_refused "report.toml epoch 0"
fill_manifest; render; sub "$NODE_CONFIG" "s|^world_id = .*|world_id = \"$C\"|"; expect_refused "node.toml world"
fill_manifest; render; sub "$REPORT_CONFIG" "s|^channel_id = .*|channel_id = \"$W\"|"; expect_refused "report.toml channel"
fill_manifest; render; sub "$NODE_CONFIG" "s|^<NONE>||;s|^endpoint = .*|endpoint = \"192.168.10.6:8543\"|"; expect_refused "node.toml endpoint"
fill_manifest; render; sub "$REPORT_CONFIG" "s|^peer_name = .*|peer_name = \"other.internal\"|"; expect_refused "report.toml peer"
fill_manifest; render; sub "$SCOPE_FILE" "s|^NODE_IDENTITY=.*|NODE_IDENTITY=CN=other|"; expect_refused "scope.env identity"
fill_manifest; render; sub "$NODE_CONFIG" "s|^address = \"0.0.0.0:7172\"|address = \"0.0.0.0:7173\"|"; expect_refused "node.toml listener port"
fill_manifest; render; sub "$NODE_CONFIG" "s|^\[readiness\]|[readiness]\nmap_revision = \"dup\"|"; expect_refused "repeated key"
fill_manifest; render; sub "$NODE_CONFIG" "s|^<NONE>||;/^route_revision/d"; expect_refused "missing key"
fill_manifest; render; real_node="$NODE_CONFIG"; ln -sf "$NODE_CONFIG" "$tmp/link"; NODE_CONFIG="$tmp/link"; expect_refused "node.toml symlink"; NODE_CONFIG="$real_node"; rm -f "$tmp/link"
# Review round 1: IPv4 octets, every manifest field and identity relation, both report identity locations.
for bad in 10.0.0.999 192.168.1 10.1.2.3.4 172.32.0.1 172.15.0.1 010.0.0.1 192.168.01.5 10.0.0.256; do
  fill_manifest; sub "$MANIFEST" "s|^lan_address = .*|lan_address = \"$bad\"|"; sub "$MANIFEST" "s|^host = .*|host = \"$bad\"|"; expect_refused "lan address $bad"
done
for good in 10.0.0.1 172.16.0.1 172.31.255.254 192.168.255.255; do
  fill_manifest; sub "$MANIFEST" "s|^lan_address = .*|lan_address = \"$good\"|"; sub "$MANIFEST" "s|^host = .*|host = \"$good\"|"
  render; sub "$NODE_CONFIG" "s|^endpoint = .*|endpoint = \"$good:8543\"|"; sub "$REPORT_CONFIG" "s|^endpoint = .*|endpoint = \"$good:8543\"|"
  run || { echo "FAIL private address $good: $(cat "$tmp/err")" >&2; exit 1; }
done
fill_manifest; sub "$MANIFEST" "s|^channel_key = .*|channel_key = \"<CHANNEL_KEY>\"|"; expect_refused "placeholder channel_key"
fill_manifest; sub "$MANIFEST" "s|^channel_key = .*|channel_key = \"Bad Key\"|"; expect_refused "invalid channel_key"
fill_manifest; sub "$MANIFEST" "/^game_ops_authority/d"; expect_refused "missing certificates entry"
fill_manifest; sub "$MANIFEST" "s|^character_projection = .*|character_projection = \"<SUBJECT>\"|"; expect_refused "placeholder certificates entry"
fill_manifest; sub "$MANIFEST" "s|^node_runtime_status = .*|node_runtime_status = \"CN=other\"|"; expect_refused "runtime-status subject differs from node identity"
fill_manifest; sub "$MANIFEST" "s|^node_native_evidence = .*|node_native_evidence = \"$NODE\"|"; expect_refused "evidence subject equals node identity"
fill_manifest; sub "$MANIFEST" "s|^game_ops_authority = .*|game_ops_authority = \"CN=oteryn-preprod-character-projection\"|"; expect_refused "duplicate certificate subjects"
fill_manifest; sub "$MANIFEST" "s|^platform_internal_san = .*|platform_internal_san = \"other.internal\"|"; expect_refused "platform SAN differs from peer_name"
fill_manifest; sub "$MANIFEST" "s|^node_gameplay_san = .*|node_gameplay_san = \"other.internal\"|"; expect_refused "gameplay SAN differs from tls_server_name"
fill_manifest; render; sub "$REPORT_CONFIG" "s|^\"$NODE\" = |\"CN=other\" = |"; expect_refused "report.toml node_certificate_files key"
fill_manifest; render; sub "$REPORT_CONFIG" "s|^node_identities = .*|node_identities = [\"CN=other\"]|"; expect_refused "report.toml node_identities"
fill_manifest; render; sub "$REPORT_CONFIG" "s|^node_identities = .*|node_identities = [\"$NODE\", \"CN=other\"]|"; expect_refused "report.toml extra node identity"
fill_manifest; render; sub "$REPORT_CONFIG" "s|^\(\"$NODE\" = .*\)|\1\n\"CN=other\" = \"/x\"|"; expect_refused "report.toml extra certificate key"
# The wrapper hashes with the domain the compiler writes: it must equal crates/world-bundle FORMAT, not a copy of itself.
rust_format="$(sed -n 's/^pub const FORMAT: &str = "\(.*\)";$/\1/p' "$here/../../crates/world-bundle/src/bundle.rs")"
[ -n "$rust_format" ] && [ "$WORLD_BUNDLE_FORMAT" = "$rust_format" ] || { echo "FAIL: WORLD_BUNDLE_FORMAT '$WORLD_BUNDLE_FORMAT' != crates/world-bundle FORMAT '$rust_format'"; exit 1; }
# World Bundle opt-in: enabled needs the bare digest as map_revision and the manifest's table in node.toml.
# A synthetic bundle: body, then sha256(FORMAT 0x00 body) as the 32-byte trailer. The manifest pin is set to its digest.
mkbundle() { # <file> <body text>
  printf '%s' "$2" > "$1.body"
  { printf '%s\0' "$WORLD_BUNDLE_FORMAT"; cat "$1.body"; } | sha256sum | cut -d ' ' -f 1 > "$1.digest"
  { cat "$1.body"; xxd_r="$(cat "$1.digest")"; printf '%s' "$xxd_r" | sed 's/../\\x&/g' | xargs -0 printf; } > "$1"
  rm -f "$1.body"
}
mkbundle "$tmp/good.otwb" "synthetic world bundle body, long enough to be a file"
D="$(cat "$tmp/good.otwb.digest")"
mkbundle "$tmp/other.otwb" "a different synthetic body"
fill_manifest; render; printf '[world_bundle]\npath = "x"\n' >> "$NODE_CONFIG"; expect_refused "node.toml table without opt-in"
fill_manifest; sub "$MANIFEST" "s|^enabled = false|enabled = true|;s|^map_revision = .*|map_revision = \"$D\"|;s|^digest = .*|digest = \"$D\"|"; render
{ echo "[world_bundle]"; sed -n '/^\[world_bundle\]/,/^$/p' "$MANIFEST" | grep -v '^\[\|^enabled\|^#\|^$' | sed "s|<BASE>|$BASE|"; } >> "$NODE_CONFIG"
good_node="$(cat "$NODE_CONFIG")"; good_manifest="$(cat "$MANIFEST")"
TARGET="$BASE/world/world.otbundle"
stage_file() { mkdir -p "$BASE/world"; rm -f "$TARGET"; cp "$1" "$TARGET"; chmod 644 "$TARGET"; }
# Fail closed on the file: absent, wrong bytes, symlink, wrong mode.
rm -rf "$BASE/world"; expect_refused "enabled bundle file missing"
stage_file "$tmp/other.otwb"; expect_refused "enabled bundle digest mismatch"
stage_file "$tmp/good.otwb"; printf 'x' >> "$TARGET"; expect_refused "bundle with trailing byte"
stage_file "$tmp/good.otwb"; chmod 600 "$TARGET"; expect_refused "bundle mode not 0644"
stage_file "$tmp/good.otwb"; mv "$TARGET" "$tmp/real.otwb"; ln -s "$tmp/real.otwb" "$TARGET"; expect_refused "bundle is a symlink"
stage_file "$tmp/good.otwb"; sed -i 's/^\(synthetic\)/X/' "$TARGET"; expect_refused "bundle body altered"
stage_file "$tmp/good.otwb"
run || { echo "FAIL enabled world bundle: $(cat "$tmp/err")" >&2; exit 1; }
for key in digest content_revision project_format_version world_schema_version production start_x start_y start_floor path; do
  printf '%s\n' "$good_manifest" > "$MANIFEST"; printf '%s\n' "$good_node" > "$NODE_CONFIG"
  sub "$NODE_CONFIG" "/^\[world_bundle\]/,\$s|^$key = .*|$key = \"other\"|"; expect_refused "node.toml world_bundle $key"
done
printf '%s\n' "$good_manifest" > "$MANIFEST"; printf '%s\n' "$good_node" > "$NODE_CONFIG"
sub "$MANIFEST" "s|^map_revision = .*|map_revision = \"map.00112233445566ff\"|"; sub "$NODE_CONFIG" "s|^map_revision = .*|map_revision = \"map.00112233445566ff\"|"; expect_refused "map_revision not the bundle's"
printf '%s\n' "$good_node" > "$NODE_CONFIG"; printf '%s\n' "$good_manifest" > "$MANIFEST"; sub "$MANIFEST" "s|^production = false|production = true|"; expect_refused "production bundle"
printf '%s\n' "$good_manifest" > "$MANIFEST"; sub "$NODE_CONFIG" "/^\[world_bundle\]/,\$d"; expect_refused "enabled but node.toml has no table"

# stage-world: refuses without incoming or on mismatch (target untouched), installs a pinned bundle atomically.
stage() { (stage_world) >/dev/null 2>"$tmp/err"; }
printf '%s\n' "$good_manifest" > "$MANIFEST"; printf '%s\n' "$good_node" > "$NODE_CONFIG"
mkdir -p "$BASE/run"; rm -rf "$BASE/world"; rm -f "$BASE/run/world-bundle.incoming"
if stage; then echo "FAIL stage-world: accepted without incoming file" >&2; exit 1; fi
cp "$tmp/other.otwb" "$BASE/run/world-bundle.incoming"
if stage; then echo "FAIL stage-world: accepted a digest mismatch" >&2; exit 1; fi
grep -q world_bundle_digest_mismatch "$tmp/err" || { echo "FAIL stage-world: mismatch not named" >&2; exit 1; }
[[ ! -e "$TARGET" ]] || { echo "FAIL stage-world: mismatch left a target" >&2; exit 1; }
[[ -z "$(ls -A "$BASE/world")" ]] || { echo "FAIL stage-world: mismatch left a temp file" >&2; exit 1; }
cp "$tmp/good.otwb" "$BASE/run/world-bundle.incoming"
stage || { echo "FAIL stage-world: $(cat "$tmp/err")" >&2; exit 1; }
cmp -s "$TARGET" "$tmp/good.otwb" && [[ "$(stat -c %a "$TARGET")" = 644 ]] || { echo "FAIL stage-world: wrong result" >&2; exit 1; }
run || { echo "FAIL staged bundle passes topology check: $(cat "$tmp/err")" >&2; exit 1; }
# A symlink as incoming file is refused; a stale good target survives a later mismatch.
rm -f "$BASE/run/world-bundle.incoming"; ln -s "$tmp/good.otwb" "$BASE/run/world-bundle.incoming"
if stage; then echo "FAIL stage-world: accepted a symlink" >&2; exit 1; fi
rm -f "$BASE/run/world-bundle.incoming"; cp "$tmp/other.otwb" "$BASE/run/world-bundle.incoming"
if stage; then echo "FAIL stage-world: accepted a mismatch over a good target" >&2; exit 1; fi
cmp -s "$TARGET" "$tmp/good.otwb" || { echo "FAIL stage-world: good target replaced" >&2; exit 1; }
# stage-world created the missing BASE/world root-owned 0755.
[[ "$(stat -c '%u %a' "$BASE/world")" = "$ROOT_UID 755" ]] || { echo "FAIL stage-world: created world dir not root 0755" >&2; exit 1; }

# Unsafe directories: refused before any temp file exists, the target and any symlink destination stay untouched.
cp "$tmp/good.otwb" "$BASE/run/world-bundle.incoming"
refuse_dir() { # <case> <expected error fragment>
  if stage; then echo "FAIL stage-world: accepted $1" >&2; exit 1; fi
  grep -q "world_bundle_unsafe_dir: .*$2" "$tmp/err" || { echo "FAIL stage-world $1: $(cat "$tmp/err")" >&2; exit 1; }
  if [[ -d "$BASE/world" && ! -L "$BASE/world" ]] && compgen -G "$BASE/world/.world-bundle.*" >/dev/null; then
    echo "FAIL stage-world $1: left a temp file" >&2; exit 1
  fi
  [[ -z "$(ls -A "$tmp/elsewhere" 2>/dev/null)" ]] || { echo "FAIL stage-world $1: wrote through a symlink" >&2; exit 1; }
}
safe_world() { rm -rf "$BASE/world" "$tmp/elsewhere"; mkdir -m 0755 "$BASE/world"; chmod 0755 "$BASE"; RUNNER_WRITABLE=""; }
# Runner-owned world dir: real ownership when the test runs as root, else stat reports the runner uid for it.
safe_world
if [[ "$(id -u)" = 0 ]]; then
  chown 1001:1001 "$BASE/world"; refuse_dir "runner-owned world dir" "$BASE/world is not root-owned (uid 1001)"
else
  # shellcheck disable=SC2329 # called by root_dir inside stage_world
  stat() { if [[ "$1" = -c && "${*: -1}" = "$BASE/world" ]]; then echo "1001 755"; else command stat "$@"; fi; }
  refuse_dir "runner-owned world dir (simulated owner)" "$BASE/world is not root-owned (uid 1001)"
  unset -f stat
fi
safe_world; RUNNER_WRITABLE="$BASE/world"; refuse_dir "world dir writable by the runner (ACL)" "$BASE/world is writable by the runner"
safe_world; RUNNER_WRITABLE="$BASE"; refuse_dir "BASE writable by the runner (ACL)" "$BASE is writable by the runner"
safe_world; chmod 0775 "$BASE/world"; refuse_dir "group-writable world dir" "$BASE/world is group or other writable"
safe_world; chmod 0757 "$BASE/world"; refuse_dir "world-writable world dir" "$BASE/world is group or other writable"
safe_world; chmod 1777 "$BASE/world"; refuse_dir "sticky world-writable world dir" "$BASE/world is group or other writable"
safe_world; chmod 0775 "$BASE"; refuse_dir "group-writable BASE" "$BASE is group or other writable"
safe_world; rmdir "$BASE/world"; mkdir -m 0755 "$tmp/elsewhere"; ln -s "$tmp/elsewhere" "$BASE/world"
refuse_dir "symlinked world dir" "$BASE/world is a symlink"
# A symlinked ancestor between BASE and the target directory (manifest path one level deeper).
safe_world; mkdir -m 0755 "$tmp/elsewhere"; ln -s "$tmp/elsewhere" "$BASE/deep"
sub "$MANIFEST" 's|^path = "<BASE>/world/world.otbundle"|path = "<BASE>/deep/world/world.otbundle"|'
refuse_dir "symlinked ancestor" "$BASE/deep is a symlink"
rm -f "$BASE/deep"; printf '%s\n' "$good_manifest" > "$MANIFEST"
# BASE itself a symlink.
safe_world; mkdir -m 0755 "$tmp/elsewhere"; ln -s "$BASE" "$tmp/base-link"
(BASE="$tmp/base-link"; NODE_CONFIG="$BASE/node/node.toml"; stage) && { echo "FAIL stage-world: accepted a symlinked BASE" >&2; exit 1; }
grep -q "world_bundle_unsafe_dir: $tmp/base-link is a symlink" "$tmp/err" || { echo "FAIL stage-world symlinked BASE: $(cat "$tmp/err")" >&2; exit 1; }
# The target path as a directory is refused; after all refusals the good target is still in place.
safe_world; mkdir "$TARGET"
if stage; then echo "FAIL stage-world: accepted a directory target" >&2; exit 1; fi
rmdir "$TARGET"; safe_world; stage || { echo "FAIL stage-world on a safe dir: $(cat "$tmp/err")" >&2; exit 1; }
cmp -s "$TARGET" "$tmp/good.otwb" || { echo "FAIL stage-world: wrong result on a safe dir" >&2; exit 1; }
# The real runner probe (root only, probing as nobody): root 0755 is safe, a nobody-owned dir is writable.
if [[ "$(id -u)" = 0 ]] && sudo -n -u nobody true 2>/dev/null; then
  probe_dir="$tmp/probe"; mkdir -m 0755 "$probe_dir"; chmod 0755 "$tmp"
  if (RUNNER_UID="$(id -u nobody)"; real_runner_can_write "$probe_dir"); then echo "FAIL probe: root 0755 dir reported writable" >&2; exit 1; fi
  chown nobody "$probe_dir"
  (RUNNER_UID="$(id -u nobody)"; real_runner_can_write "$probe_dir") || { echo "FAIL probe: nobody-owned dir reported safe" >&2; exit 1; }
  (RUNNER_UID=4294967290; real_runner_can_write "$probe_dir") 2>/dev/null || { echo "FAIL probe: a failed probe did not fail closed" >&2; exit 1; }
else
  echo "note: real runner-access probe not exercised (needs root and sudo -u nobody)"
fi

# The root wrapper rejects extra arguments and a non-root caller.
if bash "$here/deploy-ops.sh" stage-world extra 2>/dev/null; then echo "FAIL stage-world: accepted an argument" >&2; exit 1; fi
echo "preprod topology check: ok"
