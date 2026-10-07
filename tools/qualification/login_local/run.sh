#!/usr/bin/env bash
# RUNBOOK-1 (CP D825, packet §2.7): one local native login, Platform -> Gateway -> game node.
#
# Extends tools/qualification/node_boot: the real Platform producer in preproduction with the
# native admission issuer (mode 33a ownership, mode 34a route record), the Go gateway, one game
# node reporting runtime status, `ops assignment assign` reporting to Platform, one test account
# with one Character, and the client's native-login environment. Secrets are generated per run
# and live only in a 0700 work directory removed on exit (unless LOGIN_LOCAL_KEEP=1).
#
# Result lines: LOGIN_LOCAL_RESULT=BLOCKED|FAIL|READY|ADMITTED|WALKED.
#   READY    every service is up and the client environment is written.
#   ADMITTED the client binary printed "admitted to World ..." (non-Windows: stops there).
#   WALKED   only with operator attestation of the server step result seen in the Windows client:
#            while LOGIN_LOCAL_HOLD=1 the operator writes text containing this run's id to the
#            attestation file after the step (the walk has no machine-readable signal; see README).
# A requested client run (LOGIN_LOCAL_RUN_CLIENT=1) that does not admit ends FAIL with exit 1.
# PHP snippets are deliberately single-quoted for the container shell; the identities are JSON text.
# shellcheck disable=SC2016,SC2089,SC2090
set -Eeuo pipefail
umask 077

# Platform main carrying U1 (>= 71bbe6c) and the native-only gateway config (Oteryn-Platform #1472).
readonly PLATFORM_SHA=b18d32d30c4c4e496330077d12b37db3f0f29011
readonly TOPOLOGY_REVISION=login-local-v1
readonly ACCOUNT_ID=01934f10-7c00-7000-8000-0000000000a1
readonly ACCOUNT_EMAIL=login-local@example.invalid
readonly ADMISSION_KEY_ID=login-local-admission-key-1
readonly INTENT_OPERATION=01934f10-7c04-7001-805b-3b1122334499
readonly INTENT_NAME="Local Walker"
readonly INTERPRETATION=(ll-profile-1 ll-ruleset-1 ll-content-1 ll-starter-1)
readonly CHANNEL_KEY=main
readonly COMPOSE_FILE=tools/qualification/wp5_s3a/compose.yml
readonly COMPOSE_S3B=tools/qualification/wp5_s3b/compose.override.yml
readonly COMPOSE_NODE_BOOT=tools/qualification/node_boot/compose.override.yml
readonly COMPOSE_LOCAL=tools/qualification/login_local/compose.override.yml
readonly PG_IMAGE=postgres:17.6-bookworm@sha256:f3bd19c606e442c3d7bdfa8002e03fe260a1023351e0ea4598032022b68dd6e3
readonly SERVICE_USER=oteryn-login-local
readonly BASE=/srv/oteryn-login-local
readonly RUNTIME_IDENTITY=oteryn-game-node-runtime-status
readonly OPS_IDENTITY=oteryn-game-ops

blocked() { echo "LOGIN_LOCAL_RESULT=BLOCKED reason=$1"; exit 2; }
command -v docker >/dev/null 2>&1 || blocked docker_missing
docker info >/dev/null 2>&1 || blocked docker_daemon_unreachable
for tool in openssl cargo sudo git; do command -v "$tool" >/dev/null 2>&1 || blocked "${tool}_missing"; done

GAME_SOURCE="$(git rev-parse --show-toplevel)"
PLATFORM_SOURCE="${PLATFORM_SOURCE:-$GAME_SOURCE/_platform}"
[[ "$(git -C "$PLATFORM_SOURCE" rev-parse HEAD 2>/dev/null)" == "$PLATFORM_SHA" ]] || blocked exact_platform_checkout_missing

