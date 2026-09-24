#!/usr/bin/env bash
# OPS-NODE-BOOT-01 §4: physical qualification with the shipped binaries.
#
# The real Platform producer (S3-A/S3-B topology, one client identity for
# evidence and intents), a TLS-verified PostgreSQL 17.6, a system service
# user running `oteryn-game-server serve`, and root running
# `oteryn-game-ops` on a separate control-plane login. The node boots,
# waits for the operator's assignment, bootstraps Characters from real
# Platform intents through its control socket, and reproduces every #823
# SEAM stage against its own bound port.
set -Eeuo pipefail
umask 077

readonly PLATFORM_SHA=9147bfd3a771762a6646fd87b9172cdb3a6c9a01
readonly TOPOLOGY_REVISION=wp5-node-boot-v1
readonly ACCOUNT_ID=01934f10-7c00-7000-8000-000000000001
readonly SECOND_ACCOUNT_ID=01934f10-7c00-7000-8000-000000000002
readonly FRESH_KEY_ID=node-boot-fresh-key-1
readonly WORLD_ID=01934f10-7c02-7001-805b-3b1122334401
readonly CHANNEL_ID=01934f10-7c03-7001-805b-3b1122334401
readonly INTENT_OPERATIONS=(01934f10-7c04-7001-805b-3b1122334401 01934f10-7c04-7002-805b-3b1122334402)
readonly INTERPRETATION=(nb-profile-1 nb-ruleset-1 nb-content-1 nb-starter-1)
readonly COMPOSE_FILE=tools/qualification/wp5_s3a/compose.yml
readonly COMPOSE_S3B=tools/qualification/wp5_s3b/compose.override.yml
readonly COMPOSE_OVERLAY=tools/qualification/node_boot/compose.override.yml
readonly PG_IMAGE=postgres:17.6-bookworm@sha256:f3bd19c606e442c3d7bdfa8002e03fe260a1023351e0ea4598032022b68dd6e3
readonly SERVICE_USER=oteryn-node-boot
readonly BASE=/srv/oteryn-node-boot

GAME_SOURCE="$(git rev-parse --show-toplevel)"
PLATFORM_SOURCE="${PLATFORM_SOURCE:-$GAME_SOURCE/_platform}"
if [[ "$(git -C "$PLATFORM_SOURCE" rev-parse HEAD 2>/dev/null)" != "$PLATFORM_SHA" ]]; then
  echo 'NODE_BOOT_RESULT=BLOCKED reason=exact_platform_checkout_missing'
  exit 2
fi

WORK="$(mktemp -d "${RUNNER_TEMP:-/tmp}/node-boot.XXXXXX")"
WP5_PKI="$WORK/pki"
WP5_SCRATCH="$WORK/scratch"
mkdir -p "$WP5_PKI" "$WP5_SCRATCH"
WP5_PORT="${NODE_BOOT_PLATFORM_PORT:-18463}"
WP5_PROJECT="nodeboot${GITHUB_RUN_ID:-local}${GITHUB_RUN_ATTEMPT:-1}"
WP5_DB_PASSWORD="$(openssl rand -hex 24)"
WP5_DB_ROOT_PASSWORD="$(openssl rand -hex 24)"
WP5_APP_KEY="base64:$(openssl rand -base64 32 | tr -d '\n')"
WP5_TOPOLOGY_REVISION="$TOPOLOGY_REVISION"
WP5_FSYNC_FAULT=none
export GAME_SOURCE PLATFORM_SOURCE WP5_PKI WP5_SCRATCH WP5_PORT WP5_PROJECT
export WP5_DB_PASSWORD WP5_DB_ROOT_PASSWORD WP5_APP_KEY WP5_TOPOLOGY_REVISION WP5_FSYNC_FAULT
PG_CONTAINER="${WP5_PROJECT}-postgres"
PG_PORT="${NODE_BOOT_PG_PORT:-15433}"
PG_ADMIN_PASSWORD="$(openssl rand -hex 24)"
CONTROL_PASSWORD="$(openssl rand -hex 24)"
RUNTIME_PASSWORD="$(openssl rand -hex 24)"
NODE_PID=""

