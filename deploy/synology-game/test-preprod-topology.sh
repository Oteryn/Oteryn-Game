#!/usr/bin/env bash
# Tests deploy-ops.sh topology_check: a rendered node.toml, report.toml and scope.env must match
# preprod-topology.toml; every mismatch, placeholder, epoch 0 and WorldId == ChannelId is refused.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
export ROOT_BASE="$tmp/root" BASE="$tmp/base"
mkdir -p "$ROOT_BASE/ops" "$BASE/node"
# shellcheck source=deploy-ops.sh
source "$here/deploy-ops.sh"
ROOT_UID="$(id -u)" # the test user stands in for root
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
# World Bundle opt-in: enabled needs the bare digest as map_revision and the manifest's table in node.toml.
D=$(sed -n 's/^digest = "\(.*\)"/\1/p' "$here/preprod-topology.toml")
fill_manifest; render; printf '[world_bundle]\npath = "x"\n' >> "$NODE_CONFIG"; expect_refused "node.toml table without opt-in"
fill_manifest; sub "$MANIFEST" "s|^enabled = false|enabled = true|;s|^map_revision = .*|map_revision = \"$D\"|"; render
{ echo "[world_bundle]"; sed -n '/^\[world_bundle\]/,/^$/p' "$MANIFEST" | grep -v '^\[\|^enabled\|^#\|^$' | sed "s|<BASE>|$BASE|"; } >> "$NODE_CONFIG"
good_node="$(cat "$NODE_CONFIG")"; good_manifest="$(cat "$MANIFEST")"
run || { echo "FAIL enabled world bundle: $(cat "$tmp/err")" >&2; exit 1; }
for key in digest content_revision project_format_version world_schema_version production start_x start_y start_floor path; do
  printf '%s\n' "$good_manifest" > "$MANIFEST"; printf '%s\n' "$good_node" > "$NODE_CONFIG"
  sub "$NODE_CONFIG" "/^\[world_bundle\]/,\$s|^$key = .*|$key = \"other\"|"; expect_refused "node.toml world_bundle $key"
done
printf '%s\n' "$good_manifest" > "$MANIFEST"; printf '%s\n' "$good_node" > "$NODE_CONFIG"
sub "$MANIFEST" "s|^map_revision = .*|map_revision = \"map.00112233445566ff\"|"; sub "$NODE_CONFIG" "s|^map_revision = .*|map_revision = \"map.00112233445566ff\"|"; expect_refused "map_revision not the bundle's"
printf '%s\n' "$good_node" > "$NODE_CONFIG"; printf '%s\n' "$good_manifest" > "$MANIFEST"; sub "$MANIFEST" "s|^production = false|production = true|"; expect_refused "production bundle"
printf '%s\n' "$good_manifest" > "$MANIFEST"; sub "$NODE_CONFIG" "/^\[world_bundle\]/,\$d"; expect_refused "enabled but node.toml has no table"
echo "preprod topology check: ok"