WORK="$(mktemp -d "${RUNNER_TEMP:-/tmp}/login-local.XXXXXX")"
WP5_PKI="$WORK/pki"
WP5_SCRATCH="$WORK/scratch"
mkdir -p "$WP5_PKI" "$WP5_SCRATCH"
# F7: Platform's isolatedConnection() refuses MySQL outside APP_ENV=testing, so preproduction topology
# issuance runs against a retained per-run SQLite fixture. The Platform guard requires the file to be a
# regular file at <tmp>/oteryn-native-topology-<hex>/oteryn-native-topology.sqlite inside the container;
# the host side lives in the 0700 work directory and is bind-mounted there (www-data uid 33 needs write
# access to the directory; the 0700 parent keeps other host users out).
LL_TOPOLOGY_HEX="$(openssl rand -hex 8)"
LL_TOPOLOGY_DIR="$WORK/topology-db"
mkdir -p "$LL_TOPOLOGY_DIR"
: > "$LL_TOPOLOGY_DIR/oteryn-native-topology.sqlite"
chmod 0777 "$LL_TOPOLOGY_DIR"
chmod 0666 "$LL_TOPOLOGY_DIR/oteryn-native-topology.sqlite"
LL_TOPOLOGY_SQLITE="/tmp/oteryn-native-topology-$LL_TOPOLOGY_HEX/oteryn-native-topology.sqlite"
WP5_PORT="${LOGIN_LOCAL_PLATFORM_MTLS_PORT:-18563}"
LL_PLATFORM_HTTP_PORT="${LOGIN_LOCAL_PLATFORM_HTTP_PORT:-18564}"
LL_GATEWAY_PORT="${LOGIN_LOCAL_GATEWAY_PORT:-18565}"
GAME_PORT="${LOGIN_LOCAL_GAME_PORT:-17281}"
PG_PORT="${LOGIN_LOCAL_PG_PORT:-15533}"
WP5_PROJECT="loginlocal${GITHUB_RUN_ID:-local}${GITHUB_RUN_ATTEMPT:-1}"
WP5_DB_PASSWORD="$(openssl rand -hex 24)"
WP5_DB_ROOT_PASSWORD="$(openssl rand -hex 24)"
WP5_APP_KEY="base64:$(openssl rand -base64 32 | tr -d '\n')"
WP5_TOPOLOGY_REVISION="$TOPOLOGY_REVISION"
WP5_FSYNC_FAULT=none
PG_CONTAINER="${WP5_PROJECT}-postgres"
PG_ADMIN_PASSWORD="$(openssl rand -hex 24)"
CONTROL_PASSWORD="$(openssl rand -hex 24)"
RUNTIME_PASSWORD="$(openssl rand -hex 24)"
ACCOUNT_PASSWORD="$(openssl rand -hex 16)"
LL_SERVICE_TOKEN="$(openssl rand -hex 32)"
LL_SERVICE_TOKEN_SHA256="$(printf '%s' "$LL_SERVICE_TOKEN" | openssl dgst -sha256 -r | cut -d ' ' -f 1)"
LL_ADMISSION_KEY_ID="$ADMISSION_KEY_ID"
# Placeholders until the Registry issues the topology; the Platform is recreated with the real scope.
LL_WORLD_ID=00000000-0000-7000-8000-000000000000
LL_RUNTIME_STATUS_IDENTITIES='{}'
LL_SCOPE_ASSIGNMENT_IDENTITIES='{}'
export GAME_SOURCE PLATFORM_SOURCE WP5_PKI WP5_SCRATCH WP5_PORT WP5_PROJECT WP5_DB_PASSWORD WP5_DB_ROOT_PASSWORD
export LL_TOPOLOGY_HEX LL_TOPOLOGY_DIR
export WP5_APP_KEY WP5_TOPOLOGY_REVISION WP5_FSYNC_FAULT LL_PLATFORM_HTTP_PORT LL_GATEWAY_PORT
export LL_SERVICE_TOKEN LL_SERVICE_TOKEN_SHA256 LL_ADMISSION_KEY_ID LL_WORLD_ID LL_RUNTIME_STATUS_IDENTITIES LL_SCOPE_ASSIGNMENT_IDENTITIES
NODE_PID=""
result=FAIL

compose() {
  docker compose --project-name "$WP5_PROJECT" --file "$GAME_SOURCE/$COMPOSE_FILE" --file "$GAME_SOURCE/$COMPOSE_S3B" \
    --file "$GAME_SOURCE/$COMPOSE_NODE_BOOT" --file "$GAME_SOURCE/$COMPOSE_LOCAL" "$@"
}
evidence() { printf 'LOGIN_LOCAL_EVIDENCE %s\n' "$*"; }
cleanup() {
  local rc=$?
  if [[ "${LOGIN_LOCAL_KEEP:-0}" != 1 ]]; then
    [[ -z "$NODE_PID" ]] || sudo kill -TERM "$NODE_PID" 2>/dev/null || true
    sudo pkill -TERM -u "$SERVICE_USER" 2>/dev/null || true
    compose down --volumes --remove-orphans --timeout 15 >/dev/null 2>&1 || true
    docker rm --force --volumes "$PG_CONTAINER" >/dev/null 2>&1 || true
    sudo rm -rf "$BASE" || true
    rm -rf "$WORK"
    evidence "cleanup=complete"
  else
    evidence "cleanup=skipped work=$WORK (contains per-run secrets; remove it yourself)"
  fi
  echo "LOGIN_LOCAL_RESULT=$result"
  exit "$rc"
}
trap cleanup EXIT INT TERM