result=NODE_BOOT_FAIL
compose() {
  docker compose --project-name "$WP5_PROJECT" --file "$GAME_SOURCE/$COMPOSE_FILE" \
    --file "$GAME_SOURCE/$COMPOSE_S3B" --file "$GAME_SOURCE/$COMPOSE_OVERLAY" "$@"
}
evidence() { printf 'NODE_BOOT_EVIDENCE %s\n' "$*"; }
fail() { echo "NODE_BOOT_FAILURE $*"; return 1; }
cleanup() {
  local rc=$?
  [[ -z "$NODE_PID" ]] || sudo kill -TERM "$NODE_PID" 2>/dev/null || true
  sudo pkill -TERM -u "$SERVICE_USER" 2>/dev/null || true
  compose down --volumes --remove-orphans --timeout 15 >/dev/null 2>&1 || true
  docker rm --force --volumes "$PG_CONTAINER" >/dev/null 2>&1 || true
  sudo rm -rf "$BASE" || true
  find "$WORK" -type f -delete 2>/dev/null || true
  find "$WORK" -depth -type d -empty -delete 2>/dev/null || true
  evidence "cleanup=complete"
  echo "NODE_BOOT_RESULT=$result"
  exit "$rc"
}
trap cleanup EXIT INT TERM

make_ca() {
  openssl req -x509 -newkey rsa:2048 -sha256 -nodes -days 1 \
    -subj "/CN=$2" -keyout "$WP5_PKI/$1.key" -out "$WP5_PKI/$1.crt" >/dev/null 2>&1
}
make_leaf() {
  local prefix=$1 subject=$2 ca=$3 usage=$4 san=${5:-}
  openssl req -new -newkey rsa:2048 -nodes -sha256 -subj "/CN=$subject" \
    -keyout "$WP5_PKI/$prefix.key" -out "$WP5_PKI/$prefix.csr" >/dev/null 2>&1
  printf 'basicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=%s\n' "$usage" > "$WP5_PKI/$prefix.ext"
  [[ -z "$san" ]] || printf 'subjectAltName=%s\n' "$san" >> "$WP5_PKI/$prefix.ext"
  openssl x509 -req -sha256 -days 1 -in "$WP5_PKI/$prefix.csr" -CA "$WP5_PKI/$ca.crt" \
    -CAkey "$WP5_PKI/$ca.key" -CAcreateserial -extfile "$WP5_PKI/$prefix.ext" \
    -out "$WP5_PKI/$prefix.crt" >/dev/null 2>&1
}
make_ca server-ca node-boot-server-ca
make_ca client-ca node-boot-client-ca
make_ca db-ca node-boot-db-ca
make_leaf server source.test server-ca serverAuth DNS:source.test
make_leaf client oteryn-game-native-evidence client-ca clientAuth
make_leaf db db.node-boot.test db-ca serverAuth DNS:db.node-boot.test
# The nginx topology expects the S3-B intent client files; the node uses the
# single evidence identity for both routes.
cp "$WP5_PKI/client.crt" "$WP5_PKI/intent-client.crt"
cp "$WP5_PKI/client.key" "$WP5_PKI/intent-client.key"
# An end-entity certificate: TLS rejects a CA certificate used as a server leaf.
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes -days 1 \
  -subj "/CN=localhost" -addext "subjectAltName=DNS:localhost" \
  -addext "basicConstraints=critical,CA:FALSE" -addext "keyUsage=critical,digitalSignature" \
  -addext "extendedKeyUsage=serverAuth" \
  -keyout "$WP5_PKI/gameplay.key" -out "$WP5_PKI/gameplay.crt" >/dev/null 2>&1
openssl genpkey -algorithm ed25519 -out "$WP5_PKI/fresh-signing.pem" >/dev/null 2>&1
raw_hex() { tail -c 32 | od -An -v -tx1 | tr -d ' \n'; }
FRESH_PUBLIC_HEX="$(openssl pkey -in "$WP5_PKI/fresh-signing.pem" -pubout -outform DER | raw_hex)"
FRESH_SEED_HEX="$(openssl pkey -in "$WP5_PKI/fresh-signing.pem" -outform DER | raw_hex)"
evidence "pins platform=$PLATFORM_SHA topology=$TOPOLOGY_REVISION game=$(git rev-parse HEAD)"

# TLS-verified PostgreSQL 17.6 for the Game durability root.
chmod 644 "$WP5_PKI/db.crt" "$WP5_PKI/db.key"
docker run --detach --name "$PG_CONTAINER" --publish "127.0.0.1:$PG_PORT:5432" \
  --env POSTGRES_USER=oteryn_node_boot_admin --env POSTGRES_PASSWORD="$PG_ADMIN_PASSWORD" \
  --env POSTGRES_DB=postgres --volume "$WP5_PKI/db.crt:/tls/db.crt:ro" \
  --volume "$WP5_PKI/db.key:/tls/db.key:ro" --entrypoint bash "$PG_IMAGE" -c \
  'install -o postgres -m 0600 /tls/db.key /tmp/db.key && install -o postgres -m 0644 /tls/db.crt /tmp/db.crt && exec docker-entrypoint.sh postgres -c ssl=on -c ssl_cert_file=/tmp/db.crt -c ssl_key_file=/tmp/db.key' >/dev/null
chmod 600 "$WP5_PKI/db.key"
for _ in $(seq 1 60); do
  docker exec "$PG_CONTAINER" pg_isready -U oteryn_node_boot_admin -d postgres >/dev/null 2>&1 && break
  sleep 1
done
psql_admin() { docker exec -i "$PG_CONTAINER" psql -v ON_ERROR_STOP=1 -qAt -U oteryn_node_boot_admin -d "$1"; }
echo "CREATE DATABASE oteryn_node_boot" | psql_admin postgres
[[ "$(echo 'SHOW server_version_num' | psql_admin oteryn_node_boot)" == 170006 ]] || fail "postgres_version"
[[ "$(echo 'SHOW ssl' | psql_admin oteryn_node_boot)" == on ]] || fail "postgres_tls"
ADMIN_URL="postgresql://oteryn_node_boot_admin:$PG_ADMIN_PASSWORD@127.0.0.1:$PG_PORT/oteryn_node_boot"

# The Platform topology.
compose config --quiet
compose build --pull platform
compose up --detach --wait db platform nginx
# Platform output is kept: a failing operator command must be diagnosable.
php_exec() { compose exec --no-TTY --user www-data platform php -r "$1"; }
php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); foreach ([["'"$ACCOUNT_ID"'","nb-one@example.invalid"],["'"$SECOND_ACCOUNT_ID"'","nb-two@example.invalid"]] as [$accountId,$email]) { Illuminate\Support\Facades\DB::table("identities")->insert(["email"=>$email,"password"=>password_hash(bin2hex(random_bytes(24)),PASSWORD_BCRYPT),"account_id"=>$accountId,"native_security_generation"=>1,"created_at"=>now(),"updated_at"=>now()]); } app(App\GameAuth\NativeEvidence\NativeSigningTrustRegistry::class)->publishTrustedKey(App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_ISSUER,App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_PROFILE,"fresh_admission","'"$FRESH_KEY_ID"'",hex2bin("'"$FRESH_PUBLIC_HEX"'"));'
evidence "platform_seed=synthetic_accounts=2 fresh_trust=published_public_key_only client_identities=1"