make_ca() { openssl req -x509 -newkey rsa:2048 -sha256 -nodes -days 1 -subj "/CN=$2" -keyout "$WP5_PKI/$1.key" -out "$WP5_PKI/$1.crt" >/dev/null 2>&1; }
make_leaf() {
  local prefix=$1 subject=$2 ca=$3 usage=$4 san=${5:-}
  openssl req -new -newkey rsa:2048 -nodes -sha256 -subj "/CN=$subject" -keyout "$WP5_PKI/$prefix.key" -out "$WP5_PKI/$prefix.csr" >/dev/null 2>&1
  printf 'basicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=%s\n' "$usage" > "$WP5_PKI/$prefix.ext"
  [[ -z "$san" ]] || printf 'subjectAltName=%s\n' "$san" >> "$WP5_PKI/$prefix.ext"
  openssl x509 -req -sha256 -days 1 -in "$WP5_PKI/$prefix.csr" -CA "$WP5_PKI/$ca.crt" -CAkey "$WP5_PKI/$ca.key" \
    -CAcreateserial -extfile "$WP5_PKI/$prefix.ext" -out "$WP5_PKI/$prefix.crt" >/dev/null 2>&1
}
make_ca server-ca login-local-server-ca
make_ca client-ca login-local-client-ca
make_ca db-ca login-local-db-ca
make_leaf server source.test server-ca serverAuth DNS:source.test
make_leaf db db.login-local.test db-ca serverAuth DNS:db.login-local.test
# Four distinct mTLS identities with distinct keys: evidence (also the Character intent),
# node-host runtime status, ownership authority (ops) -- never shared (runtime-status contract §3).
make_leaf client oteryn-game-native-evidence client-ca clientAuth
cp "$WP5_PKI/client.crt" "$WP5_PKI/intent-client.crt"
cp "$WP5_PKI/client.key" "$WP5_PKI/intent-client.key"
make_leaf runtime-status "$RUNTIME_IDENTITY" client-ca clientAuth
make_leaf ops-authority "$OPS_IDENTITY" client-ca clientAuth
# Gateway -> Platform upstream: nginx serves TLS (SAN nginx) from a test CA the gateway trusts via SSL_CERT_FILE.
make_ca platform-upstream-ca login-local-platform-upstream-ca
make_leaf platform-upstream nginx platform-upstream-ca serverAuth DNS:nginx
chmod 644 "$WP5_PKI/platform-upstream-ca.crt"
LL_NO_SYSTEM_ROOTS="$WORK/no-system-roots"
mkdir -m 0755 "$LL_NO_SYSTEM_ROOTS"
export LL_NO_SYSTEM_ROOTS
# Gameplay listener leaf: the dev root the client trusts is this self-signed end-entity certificate.
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes -days 1 -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost" -addext "basicConstraints=critical,CA:FALSE" \
  -addext "keyUsage=critical,digitalSignature" -addext "extendedKeyUsage=serverAuth" \
  -keyout "$WP5_PKI/gameplay.key" -out "$WP5_PKI/gameplay.crt" >/dev/null 2>&1