# Shipped binaries and the SEAM client harness.
cargo +1.94.0 build --locked -p oteryn-game-server --bins
cargo +1.94.0 test --locked -p oteryn-game-server --lib --no-run
TARGET="$GAME_SOURCE/target/debug"

# Deployment administration: service user, directories, roles, migration.
sudo useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER" 2>/dev/null || true
SERVICE_UID="$(id -u "$SERVICE_USER")"
sudo install -d -o root -g root -m 0755 "$BASE" "$BASE/bin" "$BASE/state" "$BASE/fence-parent" "$BASE/node"
sudo install -d -o root -g root -m 0700 "$BASE/ops"
sudo install -d -o "$SERVICE_UID" -m 0700 "$BASE/fence-parent/fence" "$BASE/run"
sudo install -d -o "$SERVICE_UID" -m 0755 "$BASE/node/secrets"
sudo install -o root -m 0755 "$TARGET/oteryn-game-server" "$TARGET/oteryn-game-ops" "$TARGET/oteryn-game-migrate" "$BASE/bin/"
OTERYN_GAME_MIGRATION_DATABASE_URL="$ADMIN_URL" "$BASE/bin/oteryn-game-migrate"
psql_admin oteryn_node_boot <<SQL
CREATE ROLE nb_control LOGIN PASSWORD '$CONTROL_PASSWORD' IN ROLE oteryn_game_control;
CREATE ROLE nb_runtime LOGIN PASSWORD '$RUNTIME_PASSWORD' IN ROLE oteryn_game_runtime;
GRANT CONNECT ON DATABASE oteryn_node_boot TO nb_control, nb_runtime;
SQL
secret_to() { # owner source destination
  sudo install -o "$1" -m 0600 "$2" "$3"
}
printf '%s\n' "$CONTROL_PASSWORD" > "$WORK/control-password"
printf '%s\n' "$RUNTIME_PASSWORD" > "$WORK/runtime-password"
secret_to root "$WORK/control-password" "$BASE/ops/pg-password"
secret_to root "$WP5_PKI/db-ca.crt" "$BASE/ops/db-ca.pem"
secret_to "$SERVICE_UID" "$WORK/runtime-password" "$BASE/node/secrets/pg-password"
secret_to "$SERVICE_UID" "$WP5_PKI/db-ca.crt" "$BASE/node/secrets/db-ca.pem"
secret_to "$SERVICE_UID" "$WP5_PKI/server-ca.crt" "$BASE/node/secrets/platform-roots.pem"
secret_to "$SERVICE_UID" "$WP5_PKI/client.crt" "$BASE/node/secrets/platform-client.crt"
secret_to "$SERVICE_UID" "$WP5_PKI/client.key" "$BASE/node/secrets/platform-client.key"
secret_to "$SERVICE_UID" "$WP5_PKI/gameplay.crt" "$BASE/node/secrets/gameplay.crt"
secret_to "$SERVICE_UID" "$WP5_PKI/gameplay.key" "$BASE/node/secrets/gameplay.key"

cat > "$WORK/ops.toml" <<TOML
[operator]
state_directory = "$BASE/state"
service_uid = $SERVICE_UID
[database]
transport_ip = "127.0.0.1"
port = $PG_PORT
tls_server_name = "db.node-boot.test"
database = "oteryn_node_boot"
username = "nb_control"
password_file = "$BASE/ops/pg-password"
root_ca_file = "$BASE/ops/db-ca.pem"
[character]
fence_directory = "$BASE/fence-parent/fence"
authority_scope_id = "character-primary"
issuer_identity = "game-ops"
TOML
sudo install -o root -m 0600 "$WORK/ops.toml" "$BASE/ops/ops.toml"
# One descriptor installation time: changed facts under an unchanged
# revision are refused at boot, as D3 requires.
DESCRIPTOR_INSTALLED_AT="$(date +%s)"
write_node_config() { # launch-file
  cat > "$WORK/node.toml" <<TOML
[listener]
address = "127.0.0.1:${NODE_BOOT_GAME_PORT:-17181}"
entry_deadline_ms = 20000
max_connections = 256
max_handshake_units = 64
certificate_chain_file = "$BASE/node/secrets/gameplay.crt"
private_key_file = "$BASE/node/secrets/gameplay.key"
[scope]
world_id = "$WORLD_ID"
channel_id = "$CHANNEL_ID"
assignment_wait_ms = 120000
[database]
transport_ip = "127.0.0.1"
port = $PG_PORT
tls_server_name = "db.node-boot.test"
database = "oteryn_node_boot"
username = "nb_runtime"
password_file = "$BASE/node/secrets/pg-password"
root_ca_file = "$BASE/node/secrets/db-ca.pem"
[character]
fence_directory = "$BASE/fence-parent/fence"
authority_scope_id = "character-primary"
issuer_identity = "game-ops"
[control]
socket_path = "$BASE/run/control.sock"
[readiness]
source_authority = "oteryn:runtime:node-boot"
route_revision = "route-s3b-1"
runtime_observation_revision = "runtime-1"
ruleset_revision = "rules-s3b-1"
content_revision = "content-s3b-1"
map_revision = "map-s3b-1"
world_policy_revision = "policy-s3b-1"
offer_revision = "offer-s3b-1"
[platform]
source_authority = "platform"
endpoint = "127.0.0.1:$WP5_PORT"
peer_name = "source.test"
trust_roots_file = "$BASE/node/secrets/platform-roots.pem"
client_certificate_file = "$BASE/node/secrets/platform-client.crt"
client_key_file = "$BASE/node/secrets/platform-client.key"
descriptor_revision = 1
installed_at = $DESCRIPTOR_INSTALLED_AT
[launch]
authorization_file = "$BASE/state/$1"
s2_authorization_file = "$BASE/state/s2-fresh-store.json"
TOML
  sudo install -o root -m 0644 "$WORK/node.toml" "$BASE/node/node.toml"
}
ops() { sudo "$BASE/bin/oteryn-game-ops" --config "$BASE/ops/ops.toml" "$@"; }
node_log="$WORK/node.log"
start_node() {
  # Detached from the stage's output pipe, so the stage ends while it serves.
  sudo -u "$SERVICE_USER" "$BASE/bin/oteryn-game-server" serve --config "$BASE/node/node.toml" \
    < /dev/null > /dev/null 2> "$node_log" &
  NODE_PID=$!
}
await_log() { # pattern seconds
  for _ in $(seq 1 "$2"); do grep -q "$1" "$node_log" && return 0; sleep 1; done
  fail "missing node event: $1"
}

# Every stage runs to a recorded outcome, so one run reports every failing
# stage instead of stopping at the first one. A stage whose prerequisite did
# not pass is recorded as SKIP. State shared between stages lives in $VARS.
VARS="$WORK/vars"
: > "$VARS"
declare -A STATUS=()
ORDER=()
remember() { printf '%s=%q\n' "$1" "$2" >> "$VARS"; }
stage() { # name prerequisite... -- function
  local name=$1; shift
  local prerequisites=()
  while [[ $1 != -- ]]; do prerequisites+=("$1"); shift; done
  shift
  ORDER+=("$name")
  local missing
  for missing in "${prerequisites[@]}"; do
    if [[ ${STATUS[$missing]:-} != PASS ]]; then
      STATUS[$name]="SKIP(needs $missing)"
      return 0
    fi
  done
  echo "::group::stage $name"
  set +e
  ( set -Eeuo pipefail; source "$VARS"; "$@" ) 2>&1 | tee "$WORK/stage-$name.log"
  local rc=${PIPESTATUS[0]}
  set -e
  echo "::endgroup::"
  if [[ $rc == 0 ]]; then
    STATUS[$name]=PASS
  else
    STATUS[$name]="FAIL(exit $rc)"
    echo "NODE_BOOT_STAGE_FAILED $name exit=$rc"
    echo "--- last node events"; tail -n 40 "$node_log" 2>/dev/null || true
    echo "--- postgres"; docker logs --tail 40 "$PG_CONTAINER" 2>&1 | grep -E 'ERROR|FATAL|STATEMENT' || true
    echo "--- platform"; compose logs --no-color --tail 40 platform nginx 2>&1 || true
  fi
}
node_field() { # log field
  sed -n "s/.*awaiting_assignment.*$2=\([^ ]*\).*/\1/p" "$1" | tail -n 1
}