# Native admission issuer key: base64url Ed25519 seed file (Platform side) + public key published to Registry trust.
openssl genpkey -algorithm ed25519 -out "$WP5_PKI/admission-signing.pem" >/dev/null 2>&1
raw_hex() { tail -c 32 | od -An -v -tx1 | tr -d ' \n'; }
ADMISSION_PUBLIC_HEX="$(openssl pkey -in "$WP5_PKI/admission-signing.pem" -pubout -outform DER | raw_hex)"
ADMISSION_SEED_HEX="$(openssl pkey -in "$WP5_PKI/admission-signing.pem" -outform DER | raw_hex)"
[[ ${#ADMISSION_PUBLIC_HEX} == 64 && ${#ADMISSION_SEED_HEX} == 64 ]]
printf '%b' "$(printf '%s' "$ADMISSION_SEED_HEX" | sed 's/\(..\)/\\x\1/g')" | openssl base64 -A | tr '+/' '-_' | tr -d '=' > "$WP5_SCRATCH/signing.seed"
unset ADMISSION_SEED_HEX
evidence "pins platform=$PLATFORM_SHA topology=$TOPOLOGY_REVISION game=$(git rev-parse HEAD) app_env=preproduction"

# TLS-verified PostgreSQL 17.6 for the Game durability root.
chmod 644 "$WP5_PKI/db.crt" "$WP5_PKI/db.key"
docker run --detach --name "$PG_CONTAINER" --publish "127.0.0.1:$PG_PORT:5432" \
  --env POSTGRES_USER=oteryn_login_local_admin --env POSTGRES_PASSWORD="$PG_ADMIN_PASSWORD" --env POSTGRES_DB=postgres \
  --volume "$WP5_PKI/db.crt:/tls/db.crt:ro" --volume "$WP5_PKI/db.key:/tls/db.key:ro" --entrypoint bash "$PG_IMAGE" -c \
  'install -o postgres -m 0600 /tls/db.key /tmp/db.key && install -o postgres -m 0644 /tls/db.crt /tmp/db.crt && exec docker-entrypoint.sh postgres -c ssl=on -c ssl_cert_file=/tmp/db.crt -c ssl_key_file=/tmp/db.key' >/dev/null
chmod 600 "$WP5_PKI/db.key"
for _ in $(seq 1 60); do
  docker exec "$PG_CONTAINER" pg_isready -U oteryn_login_local_admin -d postgres >/dev/null 2>&1 && break
  sleep 1
done
psql_admin() { docker exec -i "$PG_CONTAINER" psql -v ON_ERROR_STOP=1 -qAt -U oteryn_login_local_admin -d "$1"; }
echo "CREATE DATABASE oteryn_login_local" | psql_admin postgres
[[ "$(echo 'SHOW server_version_num' | psql_admin oteryn_login_local)" == 170006 ]] || blocked postgres_version
ADMIN_URL="postgresql://oteryn_login_local_admin:$PG_ADMIN_PASSWORD@127.0.0.1:$PG_PORT/oteryn_login_local"

# Platform (preproduction) + nginx; the first start has no scope identities yet.
compose config --quiet
compose build --pull platform gateway
compose up --detach --wait db platform nginx
php_exec() { compose exec --no-TTY --user www-data platform php -r 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); '"$1"; }

# Registry topology: one world row, one issued WorldId/ChannelId (uuid7, read back, never invented).
# F7: issue on the retained SQLite fixture (migrated, one world row), then mirror the issued ids and
# route columns into the Platform database, whose Registry reads the same rows.
sqlite_exec() {
  compose exec --no-TTY --user www-data -e DB_CONNECTION=sqlite -e DB_DATABASE="$LL_TOPOLOGY_SQLITE" platform php -r \
    'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); '"$1"
}
ENSURE_WORLD='Illuminate\Support\Facades\Artisan::call("game-auth:world:ensure",["--id"=>"1","--slug"=>"login-local","--name"=>"Login Local","--region"=>"local","--host"=>"127.0.0.1","--port"=>"'"$GAME_PORT"'"]);'
php_exec "$ENSURE_WORLD"
sqlite_exec 'Illuminate\Support\Facades\Artisan::call("migrate",["--force"=>true,"--no-interaction"=>true]); '"$ENSURE_WORLD"' $r=app(App\GameAuth\Worlds\NativeTopologyRegistry::class)->issueForPreproduction(1,"'"$CHANNEL_KEY"'"); echo $r->worldId," ",$r->channelId,"\n";' | tail -n 1 > "$WORK/topology"
read -r WORLD_ID CHANNEL_ID < "$WORK/topology"
[[ "$WORLD_ID" =~ ^[0-9a-f-]{36}$ && "$CHANNEL_ID" =~ ^[0-9a-f-]{36}$ ]] || { echo "registry topology issuance failed"; exit 1; }
# Mode 34a: route record (tls_server_name must be in the gameplay certificate SAN) with native login enabled.
ROUTE_REVISION="$(sqlite_exec 'echo app(App\GameAuth\Worlds\NativeTopologyRegistry::class)->publishRouteForPreproduction(1,"'"$CHANNEL_KEY"'","127.0.0.1",'"$GAME_PORT"',"localhost",true)->routeRevision;' | tail -n 1)"
TOPOLOGY_MIRROR="$(sqlite_exec 'echo base64_encode(json_encode(["world_id"=>Illuminate\Support\Facades\DB::table("game_worlds")->where("id",1)->value("world_id"),"channel"=>(array) Illuminate\Support\Facades\DB::table("game_channels")->where("game_world_id",1)->where("channel_key","'"$CHANNEL_KEY"'")->first()]));' | tail -n 1)"
php_exec '$d=json_decode(base64_decode("'"$TOPOLOGY_MIRROR"'"),true); $c=$d["channel"]; unset($c["id"]); Illuminate\Support\Facades\DB::table("game_worlds")->where("id",1)->update(["world_id"=>$d["world_id"]]); Illuminate\Support\Facades\DB::table("game_channels")->updateOrInsert(["game_world_id"=>1,"channel_key"=>"'"$CHANNEL_KEY"'"],$c);'
unset TOPOLOGY_MIRROR
[[ "$ROUTE_REVISION" =~ ^rt\.[0-9]+\.[0-9a-f]+$ ]] || { echo "route publication failed"; exit 1; }
evidence "registry world_id=$WORLD_ID channel_id=$CHANNEL_ID route_revision=$ROUTE_REVISION native_login_enabled=true"

# Real scope identities and mode 33a; recreate Platform with them, then install the issuer seed (0600, www-data).
LL_WORLD_ID="$WORLD_ID"
LL_RUNTIME_STATUS_IDENTITIES="{\"CN=$RUNTIME_IDENTITY\":[\"$WORLD_ID/$CHANNEL_ID\"]}"
LL_SCOPE_ASSIGNMENT_IDENTITIES="{\"CN=$OPS_IDENTITY\":[\"$WORLD_ID/$CHANNEL_ID\"]}"
export LL_WORLD_ID LL_RUNTIME_STATUS_IDENTITIES LL_SCOPE_ASSIGNMENT_IDENTITIES
compose up --detach --wait --force-recreate platform
# The gateway trusts exactly the per-run CA: one certificate in SSL_CERT_FILE, an empty SSL_CERT_DIR.
[[ "$(grep -c 'BEGIN CERTIFICATE' "$WP5_PKI/platform-upstream-ca.crt")" == 1 && -z "$(ls -A "$LL_NO_SYSTEM_ROOTS")" ]] \
  || { echo "gateway trust roots are not exactly the per-run CA"; exit 1; }
compose config gateway | grep -q 'SSL_CERT_DIR: /run/login-local/no-system-roots' \
  || { echo "gateway SSL_CERT_DIR is not the empty roots directory"; exit 1; }
compose up --detach --wait nginx gateway
compose exec --no-TTY --user root platform install -o www-data -g www-data -m 0600 /run/wp5/signing.seed /run/oteryn-admission/signing.seed
php_exec 'app(App\GameAuth\NativeEvidence\NativeSigningTrustRegistry::class)->publishTrustedKey(App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_ISSUER,App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_PROFILE,"fresh_admission","'"$ADMISSION_KEY_ID"'",hex2bin("'"$ADMISSION_PUBLIC_HEX"'"));'
php_exec 'Illuminate\Support\Facades\DB::table("identities")->insert(["email"=>"'"$ACCOUNT_EMAIL"'","password"=>password_hash("'"$ACCOUNT_PASSWORD"'",PASSWORD_BCRYPT),"account_id"=>"'"$ACCOUNT_ID"'","native_security_generation"=>1,"created_at"=>now(),"updated_at"=>now()]);'
OAUTH_CLIENT_ID="$(php_exec 'Illuminate\Support\Facades\Artisan::call("game-auth:oauth-client:ensure"); echo Illuminate\Support\Facades\Artisan::output();' | sed -n 's/.*client id: \([^ ]*\).*/\1/p')"
[[ -n "$OAUTH_CLIENT_ID" ]] || { echo "oauth client id not found"; exit 1; }
evidence "platform_seed=account=1 admission_trust=published_public_key_only oauth_client=ensured mode=33a"

# Shipped binaries, service user, roles, migration (as node_boot).
cargo +1.94.0 build --locked -p oteryn-game-server -p oteryn-client --bins
TARGET="$GAME_SOURCE/target/debug"
sudo useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER" 2>/dev/null || true
SERVICE_UID="$(id -u "$SERVICE_USER")"
sudo install -d -o root -g root -m 0755 "$BASE" "$BASE/bin" "$BASE/state" "$BASE/node"
sudo install -d -o root -g root -m 0700 "$BASE/ops"
sudo install -d -o "$SERVICE_UID" -m 0700 "$BASE/fence-parent/fence" "$BASE/run"
sudo install -d -o "$SERVICE_UID" -m 0755 "$BASE/node/secrets"
sudo install -o root -m 0755 "$TARGET/oteryn-game-server" "$TARGET/oteryn-game-ops" "$TARGET/oteryn-game-migrate" "$BASE/bin/"
OTERYN_GAME_MIGRATION_DATABASE_URL="$ADMIN_URL" "$BASE/bin/oteryn-game-migrate"
psql_admin oteryn_login_local <<SQL
CREATE ROLE ll_control LOGIN PASSWORD '$CONTROL_PASSWORD' IN ROLE oteryn_game_control;
CREATE ROLE ll_runtime LOGIN PASSWORD '$RUNTIME_PASSWORD' IN ROLE oteryn_game_runtime;
GRANT CONNECT ON DATABASE oteryn_login_local TO ll_control, ll_runtime;
INSERT INTO game_control_scope_grants SELECT 'll_control', '$WORLD_ID', '$CHANNEL_ID', op FROM generate_series(1, 4) op;
SQL
printf '%s\n' "$CONTROL_PASSWORD" > "$WORK/control-password"
printf '%s\n' "$RUNTIME_PASSWORD" > "$WORK/runtime-password"
secret_to() { sudo install -o "$1" -m 0600 "$2" "$3"; }
secret_to root "$WORK/control-password" "$BASE/ops/pg-password"
secret_to root "$WP5_PKI/db-ca.crt" "$BASE/ops/db-ca.pem"
secret_to root "$WP5_PKI/server-ca.crt" "$BASE/ops/platform-roots.pem"
secret_to root "$WP5_PKI/ops-authority.crt" "$BASE/ops/authority.crt"
secret_to root "$WP5_PKI/ops-authority.key" "$BASE/ops/authority.key"
secret_to root "$WP5_PKI/runtime-status.crt" "$BASE/ops/node-runtime-status.crt"
secret_to "$SERVICE_UID" "$WORK/runtime-password" "$BASE/node/secrets/pg-password"
secret_to "$SERVICE_UID" "$WP5_PKI/db-ca.crt" "$BASE/node/secrets/db-ca.pem"
secret_to "$SERVICE_UID" "$WP5_PKI/server-ca.crt" "$BASE/node/secrets/platform-roots.pem"
secret_to "$SERVICE_UID" "$WP5_PKI/client.crt" "$BASE/node/secrets/platform-client.crt"
secret_to "$SERVICE_UID" "$WP5_PKI/client.key" "$BASE/node/secrets/platform-client.key"
secret_to "$SERVICE_UID" "$WP5_PKI/runtime-status.crt" "$BASE/node/secrets/runtime-status.crt"
secret_to "$SERVICE_UID" "$WP5_PKI/runtime-status.key" "$BASE/node/secrets/runtime-status.key"
secret_to "$SERVICE_UID" "$WP5_PKI/gameplay.crt" "$BASE/node/secrets/gameplay.crt"
secret_to "$SERVICE_UID" "$WP5_PKI/gameplay.key" "$BASE/node/secrets/gameplay.key"

cat > "$WORK/ops.toml" <<TOML
[operator]
state_directory = "$BASE/state"
service_uid = $SERVICE_UID
[database]
transport_ip = "127.0.0.1"
port = $PG_PORT
tls_server_name = "db.login-local.test"
database = "oteryn_login_local"
username = "ll_control"
password_file = "$BASE/ops/pg-password"
root_ca_file = "$BASE/ops/db-ca.pem"
[character]
fence_directory = "$BASE/fence-parent/fence"
authority_scope_id = "character-primary"
issuer_identity = "game-ops"
TOML
sudo install -o root -m 0600 "$WORK/ops.toml" "$BASE/ops/ops.toml"
# assignment_epoch is declared (positive) and never raised by ops; node and report config must agree.
cat > "$WORK/report.toml" <<TOML
endpoint = "127.0.0.1:$WP5_PORT"
peer_name = "source.test"
trust_roots_file = "$BASE/ops/platform-roots.pem"
client_certificate_file = "$BASE/ops/authority.crt"
client_key_file = "$BASE/ops/authority.key"
assignment_epoch = 1
[node_certificate_files]
"CN=$RUNTIME_IDENTITY" = "$BASE/ops/node-runtime-status.crt"
[[scope]]
world_id = "$WORLD_ID"
channel_id = "$CHANNEL_ID"
node_identities = ["CN=$RUNTIME_IDENTITY"]
TOML
sudo install -o root -m 0600 "$WORK/report.toml" "$BASE/ops/report.toml"
DESCRIPTOR_INSTALLED_AT="$(date +%s)"
write_node_config() {
  cat > "$WORK/node.toml" <<TOML
[listener]
address = "127.0.0.1:$GAME_PORT"
entry_deadline_ms = 20000
max_connections = 256
max_handshake_units = 64
certificate_chain_file = "$BASE/node/secrets/gameplay.crt"
private_key_file = "$BASE/node/secrets/gameplay.key"
[scope]
world_id = "$WORLD_ID"
channel_id = "$CHANNEL_ID"
assignment_wait_ms = 120000
preproduction_actor_capacity = 131072
[database]
transport_ip = "127.0.0.1"
port = $PG_PORT
tls_server_name = "db.login-local.test"
database = "oteryn_login_local"
username = "ll_runtime"
password_file = "$BASE/node/secrets/pg-password"
root_ca_file = "$BASE/node/secrets/db-ca.pem"
[character]
fence_directory = "$BASE/fence-parent/fence"
authority_scope_id = "character-primary"
issuer_identity = "game-ops"
[control]
socket_path = "$BASE/run/control.sock"
[readiness]
source_authority = "oteryn:runtime:login-local"
route_revision = "$ROUTE_REVISION"
runtime_observation_revision = "runtime-1"
ruleset_revision = "rules-ll-1"
content_revision = "content-ll-1"
map_revision = "map-ll-1"
world_policy_revision = "policy-ll-1"
offer_revision = "offer-ll-1"
[platform]
source_authority = "platform"
endpoint = "127.0.0.1:$WP5_PORT"
peer_name = "source.test"
trust_roots_file = "$BASE/node/secrets/platform-roots.pem"
client_certificate_file = "$BASE/node/secrets/platform-client.crt"
client_key_file = "$BASE/node/secrets/platform-client.key"
descriptor_revision = 1
installed_at = $DESCRIPTOR_INSTALLED_AT
[platform.runtime_status]
client_certificate_file = "$BASE/node/secrets/runtime-status.crt"
client_key_file = "$BASE/node/secrets/runtime-status.key"
assignment_epoch = 1
[launch]
authorization_file = "$BASE/state/launch-a.json"
s2_authorization_file = "$BASE/state/s2-fresh-store.json"
TOML
  sudo install -o root -m 0644 "$WORK/node.toml" "$BASE/node/node.toml"
}
ops() { sudo "$BASE/bin/oteryn-game-ops" --config "$BASE/ops/ops.toml" "$@"; }
node_log="$WORK/node.log"
await_log() { for _ in $(seq 1 "$2"); do grep -q "$1" "$node_log" && return 0; sleep 1; done; echo "missing node event: $1"; tail -n 40 "$node_log"; exit 1; }

ops authorization issue --file launch-a.json --binding login-local-a
write_node_config
ops s2 issue --node-config "$BASE/node/node.toml" --file s2-fresh-store.json --namespace login-local --authorization disposable-qualification
ops character fresh-store --request fresh-store.json
ops character interpretation --profile "${INTERPRETATION[0]}" --ruleset "${INTERPRETATION[1]}" --content "${INTERPRETATION[2]}" --starter "${INTERPRETATION[3]}"

sudo -u "$SERVICE_USER" "$BASE/bin/oteryn-game-server" serve --config "$BASE/node/node.toml" < /dev/null > /dev/null 2> "$node_log" &
NODE_PID=$!
await_log awaiting_assignment 120
node="$(sed -n 's/.*awaiting_assignment.*node_id=\([^ ]*\).*/\1/p' "$node_log" | tail -n 1)"
revision="$(sed -n 's/.*awaiting_assignment.*registration_revision=\([^ ]*\).*/\1/p' "$node_log" | tail -n 1)"
ops content activate --request content-1.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --sequence 1 --previous empty
# `assignment assign` reports ReportScopeAssignmentV1 to Platform when both configs are given.
ops assignment assign --request assign-a.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --node-id "$node" --revision "$revision" \
  --node-config "$BASE/node/node.toml" --report-config "$BASE/ops/report.toml" | tee "$WORK/assign.out"
grep -q 'report=ReportScopeAssignmentV1' "$WORK/assign.out" || { echo "assignment was not reported to Platform"; exit 1; }
await_log "readiness ready=true" 60
evidence "node assigned=operator report=ReportScopeAssignmentV1 assignment_epoch=1 readiness=true runtime_status=configured"

# Character from a real Platform intent (the account owns it; mode 33a does not verify ownership).
php_exec '$id=Illuminate\Support\Facades\DB::table("identities")->where("account_id","'"$ACCOUNT_ID"'")->value("id"); $code=Illuminate\Support\Facades\Artisan::call("game-auth:character-bootstrap-intent:issue",["--identity-id"=>(string)$id,"--operation-id"=>"'"$INTENT_OPERATION"'","--target-world-id"=>"'"$WORLD_ID"'","--requested-name"=>"'"$INTENT_NAME"'","--profile-revision"=>"'"${INTERPRETATION[0]}"'","--ruleset-revision"=>"'"${INTERPRETATION[1]}"'","--content-revision"=>"'"${INTERPRETATION[2]}"'","--starter-template-revision"=>"'"${INTERPRETATION[3]}"'"]); fwrite(STDERR, Illuminate\Support\Facades\Artisan::output()); exit($code);'
ops character bootstrap --socket "$BASE/run/control.sock" --operation-id "$INTENT_OPERATION"
CHARACTER_ID="$(echo 'SELECT character_id FROM game_character_roots LIMIT 1' | psql_admin oteryn_login_local)"
[[ "$CHARACTER_ID" =~ ^[0-9a-f-]{36}$ ]] || { echo "character not created"; exit 1; }
evidence "character=1 source=platform_intent character_id=$CHARACTER_ID"

# Client environment (public values plus the per-run test password, in a 0600 file, never in the log).
CLIENT_ENV="${LOGIN_LOCAL_CLIENT_ENV:-$WORK/client.env}"
ATTEST_FILE="${LOGIN_LOCAL_WALKED_ATTEST_FILE:-$WORK/walked.attest}"
RUN_ID="$(openssl rand -hex 8)"
# A stale attestation from an earlier run must never count: clear it before READY.
rm -f "$ATTEST_FILE"
# A Windows client cannot resolve a POSIX path: emit the Windows-readable path of the dev root.
DEV_ROOT="$WP5_PKI/gameplay.crt"
if [[ "${LOGIN_LOCAL_RUN_CLIENT:-1}" != 1 ]]; then
  chmod 644 "$DEV_ROOT"
  if [[ -n "${LOGIN_LOCAL_DEV_ROOT_WINDOWS:-}" ]]; then
    cp "$DEV_ROOT" "$LOGIN_LOCAL_DEV_ROOT_WINDOWS" 2>/dev/null || blocked dev_root_windows_path_unwritable
    DEV_ROOT="${LOGIN_LOCAL_DEV_ROOT_WINDOWS_AS_SEEN:-$LOGIN_LOCAL_DEV_ROOT_WINDOWS}"
  elif command -v wslpath >/dev/null 2>&1; then
    DEV_ROOT="$(wslpath -w "$DEV_ROOT")"
  else
    blocked dev_root_windows_path_missing
  fi
fi
{
  echo "OTERYN_PLATFORM_URL=http://127.0.0.1:$LL_PLATFORM_HTTP_PORT"
  echo "OTERYN_GATEWAY_URL=http://127.0.0.1:$LL_GATEWAY_PORT"
  echo "OTERYN_OAUTH_CLIENT_ID=$OAUTH_CLIENT_ID"
  echo "OTERYN_WORLD=$WORLD_ID"
  echo "OTERYN_CHARACTER_ID=$CHARACTER_ID"
  echo "OTERYN_DEV_ROOT=$DEV_ROOT"
  echo "# browser sign-in at /login: $ACCOUNT_EMAIL / $ACCOUNT_PASSWORD"
} > "$CLIENT_ENV"
result=READY
evidence "ready client_env=$CLIENT_ENV platform=http://127.0.0.1:$LL_PLATFORM_HTTP_PORT gateway=http://127.0.0.1:$LL_GATEWAY_PORT"

if [[ "${LOGIN_LOCAL_RUN_CLIENT:-1}" == 1 ]]; then
  # The client opens the system browser for the OAuth sign-in; this needs an operator and a display.
  grep -v '^#' "$CLIENT_ENV" > "$WORK/client.vars"
  set -a
  # shellcheck disable=SC1091
  source "$WORK/client.vars"
  set +a
  client_rc=0
  timeout "${LOGIN_LOCAL_CLIENT_TIMEOUT:-300}" "$TARGET/oteryn-client" | tee "$WORK/client.out" || client_rc=$?
  # A requested client run that does not admit is a failed qualification, never READY.
  if ! grep -q '^Oteryn: admitted to World ' "$WORK/client.out"; then
    result=FAIL
    evidence "client did not admit (status=$client_rc); see the client output above"
    exit 1
  fi
  result=ADMITTED
fi
if [[ "${LOGIN_LOCAL_HOLD:-0}" == 1 ]]; then
  # Windows walk: after the step result is seen in the Windows client, the operator writes text containing this
  # run's id into the file below; the run then ends with WALKED. Ctrl-C without it ends with the current result.
  evidence "holding services for a Windows client; after the step result is seen write text containing run id $RUN_ID to $ATTEST_FILE (or press Ctrl-C to tear down)"
  until grep -qF "$RUN_ID" "$ATTEST_FILE" 2>/dev/null; do sleep 2; done
  result=WALKED
  evidence "walked attested_by_operator=1"
fi