operator_setup() { # §4.1 operator setup before the node starts
  ops authorization issue --file launch-a.json --binding node-boot-a
  write_node_config launch-a.json
  ops s2 issue --node-config "$BASE/node/node.toml" --file s2-fresh-store.json --namespace node-boot --authorization disposable-qualification
  if ops character interpretation --profile "${INTERPRETATION[0]}" --ruleset "${INTERPRETATION[1]}" --content "${INTERPRETATION[2]}" --starter "${INTERPRETATION[3]}"; then
    fail "interpretation configured before the fresh store"
  fi
  ops character fresh-store --request fresh-store.json
  ops character fresh-store --request fresh-store.json
  ops character interpretation --profile "${INTERPRETATION[0]}" --ruleset "${INTERPRETATION[1]}" --content "${INTERPRETATION[2]}" --starter "${INTERPRETATION[3]}"
  evidence "operator=authorization,s2_issuance,fresh_store,interpretation interpretation_before_store=refused fresh_store_rerun=idempotent"
}

configuration_negatives() { # exit before binding, no secret in output
  sudo cp "$BASE/node/node.toml" "$BASE/node/insecure.toml"; sudo chmod 0666 "$BASE/node/insecure.toml"
  sudo ln -sf "$BASE/node/node.toml" "$BASE/node/link.toml"
  sudo sed '/^map_revision/d' "$BASE/node/node.toml" | sudo tee "$BASE/node/missing.toml" >/dev/null
  sudo sed 's/^max_connections = 256/max_connections = 257/' "$BASE/node/node.toml" | sudo tee "$BASE/node/over.toml" >/dev/null
  sudo chmod 0644 "$BASE/node/missing.toml" "$BASE/node/over.toml"
  local config code failed=0
  for config in insecure link missing over; do
    set +e
    sudo -u "$SERVICE_USER" "$BASE/bin/oteryn-game-server" serve --config "$BASE/node/$config.toml" 2>> "$WORK/negative.log"
    code=$?
    set -e
    echo "configuration $config exit=$code"
    [[ $code == 10 ]] || failed=1
  done
  ! grep -q "$RUNTIME_PASSWORD" "$WORK/negative.log" || { echo "secret in output"; failed=1; }
  [[ $failed == 0 ]]
  evidence "configuration group_writable=exit10 symlink=exit10 missing_key=exit10 over_maximum=exit10 secrets_in_output=0"
}

node_assigned_ready() { # §4.2–§4.4
  start_node
  remember NODE_PID "$NODE_PID"
  await_log awaiting_assignment 120
  local node revision
  node="$(node_field "$node_log" node_id)"; revision="$(node_field "$node_log" registration_revision)"
  remember node_a "$node"
  if ops assignment assign --request assign-ungranted.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --node-id "$node" --revision "$revision"; then
    fail "assignment without an exact-scope grant"
  fi
  psql_admin oteryn_node_boot <<SQL
INSERT INTO game_control_scope_grants SELECT 'nb_control', '$WORLD_ID', '$CHANNEL_ID', op FROM generate_series(1, 3) op;
SQL
  ops assignment assign --request assign-a.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --node-id "$node" --revision "$revision"
  await_log "readiness ready=true" 60
  [[ "$(sudo stat -c '%u %a' "$BASE/run/control.sock")" == "$SERVICE_UID 600" ]] || fail "control socket mode"
  [[ "$(echo 'SELECT count(*) FROM game_node_registrations' | psql_admin oteryn_node_boot)" == 1 ]] || fail "operator registered an incarnation"
  evidence "node registered=1 assignment=operator ungranted=refused readiness=true control_socket=uid${SERVICE_UID}_0600"
}

control_socket_peer() { # a non-root peer is closed without an answer
  sudo -u "$SERVICE_USER" python3 - "$BASE/run/control.sock" "${INTENT_OPERATIONS[0]}" <<'PY'
import socket, sys
s = socket.socket(socket.AF_UNIX)
s.connect(sys.argv[1])
try:
    s.sendall(sys.argv[2].encode() + b"\n")
    s.shutdown(socket.SHUT_WR)
    answer = s.recv(64)
except OSError:
    answer = b""
sys.exit(0 if answer == b"" else 1)
PY
  evidence "control_socket service_user_peer=closed"
}

character_bootstrap() { # §4.5 Characters from real Platform intents
  local index=0 account operation failed=0
  for account in "$ACCOUNT_ID" "$SECOND_ACCOUNT_ID"; do
    operation="${INTENT_OPERATIONS[$index]}"
    index=$((index + 1))
    if ! php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); $id=Illuminate\Support\Facades\DB::table("identities")->where("account_id","'"$account"'")->value("id"); $code=Illuminate\Support\Facades\Artisan::call("game-auth:character-bootstrap-intent:issue",["--identity-id"=>(string)$id,"--operation-id"=>"'"$operation"'","--target-world-id"=>"'"$WORLD_ID"'","--profile-revision"=>"'"${INTERPRETATION[0]}"'","--ruleset-revision"=>"'"${INTERPRETATION[1]}"'","--content-revision"=>"'"${INTERPRETATION[2]}"'","--starter-template-revision"=>"'"${INTERPRETATION[3]}"'"]); fwrite(STDERR, Illuminate\Support\Facades\Artisan::output()); exit($code);'; then
      echo "platform intent issue failed for operation $operation"
      failed=1
      continue
    fi
    ops character bootstrap --socket "$BASE/run/control.sock" --operation-id "$operation" || { failed=1; continue; }
    # An exact retry of the same operation id is idempotent.
    ops character bootstrap --socket "$BASE/run/control.sock" --operation-id "$operation" || failed=1
  done
  [[ $failed == 0 ]]
  evidence "characters=2 source=platform_intents path=control_socket retry=idempotent"
}

seam_stages() { # §4.6 every #823 stage against the node's own port
  # The bootstrap evidence ages past five seconds, so each admission fetches its own.
  sleep 6
  NODE_BOOT_ADDRESS="127.0.0.1:${NODE_BOOT_GAME_PORT:-17181}" \
  NODE_BOOT_GAMEPLAY_CERT="$WP5_PKI/gameplay.crt" \
  NODE_BOOT_DATABASE_URL="$ADMIN_URL" \
  NODE_BOOT_WORLD_ID="$WORLD_ID" NODE_BOOT_CHANNEL_ID="$CHANNEL_ID" \
  WP5_S3A_PORT="$WP5_PORT" WP5_S3A_CA_CERT="$WP5_PKI/server-ca.crt" \
  WP5_S3A_CLIENT_CERT="$WP5_PKI/client.crt" WP5_S3A_CLIENT_KEY="$WP5_PKI/client.key" \
  WP5_S3A_ACCOUNT_ID="$ACCOUNT_ID" WP5_S3B_SECOND_ACCOUNT_ID="$SECOND_ACCOUNT_ID" \
  WP5_S3B_FRESH_KEY_ID="$FRESH_KEY_ID" WP5_S3B_FRESH_KEY_SEED="$FRESH_SEED_HEX" \
    cargo +1.94.0 test --locked -p oteryn-game-server --lib \
    gameplay_transport::qualification::node_boot_seam_against_running_node -- --ignored --exact --nocapture
}

graceful_shutdown() { # ready=false first, socket removed
  sudo kill -TERM "$NODE_PID"
  local code=0
  wait "$NODE_PID" 2>/dev/null || code=$?
  # A process started by another stage's subshell is not our child; wait for it.
  for _ in $(seq 1 30); do sudo kill -0 "$NODE_PID" 2>/dev/null || break; sleep 1; done
  grep -q "readiness ready=false" "$node_log" || fail "no ready=false on shutdown"
  grep -q "shutdown state=complete" "$node_log" || fail "no clean shutdown"
  [[ ! -e "$BASE/run/control.sock" ]] || fail "socket left after graceful shutdown"
  [[ "$(echo 'SELECT ready FROM game_durability_admission_runtime_guards' | psql_admin oteryn_node_boot)" == f ]] || fail "guard still ready"
  evidence "shutdown ready=false socket=removed"
}

restart_without_supersession() { # fails at S2 custody, never ready
  ops authorization issue --file launch-b.json --binding node-boot-b
  write_node_config launch-b.json
  local code=0
  sudo -u "$SERVICE_USER" timeout 120 "$BASE/bin/oteryn-game-server" serve --config "$BASE/node/node.toml" 2> "$WORK/restart.log" || code=$?
  cat "$WORK/restart.log"
  [[ $code == 13 ]] || fail "restart without supersession exit=$code"
  local line node revision
  line="$(grep 'event=registered' "$WORK/restart.log")"
  node="$(sed -n 's/.*node_id=\([^ ]*\).*/\1/p' <<<"$line")"
  revision="$(sed -n 's/.*registration_revision=\([0-9]*\).*/\1/p' <<<"$line")"
  ops registration revoke --node-id "$node" --revision "$revision"
  evidence "restart_without_supersession=refused_at_custody exit=13"
}

superseding_replacement() { # claims custody, ready by CAS after replace
  ops authorization issue --file launch-c.json --binding node-boot-c --supersedes "$node_a"
  write_node_config launch-c.json
  : > "$node_log"
  start_node
  await_log awaiting_assignment 120
  local node revision
  node="$(node_field "$node_log" node_id)"; revision="$(node_field "$node_log" registration_revision)"
  ops assignment replace --request replace-c.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --node-id "$node" --revision "$revision"
  await_log "readiness ready=true" 60
  sudo kill -TERM "$NODE_PID"
  for _ in $(seq 1 30); do sudo kill -0 "$NODE_PID" 2>/dev/null || break; sleep 1; done
  grep -q "shutdown state=complete" "$node_log" || fail "replacement did not shut down cleanly"
  evidence "replacement=superseding custody=claimed readiness=cas"
}

stage operator_setup -- operator_setup
stage configuration_negatives operator_setup -- configuration_negatives
stage node_assigned_ready operator_setup -- node_assigned_ready
stage control_socket_peer node_assigned_ready -- control_socket_peer
stage character_bootstrap node_assigned_ready -- character_bootstrap
stage seam_stages character_bootstrap -- seam_stages
stage graceful_shutdown node_assigned_ready -- graceful_shutdown
stage restart_without_supersession graceful_shutdown -- restart_without_supersession
stage superseding_replacement graceful_shutdown -- superseding_replacement
NODE_PID=""

echo "NODE_BOOT_STAGES"
failures=0
for name in "${ORDER[@]}"; do
  printf 'NODE_BOOT_STAGE %-30s %s\n' "$name" "${STATUS[$name]}"
  [[ ${STATUS[$name]} == PASS ]] || failures=$((failures + 1))
done
unset FRESH_SEED_HEX
if [[ $failures == 0 ]]; then
  result=NODE_BOOT_PASS
else
  exit 1
fi
