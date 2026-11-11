#!/usr/bin/env bash
# OPS-NODE-BOOT-01 §4: physical qualification with the shipped binaries.
#
# The real Platform producer (S3-A/S3-B topology, one client identity for
# evidence and intents), a TLS-verified PostgreSQL 18.6, a system service
# user running `oteryn-game-server serve`, and root running
# `oteryn-game-ops` on a separate control-plane login. The node boots,
# waits for the operator's assignment, bootstraps Characters from real
# Platform intents through its control socket, and reproduces every #823
# SEAM stage against its own bound port.
set -Eeuo pipefail
umask 077

readonly PLATFORM_SHA=3bc2d0e923db7472ec1f9b534ebe87aaffa1ef05
readonly TOPOLOGY_REVISION=wp5-node-boot-v1
readonly ACCOUNT_ID=01934f10-7c00-7000-8000-000000000001
readonly SECOND_ACCOUNT_ID=01934f10-7c00-7000-8000-000000000002
readonly PACKET_ACCOUNT_ID=01934f10-7c00-7000-8000-000000000003
readonly SECOND_PACKET_ACCOUNT_ID=01934f10-7c00-7000-8000-000000000004
readonly FRESH_KEY_ID=node-boot-fresh-key-1
readonly RECOVERY_KEY_ID=node-boot-recovery-key-1
readonly WORLD_ID=01934f10-7c02-7001-805b-3b1122334401
readonly CHANNEL_ID=01934f10-7c03-7001-805b-3b1122334401
readonly INTENT_OPERATIONS=(01934f10-7c04-7001-805b-3b1122334401 01934f10-7c04-7002-805b-3b1122334402 01934f10-7c04-7003-805b-3b1122334403 01934f10-7c04-7004-805b-3b1122334404)
# CHAR-NAME-1: bootstrap intent contract version 2 names each Character (naming policy revision 1).
readonly INTENT_NAMES=("Qualification One" "Qualification Two" "Packet One" "Packet Two")
# Runtime build writers bind to the shipped training formula's content revision,
# just as the S3-B seam does; progression is initialized from this interpretation.
readonly INTERPRETATION=(nb-profile-1 nb-ruleset-1 s3b-content-1 nb-starter-1)
readonly COMPOSE_FILE=tools/qualification/wp5_s3a/compose.yml
readonly COMPOSE_S3B=tools/qualification/wp5_s3b/compose.override.yml
readonly COMPOSE_OVERLAY=tools/qualification/node_boot/compose.override.yml
readonly PG_IMAGE=postgres:18.6-bookworm@sha256:bbf83594798e144bff365a9c191c5d779f181fb3135902064ab8ff11918020fc
readonly SERVICE_USER=oteryn-node-boot
readonly BASE=/srv/oteryn-node-boot

GAME_SOURCE="$(git rev-parse --show-toplevel)"
export NODE_BOOT_PROFILE="${NODE_BOOT_PROFILE:-sorcerer}"
case "$NODE_BOOT_PROFILE" in
  explorer|knight|paladin|sorcerer|druid|monk) ;;
  *) echo 'NODE_BOOT_RESULT=BLOCKED reason=invalid_qualification_profile'; exit 2 ;;
esac
# Optional full spell input uses this same disposable topology and admission path.
NODE_BOOT_SPELLS="${NODE_BOOT_SPELLS:-0}"
case "$NODE_BOOT_SPELLS" in
  0|1) ;;
  *) echo 'NODE_BOOT_RESULT=BLOCKED reason=invalid_spell_mode'; exit 2 ;;
esac
# Keep every fixture-only transport/door assertion intact in its own disposable pass.
NODE_BOOT_WORLD="${NODE_BOOT_WORLD:-1}"
case "$NODE_BOOT_WORLD" in 0|1) ;; *) echo 'NODE_BOOT_RESULT=BLOCKED reason=invalid_world_mode'; exit 2 ;; esac
if [[ "$NODE_BOOT_WORLD" == 1 && "$NODE_BOOT_SPELLS" == 0 ]]; then
  NODE_BOOT_WORLD=0 bash "$GAME_SOURCE/tools/qualification/node_boot/run.sh"
fi
if [[ "$NODE_BOOT_SPELLS" == 1 ]]; then NODE_BOOT_WORLD=0; fi
BOOTSTRAP_ACCOUNTS=("$ACCOUNT_ID" "$SECOND_ACCOUNT_ID")
if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
  # Packet journeys may move, loot or kill their Characters before the required core runs.
  BOOTSTRAP_ACCOUNTS+=("$PACKET_ACCOUNT_ID" "$SECOND_PACKET_ACCOUNT_ID")
fi
readonly BOOTSTRAP_ACCOUNTS
source "$GAME_SOURCE/tools/qualification/node_boot/activation_wait.sh"
NODE_ACTIVATION_WAIT_SECONDS="$(node_activation_wait_seconds "$NODE_BOOT_WORLD" "$NODE_BOOT_SPELLS")"
NODE_GAMEPLAY_ENV=()
if [[ "$NODE_BOOT_SPELLS" == 1 || "$NODE_BOOT_WORLD" == 1 ]]; then
  # Default: the canonical imported book. The r21 reference artifact stays selectable explicitly.
  NODE_BOOT_SPELL_MANIFEST="${NODE_BOOT_SPELL_MANIFEST:-$GAME_SOURCE/content/spells.manifest.json}"
  NODE_BOOT_SPELL_MANIFEST="$(realpath -e -- "$NODE_BOOT_SPELL_MANIFEST" 2>/dev/null)" || { echo 'NODE_BOOT_RESULT=BLOCKED reason=spell_manifest_missing'; exit 2; }
  [[ -f "$NODE_BOOT_SPELL_MANIFEST" ]] || { echo 'NODE_BOOT_RESULT=BLOCKED reason=spell_manifest_missing'; exit 2; }
  # The staging root is the repository root: the manifest keeps its repository-relative path.
  [[ "$NODE_BOOT_SPELL_MANIFEST" == "$GAME_SOURCE"/* ]] || { echo 'NODE_BOOT_RESULT=BLOCKED reason=spell_manifest_outside_repository'; exit 2; }
  NODE_BOOT_SPELL_MANIFEST_REL="${NODE_BOOT_SPELL_MANIFEST#"$GAME_SOURCE"/}"
  NODE_GAMEPLAY_ENV=("OTERYN_NATIVE_GAMEPLAY_MANIFEST=$BASE/gameplay/$NODE_BOOT_SPELL_MANIFEST_REL")
fi
# ADMIT-CAP-1: the entry-room seam stages run against a Channel that requires LOGOUT_V1 (21)
# and prove the owner's capability refusals; the spell scenarios declare no required set.
NODE_REQUIRED_CAPABILITIES='[21]'
if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
  # The real bundle offers map interaction, map view and spatial fields, whose
  # mandatory floor must be declared as well as the existing logout requirement.
  NODE_REQUIRED_CAPABILITIES='[11,18,21,30]'
elif [[ "$NODE_BOOT_SPELLS" == 1 ]]; then
  NODE_REQUIRED_CAPABILITIES='[]'
fi
PLATFORM_SOURCE="${PLATFORM_SOURCE:-$GAME_SOURCE/_platform}"
if [[ "$(git -C "$PLATFORM_SOURCE" rev-parse HEAD 2>/dev/null)" != "$PLATFORM_SHA" ]]; then
  echo 'NODE_BOOT_RESULT=BLOCKED reason=exact_platform_checkout_missing'
  exit 2
fi
if [[ "$NODE_BOOT_SPELLS" == 1 ]]; then
  # Execute the existing compile/decode/stage qualification before creating services.
  RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}" \
    bash "$GAME_SOURCE/tools/qualification/spells/run.sh" map "$NODE_BOOT_SPELL_MANIFEST"
fi

WORK="$(mktemp -d "${RUNNER_TEMP:-/tmp}/node-boot.XXXXXX")"
node_log="$WORK/node.log"
failed_node_log="$WORK/failed-node.log"
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
RUNTIME_DIAGNOSTICS_PID=""
ACTIVE_STAGE=setup

result=NODE_BOOT_FAIL
compose() {
  docker compose --project-name "$WP5_PROJECT" --file "$GAME_SOURCE/$COMPOSE_FILE" \
    --file "$GAME_SOURCE/$COMPOSE_S3B" --file "$GAME_SOURCE/$COMPOSE_OVERLAY" "$@"
}
evidence() { printf 'NODE_BOOT_EVIDENCE %s\n' "$*"; }
fail() { echo "NODE_BOOT_FAILURE $*"; return 1; }
stop_runtime_diagnostics() {
  [[ -z "$RUNTIME_DIAGNOSTICS_PID" ]] || {
    kill -TERM "$RUNTIME_DIAGNOSTICS_PID" 2>/dev/null || true
    wait "$RUNTIME_DIAGNOSTICS_PID" 2>/dev/null || true
    RUNTIME_DIAGNOSTICS_PID=""
  }
}
interrupted() {
  result=NODE_BOOT_FAIL
  evidence "interrupted signal=$1 stage=$ACTIVE_STAGE world_bundle=$NODE_BOOT_WORLD"
  exit "$2"
}
cleanup() {
  local rc=$?
  trap - EXIT INT TERM
  set +e
  # Flush the independent reader before either the server or its private log is removed.
  stop_runtime_diagnostics
  [[ -z "$NODE_PID" ]] || sudo kill -TERM "$NODE_PID" 2>/dev/null || true
  sudo pkill -TERM -u "$SERVICE_USER" 2>/dev/null || true
  compose down --volumes --remove-orphans --timeout 15 >/dev/null 2>&1 || true
  docker rm --force --volumes "$PG_CONTAINER" >/dev/null 2>&1 || true
  sudo rm -rf "$BASE" || true
  if [[ $rc != 0 && -n "${NODE_BOOT_FAILURE_LOG_DIR:-}" ]]; then
    mkdir -p "$NODE_BOOT_FAILURE_LOG_DIR"
    if [[ -f "$failed_node_log" ]]; then
      cp "$failed_node_log" "$NODE_BOOT_FAILURE_LOG_DIR/node.log"
    elif [[ -f "$node_log" ]]; then
      cp "$node_log" "$NODE_BOOT_FAILURE_LOG_DIR/node.log"
    fi
  fi
  find "$WORK" -type f -delete 2>/dev/null || true
  find "$WORK" -depth -type d -empty -delete 2>/dev/null || true
  evidence "cleanup=complete"
  evidence "result_scope world_bundle=$NODE_BOOT_WORLD spells=$NODE_BOOT_SPELLS stage=$ACTIVE_STAGE exit=$rc"
  echo "NODE_BOOT_RESULT=$result"
  exit "$rc"
}
trap cleanup EXIT
trap 'interrupted INT 130' INT
trap 'interrupted TERM 143' TERM

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
# Separate ephemeral Recovery key (#822 resume); fixed issuer/profile/purpose is not Fresh.
openssl genpkey -algorithm ed25519 -out "$WP5_PKI/recovery-signing.pem" >/dev/null 2>&1
RECOVERY_PUBLIC_HEX="$(openssl pkey -in "$WP5_PKI/recovery-signing.pem" -pubout -outform DER | raw_hex)"
RECOVERY_SEED_HEX="$(openssl pkey -in "$WP5_PKI/recovery-signing.pem" -outform DER | raw_hex)"
[[ ${#RECOVERY_PUBLIC_HEX} == 64 && ${#RECOVERY_SEED_HEX} == 64 ]]
evidence "pins platform=$PLATFORM_SHA topology=$TOPOLOGY_REVISION game=$(git rev-parse HEAD)"

# TLS-verified PostgreSQL 18.6 for the Game durability root.
chmod 644 "$WP5_PKI/db.crt" "$WP5_PKI/db.key"
docker run --detach --name "$PG_CONTAINER" --publish "127.0.0.1:$PG_PORT:5432" \
  --env POSTGRES_USER=oteryn_node_boot_admin --env POSTGRES_PASSWORD="$PG_ADMIN_PASSWORD" \
  --env POSTGRES_DB=postgres --volume "$WP5_PKI/db.crt:/tls/db.crt:ro" \
  --volume "$WP5_PKI/db.key:/tls/db.key:ro" --entrypoint bash "$PG_IMAGE" -c \
  'install -o postgres -m 0600 /tls/db.key /tmp/db.key && install -o postgres -m 0644 /tls/db.crt /tmp/db.crt && exec docker-entrypoint.sh postgres -c ssl=on -c ssl_cert_file=/tmp/db.crt -c ssl_key_file=/tmp/db.key' >/dev/null
chmod 600 "$WP5_PKI/db.key"
# Probe over TCP: the entrypoint's temporary init server listens on the unix socket only
# and is shut down after init, so a socket probe can pass before the real server is up.
pg_ready=0
for _ in $(seq 1 60); do
  docker exec "$PG_CONTAINER" pg_isready -h 127.0.0.1 -p 5432 -U oteryn_node_boot_admin -d postgres >/dev/null 2>&1 && { pg_ready=1; break; }
  sleep 1
done
[[ $pg_ready == 1 ]] || fail "postgres_ready"
psql_admin() { docker exec -i "$PG_CONTAINER" psql -v ON_ERROR_STOP=1 -qAt -U oteryn_node_boot_admin -d "$1"; }
echo "CREATE DATABASE oteryn_node_boot" | psql_admin postgres
[[ "$(echo 'SHOW server_version_num' | psql_admin oteryn_node_boot)" == 180006 ]] || fail "postgres_version"
[[ "$(echo 'SHOW ssl' | psql_admin oteryn_node_boot)" == on ]] || fail "postgres_tls"
ADMIN_URL="postgresql://oteryn_node_boot_admin:$PG_ADMIN_PASSWORD@127.0.0.1:$PG_PORT/oteryn_node_boot"

# The Platform topology.
compose config --quiet
compose build --pull platform
compose up --detach --wait db platform nginx
# Platform output is kept: a failing operator command must be diagnosable.
php_exec() { compose exec --no-TTY --user www-data platform php -r "$1"; }
php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); foreach ([["'"$ACCOUNT_ID"'","nb-one@example.invalid"],["'"$SECOND_ACCOUNT_ID"'","nb-two@example.invalid"]] as [$accountId,$email]) { Illuminate\Support\Facades\DB::table("identities")->insert(["email"=>$email,"password"=>password_hash(bin2hex(random_bytes(24)),PASSWORD_BCRYPT),"account_id"=>$accountId,"native_security_generation"=>1,"created_at"=>now(),"updated_at"=>now()]); } app(App\GameAuth\NativeEvidence\NativeSigningTrustRegistry::class)->publishTrustedKey(App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_ISSUER,App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_PROFILE,"fresh_admission","'"$FRESH_KEY_ID"'",hex2bin("'"$FRESH_PUBLIC_HEX"'")); app(App\GameAuth\NativeEvidence\NativeSigningTrustRegistry::class)->publishTrustedKey("urn:oteryn:platform:game-recovery","oteryn-reauth-recovery-v1","existing_actor_recovery","'"$RECOVERY_KEY_ID"'",hex2bin("'"$RECOVERY_PUBLIC_HEX"'"));'
if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
  php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); foreach ([["'"$PACKET_ACCOUNT_ID"'","nb-three@example.invalid"],["'"$SECOND_PACKET_ACCOUNT_ID"'","nb-four@example.invalid"]] as [$accountId,$email]) { Illuminate\Support\Facades\DB::table("identities")->insert(["email"=>$email,"password"=>password_hash(bin2hex(random_bytes(24)),PASSWORD_BCRYPT),"account_id"=>$accountId,"native_security_generation"=>1,"created_at"=>now(),"updated_at"=>now()]); }'
fi
evidence "platform_seed=synthetic_accounts=${#BOOTSTRAP_ACCOUNTS[@]} fresh_trust=published_public_key_only client_identities=1"

# The accelerated build is disposable: production binaries omit its file input.
# Real-clock qualification deliberately builds the ordinary shipping binary.
NODE_CLOCK_FEATURES=()
export OTERYN_NODE_BOOT_REAL_CLOCK="${OTERYN_NODE_BOOT_REAL_CLOCK:-0}"
case "$OTERYN_NODE_BOOT_REAL_CLOCK" in 0|1) ;; *) fail invalid_real_clock_mode ;; esac
if [[ "$NODE_BOOT_WORLD" == 1 && "$OTERYN_NODE_BOOT_REAL_CLOCK" == 0 ]]; then
  NODE_CLOCK_FEATURES=(--features qualification-clock)
fi
# Shipped binaries and the SEAM client harness.
cargo +1.99.1 build --locked -p oteryn-game-server --bins "${NODE_CLOCK_FEATURES[@]}"
cargo +1.99.1 test --locked -p oteryn-game-server --lib --no-run "${NODE_CLOCK_FEATURES[@]}"
TARGET="$GAME_SOURCE/target/debug"

# Deployment administration: service user, directories, roles, migration.
sudo useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER" 2>/dev/null || true
SERVICE_UID="$(id -u "$SERVICE_USER")"
sudo install -d -o root -g root -m 0755 "$BASE" "$BASE/bin" "$BASE/state" "$BASE/fence-parent" "$BASE/node"
sudo install -d -o root -g root -m 0700 "$BASE/ops"
if [[ "$NODE_BOOT_SPELLS" == 1 || "$NODE_BOOT_WORLD" == 1 ]]; then
  # Stage only declared, hash-bound inputs; never publish unrelated siblings.
  python3 - "$NODE_BOOT_SPELL_MANIFEST" "$GAME_SOURCE" "$WORK/gameplay" <<'PY_STAGE'
import hashlib
import json
import pathlib
import sys

manifest = pathlib.Path(sys.argv[1]).resolve(strict=True)
repo_root = pathlib.Path(sys.argv[2]).resolve(strict=True)
staged = pathlib.Path(sys.argv[3])
fields = {
    "catalog", "source_selection", "creature_profiles", "presentation_profiles",
    "item_profiles", "spell_appearances", "build_training", "familiar_config",
    "familiar_defenses", "wheel_profile", "source_world", "progression", "item_keys",
    "loot_tables",
    "cosmetics", "starting_loadout_item_profiles", "item_profile_updates",
}
manifest_bytes = manifest.read_bytes()
document = json.loads(manifest_bytes)
# Staged paths are repository-relative, so a locator that leaves the manifest directory
# (the canonical manifest's source_world and wheel_profile) lands where it resolves.
selected = {manifest.relative_to(repo_root).as_posix(): manifest_bytes}
for field in fields:
    pin = document.get(field)
    if pin is None:
        continue
    raw = pin["path"]
    locator = pathlib.PurePosixPath(raw)
    if locator.is_absolute() or "\\" in raw:
        raise ValueError("unsafe manifest locator")
    source = manifest.parent.joinpath(*locator.parts).resolve(strict=True)
    if not source.is_relative_to(repo_root):
        raise ValueError("manifest input escapes repository root")
    data = source.read_bytes()
    if hashlib.sha256(data).hexdigest() != pin["sha256"]:
        raise ValueError("manifest input digest mismatch")
    name = source.relative_to(repo_root).as_posix()
    if name in selected and selected[name] != data:
        raise ValueError("conflicting manifest input")
    selected[name] = data
sys.path.insert(0, str(repo_root / "tools/qualification/spells"))
from provisioning_inputs import select_cosmetics_inputs
select_cosmetics_inputs(document, manifest, repo_root, selected)
staged.mkdir(mode=0o700)
for name, data in selected.items():
    destination = staged / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(data)
PY_STAGE
  sudo install -d -o root -g root -m 0755 "$BASE/gameplay"
  sudo cp -R "$WORK/gameplay/." "$BASE/gameplay/"
  sudo chmod -R a+rX "$BASE/gameplay"
  evidence "spell_input=full_manifest manifest_sha256=$(sha256sum "$NODE_BOOT_SPELL_MANIFEST" | cut -d ' ' -f 1) activation_wait_seconds=$NODE_ACTIVATION_WAIT_SECONDS"
fi
MAP_REVISION=map-s3b-1
if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
  # Real Thais/town outskirts and the default-start region for ordinary home respawns.
  cargo +1.99.1 run --locked --release -p oteryn-world-bundle-compiler -- compile-regions "$GAME_SOURCE" "$GAME_SOURCE/content/world/pins/oteryn.identity.json" "$WORK/world.otwb" '126,125;126,126;127,127' > "$WORK/world-compile.json"
  PIN="$WORK/world.json"
  python3 - "$GAME_SOURCE/content/world/pins/oteryn.json" "$WORK/world-compile.json" "$PIN" <<'PY_REGION_PIN'
import json, sys
pin = json.load(open(sys.argv[1]))
pin["schema"] = "OTERYN_QUALIFICATION_WORLD/v1"
pin["digest"] = json.load(open(sys.argv[2]))["digest"]
pin["entry_start"] = {"x": 32369, "y": 32238, "floor": -7}
with open(sys.argv[3], "w") as output:
    json.dump(pin, output, sort_keys=True)
PY_REGION_PIN
  WORLD_DIGEST="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["digest"])' "$PIN")"
  sudo install -d -o root -g root -m 0755 "$BASE/world"
  sudo install -o root -g root -m 0644 "$WORK/world.otwb" "$BASE/world/world.otbundle"
  sudo install -o root -g root -m 0644 "$PIN" "$BASE/world/world.json"
  MAP_REVISION="$WORLD_DIGEST"
fi
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
qualification_fixture = true
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
preproduction_actor_capacity = 131072
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
map_revision = "$MAP_REVISION"
world_policy_revision = "policy-s3b-1"
required_gameplay_capabilities = $NODE_REQUIRED_CAPABILITIES
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
[platform.public_projections]
enabled = false
character_online_sessions_enabled = false
world_daily_kills_enabled = false
source_authority = "oteryn:game:primary"
client_certificate_file = "$BASE/node/secrets/public-projections.crt"
client_key_file = "$BASE/node/secrets/public-projections.key"
epoch_fence_file = "$BASE/public-projections/epoch-fence"
[launch]
authorization_file = "$BASE/state/$1"
s2_authorization_file = "$BASE/state/s2-fresh-store.json"
TOML
  if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
    python3 - "$PIN" "$BASE/world/world.otbundle" >> "$WORK/node.toml" <<'PY_WORLD'
import json, sys
pin = json.load(open(sys.argv[1]))
print(f'''[world_bundle]
path = "{sys.argv[2]}"
digest = "{pin['digest']}"
project_format_version = "{pin['project_format_version']}"
world_schema_version = "{pin['world_schema_version']}"
content_revision = "{pin['content_revision']}"
production = false
start_x = 32369
start_y = 32238
start_floor = -7''')
PY_WORLD
  fi
  sudo install -o root -m 0644 "$WORK/node.toml" "$BASE/node/node.toml"
}
# Root/client writes an atomic offset; the service can only read it. Start at
# zero on every disposable run, and keep it across the shutdown/restart probes.
if (( ${#NODE_CLOCK_FEATURES[@]} )); then
  sudo install -d -o "$(id -u)" -m 0755 "$BASE/qualification-clock"
  export OTERYN_QUALIFICATION_CLOCK_FILE="$BASE/qualification-clock/offset-ms"
  printf '0' > "$OTERYN_QUALIFICATION_CLOCK_FILE"
  chmod 0644 "$OTERYN_QUALIFICATION_CLOCK_FILE"
  NODE_GAMEPLAY_ENV+=("OTERYN_QUALIFICATION_CLOCK_FILE=$OTERYN_QUALIFICATION_CLOCK_FILE")
fi

ops() { sudo env "${NODE_GAMEPLAY_ENV[@]}" "$BASE/bin/oteryn-game-ops" --config "$BASE/ops/ops.toml" "$@"; }
start_node() {
  # Detached from the stage's output pipe, so the stage ends while it serves.
  sudo -u "$SERVICE_USER" env "${NODE_GAMEPLAY_ENV[@]}" "$BASE/bin/oteryn-game-server" serve --config "$BASE/node/node.toml" \
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
durability_report_lines() {
  # Keep the report's legacy selectors while preserving the raw node log.
  sed -E \
    -e 's/^oteryn-game-server ts=[0-9]+ level=(error|warn|info|debug) module=durability event=holder_window /DURABILITY_HOLDER_WINDOW /' \
    -e 's/^oteryn-game-server ts=[0-9]+ level=(error|warn|info|debug) module=durability event=holder_usage /DURABILITY_HOLDER_USAGE /' \
    -e 's/^oteryn-game-server ts=[0-9]+ level=(error|warn|info|debug) module=durability event=pass_dropped /DURABILITY_PASS_DROPPED /'
}
report_character_authority_failures() {
  # Startup runs outside the seam monitor. Export only the closed check names
  # from the current log snapshot, never identifiers, SQL, or stored payloads.
  python3 - "$node_log" "$1" "$NODE_BOOT_WORLD" <<'PY_CHARACTER_AUTHORITY'
import os, re, sys

path, stage, world = sys.argv[1:]
checks = {'recovery_fence', 'offline_training', 'npc_purchase', 'creation',
          'bazaar', 'revision_chain', 'appearance', 'build_chain',
          'proficiency_history', 'familiar_chain', 'spell_familiar_group',
          'spell_item_chain', 'audit_payload'}
reported = set()
try:
    with open(path, 'rb') as stream:
        remaining = os.fstat(stream.fileno()).st_size
        oversized = False
        while remaining > 0 and len(reported) < len(checks):
            piece = stream.readline(min(4097, remaining))
            if not piece:
                break
            remaining -= len(piece)
            complete = piece.endswith(b'\n')
            if oversized or len(piece) > 4096 or not complete:
                oversized = not complete
                continue
            line = piece.decode('utf-8', errors='replace')
            if not line.startswith('oteryn-game-server '):
                continue
            fields = dict(re.findall(r'\b([a-z_]+)=([^\s]+)', line))
            check = fields.get('stage')
            if (fields.get('module') == 'durability'
                    and fields.get('event') == 'character_authority_check_failed'
                    and check in checks and check not in reported):
                print(f'NODE_BOOT_CHARACTER_AUTHORITY_CHECK stage={stage} '
                      f'world_bundle={world} check={check}')
                reported.add(check)
except FileNotFoundError:
    pass
PY_CHARACTER_AUTHORITY
}
start_runtime_diagnostics() {
  # Read the regular file independently: neither a slow log consumer nor a stalled
  # qualification pipeline can hold up the server's stderr. No raw error text is copied.
  local offset=0
  [[ ! -f "$node_log" ]] || offset=$(stat -c '%s' "$node_log")
  python3 -u - "$node_log" "$1" "$NODE_BOOT_WORLD" "$offset" <<'PY_RUNTIME_DIAGNOSTICS' &
import collections, os, re, signal, sys, time

path, stage, world, initial_offset = sys.argv[1:]
stopping = False
def stop(_signal, _frame):
    global stopping
    stopping = True
signal.signal(signal.SIGINT, stop)
signal.signal(signal.SIGTERM, stop)
events = {'serve_end', 'serve_command_end', 'command_guard_failed',
          'join_prepare_failed', 'listener_poll_slow', 'listener_connection_poll_slow',
          'pass_dropped', 'step_rejected_dawnport', 'step_rejected_source_map',
          'step_rejected_dead', 'step_rejected_lane', 'step_rejected_pvp_locks',
          'step_rejected_stale_position', 'step_rejected_pending_spell',
          'step_rejected_drome', 'step_rejected_movement', 'step_rejected_holder_busy',
          'session_durability_failed', 'logout_unknown', 'logout_busy',
          'control_loss_reconcile_pending', 'control_loss_result',
          'control_loss_publication', 'control_loss_grace', 'character_authority_check_failed',
          'appearance_initial_failed', 'appearance_observation_failed'}
legacy = {'DURABILITY_PASS_SLOW', 'DURABILITY_PASS_RETIRED',
          'DURABILITY_HOLDER_STUCK', 'DURABILITY_ROOT_REESTABLISHED'}
labels = {
    'kind': {'Actor', 'SnapshotMismatch', 'ContextMismatch', 'ScopeMismatch',
             'CoordinateOverflow', 'Cell', 'Blocked', 'CapacityExceeded', 'NotQualified',
             'holder_busy', 'storage', 'deadline', 'commit_unknown', 'authority',
             'invalid_state', 'unavailable', 'unknown', 'recorded', 'resumed_history',
             'terminal', 'not_applicable', 'refused', 'pending'},
    'class': {'observation', 'mappings', 'attach', 'encode', 'content', 'fence',
              'revision', 'state', 'visibility', 'conditions'},
    'failure_stage': {'account_closure', 'activity_destination', 'activity_loss_session',
                      'activity_publish', 'activity_return', 'activity_scope',
                      'activity_successor', 'activity_view', 'actor_unreserved',
                      'admitted_binding', 'appearance_observation', 'familiar_cast',
                      'combat_hold', 'condition_save', 'death_save', 'entry', 'familiar_save',
                      'foreground_demand', 'gourmet_checkpoint', 'hunting_stamina_readback',
                      'item_effects', 'kill_rewards', 'logout_settle', 'loss_activity_origin',
                      'loss_readback', 'monk_save', 'native_publication',
                      'loss_resumed_history', 'loss_session', 'loss_session_time',
                      'loss_transferred_history', 'native_cast', 'newhaven_travel',
                      'parameters', 'pending_spell_commit', 'prey_checkpoint_fence',
                      'prey_checkpoint_session', 'proficiency_character', 'proficiency_fence',
                      'proficiency_readback', 'proficiency_snapshot', 'pvp_refresh',
                      'release_attempt', 'release_commit', 'release_fence',
                      'release_not_applicable', 'release_not_expired', 'release_retry_lifted',
                      'release_settle',
                      'resting_stamina_readback', 'retire', 'retire_session',
                      'spell_reconcile', 'staff_release', 'stamina_character', 'stamina_session',
                      'terminal_session', 'training_character', 'training_readback',
                      'training_save', 'training_session', 'wheel_gift', 'world_items',
                      'zones_after_reconcile', 'zones_before_reconcile',
                      'recovery_fence', 'offline_training', 'npc_purchase', 'creation',
                      'bazaar', 'revision_chain', 'appearance', 'build_chain',
                      'proficiency_history', 'familiar_chain', 'spell_familiar_group',
                      'spell_item_chain', 'audit_payload'},
    'reason': {'transferred', 'disconnected', 'control_lost', 'logged_out',
               'protocol_error', 'commit_outcome_unknown', 'closed', 'no_evidence'},
    'path': {'server_message_timeout', 'liveness_missed', 'probe_write_timeout',
             'updates_timeout', 'activity_transfer', 'house_transfer'},
    'guard': {'house_transfer_input', 'activity_transfer_input'},
    'phase': {'activity_scope', 'initial', 'items', 'sessions', 'world',
              'trade_open', 'trade_observation', 'inventory', 'visibility', 'appearances',
              'frame', 'frame_error', 'held', 'step_due', 'probe', 'server_message',
              'serene', 'quest_log', 'chat', 'combat', 'chase', 'target_move', 'npc'},
    'deadline_exceeded': {'true', 'false'},
    'ready': {'true', 'false'},
}
authority_stages = {'recovery_fence', 'offline_training', 'npc_purchase', 'creation',
                    'bazaar', 'revision_chain', 'appearance', 'build_chain',
                    'proficiency_history', 'familiar_chain', 'spell_familiar_group',
                    'spell_item_chain', 'audit_payload'}
numbers = {'ts', 'next_command_id', 'server_sequence', 'command_type', 'command_id',
           'ms', 'connections', 'lifecycles', 'held_ms'}
counts = collections.Counter()
event_counts = collections.Counter()
emitted = 0
limited = set()
started = time.monotonic()

def report(line):
    global emitted
    fields = dict(re.findall(r'\b([a-z_]+)=([^\s]+)', line))
    # Keep the qualification stage and the session failure stage distinct.
    fields['failure_stage'] = fields.get('stage', '')
    event = fields.get('event') if line.startswith('oteryn-game-server ') else None
    marker = line.split(' ', 1)[0]
    if marker in legacy:
        event = marker.lower()
    signature = ('permission_denied' if 'permission denied' in line else
                 'root_not_ready' if 'root is not ready' in line else None)
    if event not in events and marker not in legacy:
        if not signature:
            return
        event = 'runtime_failure'
    safe = [('event', event)]
    for key, allowed in labels.items():
        if fields.get(key) in allowed:
            safe.append((key, fields[key]))
    if event == 'character_authority_check_failed' and fields.get('stage') in authority_stages:
        safe.append(('check_stage', fields['stage']))
    for key in ('source', 'caller'):
        value = fields.get(key, '')
        if value in {'maintenance', 'none'} or re.fullmatch(
                r'(?:crates|apps)/[A-Za-z0-9_./-]{1,220}\.rs:[0-9]{1,6}:[0-9]{1,6}', value):
            safe.append((key, value))
    if signature:
        safe.append(('signature', signature))
    identity = tuple(safe)
    if counts[identity] >= 2:
        return
    # Reserve room for terminal failures even if many different slow callers recur.
    limit = {'serve_end': 32, 'durability_pass_slow': 16,
             'logout_busy': 16, 'control_loss_reconcile_pending': 16,
             'control_loss_grace': 16,
             'character_authority_check_failed': 16}.get(event, 8)
    if event_counts[event] >= limit or emitted >= 128:
        if event not in limited:
            print(f'NODE_BOOT_RUNTIME_LIMIT stage={stage} world_bundle={world} '
                  f'event={event} records={event_counts[event]}')
            limited.add(event)
        return
    counts[identity] += 1
    event_counts[event] += 1
    for key in sorted(numbers):
        if re.fullmatch(r'[0-9]{1,20}', fields.get(key, '')):
            safe.append((key, fields[key]))
    print(f'NODE_BOOT_RUNTIME stage={stage} world_bundle={world} '
          + ' '.join(f'{key}={value}' for key, value in safe)
          + f' observed_ms={int((time.monotonic() - started) * 1000)}')
    emitted += 1

print(f'NODE_BOOT_RUNTIME_MONITOR state=watching stage={stage} world_bundle={world}')
stream = None
pending = b''
oversized = False
try:
    while True:
        final_pass = stopping
        try:
            if stream is None:
                stream = open(path, 'rb')
                stream.seek(int(initial_offset))
            if os.fstat(stream.fileno()).st_size < stream.tell():
                stream.seek(0)
                pending, oversized = b'', False
            # A final flush is also bounded; it never waits for PostgreSQL or Docker.
            for _ in range(1024):
                piece = stream.readline(4097)
                if not piece:
                    break
                if not oversized:
                    pending += piece
                    oversized = len(pending) > 4096
                if piece.endswith(b'\n'):
                    if not oversized:
                        report(pending.decode('utf-8', errors='replace').rstrip('\n'))
                    pending, oversized = b'', False
        except FileNotFoundError:
            pass
        if final_pass:
            break
        if not stopping:
            time.sleep(0.25)
finally:
    pending_bytes = max(0, os.fstat(stream.fileno()).st_size - stream.tell()) if stream else 0
    if stream:
        stream.close()
    print(f'NODE_BOOT_RUNTIME_MONITOR state=stopped stage={stage} world_bundle={world} '
          f'records={emitted} pending_bytes={pending_bytes}')
PY_RUNTIME_DIAGNOSTICS
  RUNTIME_DIAGNOSTICS_PID=$!
}
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
  ACTIVE_STAGE=$name
  echo "::group::stage $name"
  evidence "stage=$name state=start world_bundle=$NODE_BOOT_WORLD spells=$NODE_BOOT_SPELLS"
  case "$name" in seam_stages|spell_scenarios) start_runtime_diagnostics "$name" ;; esac
  set +e
  ( set -Eeuo pipefail; source "$VARS"; "$@" ) 2>&1 | tee "$WORK/stage-$name.log"
  local rc=${PIPESTATUS[0]}
  stop_runtime_diagnostics
  set -e
  evidence "stage=$name state=finish world_bundle=$NODE_BOOT_WORLD exit=$rc"
  echo "::endgroup::"
  if [[ $rc == 0 ]]; then
    STATUS[$name]=PASS
    if grep -q 'e2e_all scenario=.* status=FAIL' "$WORK/stage-$name.log"; then
      echo "--- session close events behind failed e2e rows"
      grep -E 'event=(serve_end|serve_command_end|command_guard_failed|join_prepare_failed|connection_[a-z_]+)( |$)' "$node_log" 2>/dev/null | tail -n 120 || true
    fi
  else
    STATUS[$name]="FAIL(exit $rc)"
    # Later probes start new server incarnations and truncate node_log. Preserve
    # the first failed incarnation now so cleanup uploads the log that explains
    # the qualification failure rather than a subsequent successful shutdown.
    [[ -f "$failed_node_log" || ! -f "$node_log" ]] || cp "$node_log" "$failed_node_log"
    echo "NODE_BOOT_STAGE_FAILED $name exit=$rc"
    report_character_authority_failures "$name"
    echo "--- session close events"
    grep -E 'event=(serve_end|serve_command_end|command_guard_failed|join_prepare_failed)( |$)' "$node_log" 2>/dev/null | tail -n 40 || true
    echo "--- last node events"; durability_report_lines < "$node_log" 2>/dev/null | grep -v 'DURABILITY_' | tail -n 40 || true
    # Repeated background-pass retries hide the event that took the root down.
    echo "--- node events without background retries"
    durability_report_lines < "$node_log" 2>/dev/null | grep -vE 'statistics_retention|audit_collector|corpse_decay_read_deferred|server_save_countdown_read_deferred|_world_pass state=retry|admission_refused|DURABILITY_' | tail -n 150 || true
    echo "--- first root-not-ready"
    grep -n -m 1 -B 40 'root is not ready' "$node_log" 2>/dev/null | tail -n 41 || true
    # Which node events dominate, and what admission reported, beyond the tail above.
    echo "--- node event counts"
    grep -oE 'event=[a-z_]+' "$node_log" 2>/dev/null | sort | uniq -c | sort -rn | head -n 40 || true
    # Which semantic passes kept the single holder long or retired it (NBDIAG-3).
    echo "--- durability holder reports"
    durability_report_lines < "$node_log" 2>/dev/null | grep -oE 'DURABILITY_[A-Z_]+( caller=[^ ]+)?( reason=[a-z_]+)?' | sort | uniq -c | sort -rn | head -n 40 || true
    echo "--- slowest durability passes"
    grep -oE 'DURABILITY_PASS_SLOW caller=[^ ]+ held_ms=[0-9]+ deadline_exceeded=[a-z]+' "$node_log" 2>/dev/null | sort -t= -k3 -n -r | head -n 25 || true
    echo "--- holder kept or dropped by a pass"
    durability_report_lines < "$node_log" 2>/dev/null | grep -oE 'DURABILITY_(HOLDER_STUCK|PASS_DROPPED) caller=[^ ]+( held_ms=[0-9]+)?' | head -n 25 || true
    # How busy the single holder was and who kept it (NBDIAG-5).
    echo "--- holder usage windows"
    durability_report_lines < "$node_log" 2>/dev/null | grep -oE 'DURABILITY_HOLDER_WINDOW .*' | head -n 60 || true
    echo "--- holder usage by caller over the run"
    durability_report_lines < "$node_log" 2>/dev/null | grep -oE 'DURABILITY_HOLDER_USAGE caller=[^ ]+ passes=[0-9]+ held_ms=[0-9]+ refused=[0-9]+' \
        | awk '{split($2,c,"=");split($3,p,"=");split($4,h,"=");split($5,r,"=");P[c[2]]+=p[2];H[c[2]]+=h[2];R[c[2]]+=r[2]} END {for (k in H) print H[k], P[k], R[k], k}' \
        | sort -rn | head -n 30 || true
    echo "--- node message kinds"
    sed -E 's/ts=[0-9]+ //; s/trace=[0-9a-f-]+ //; s/[0-9]+/N/g' "$node_log" 2>/dev/null | cut -c1-140 | sort | uniq -c | sort -rn | head -n 40 || true
    echo "--- node admission and session events"
    durability_report_lines < "$node_log" 2>/dev/null | grep -iE 'admission|admit|refus|serve_|session_' | grep -vE 'quest_log_excluded|DURABILITY_' | tail -n 60 || true
    # How each gameplay connection entered and ended, and whether the shared listener task
    # was kept by one poll (NBDIAG-6).
    echo "--- connection entry and admission outcomes"
    grep -oE 'event=(connection_[a-z_]+|listener_[a-z_]+)( [a-z_]+=[a-z_]+)*' "$node_log" 2>/dev/null | sort | uniq -c | sort -rn | head -n 30 || true
    echo "--- connection and listener events"
    grep -E 'event=(connection_|listener_)' "$node_log" 2>/dev/null | sed -E 's/^.*(event=)/\1/' | head -n 80 || true
    echo "--- postgres sessions"
    echo "SELECT pid, usename, application_name, state, wait_event_type, wait_event, now() - backend_start, now() - xact_start, now() - state_change, left(regexp_replace(query, '\\s+', ' ', 'g'), 160) FROM pg_stat_activity WHERE datname = 'oteryn_node_boot' AND pid <> pg_backend_pid() ORDER BY backend_start" \
      | psql_admin oteryn_node_boot 2>&1 | tail -n 30 || true
    # The holder is alive but always busy: show which statements and tables keep it so.
    echo "--- postgres statements on the holder (200 samples)"
    for _ in $(seq 1 200); do
      echo "SELECT state || ' ' || left(regexp_replace(query, '\\s+', ' ', 'g'), 200) FROM pg_stat_activity WHERE datname = 'oteryn_node_boot' AND usename <> 'oteryn_node_boot_admin'" \
        | psql_admin oteryn_node_boot 2>/dev/null || true
      sleep 0.02
    done | sort | uniq -c | sort -rn | head -n 25
    echo "--- postgres table writes and scans over 10 s"
    psql_admin oteryn_node_boot 2>&1 <<'SQL' | tail -n 30 || true
CREATE TEMP TABLE node_boot_stat_before AS
  SELECT relid, n_tup_ins + n_tup_upd + n_tup_del AS writes, coalesce(seq_scan, 0) + coalesce(idx_scan, 0) AS scans
  FROM pg_stat_user_tables;
SELECT pg_sleep(10);
SELECT pg_stat_clear_snapshot();
SELECT s.relname, (s.n_tup_ins + s.n_tup_upd + s.n_tup_del) - b.writes AS writes, (coalesce(s.seq_scan, 0) + coalesce(s.idx_scan, 0)) - b.scans AS scans
  FROM pg_stat_user_tables s JOIN node_boot_stat_before b USING (relid)
  WHERE (s.n_tup_ins + s.n_tup_upd + s.n_tup_del) - b.writes + (coalesce(s.seq_scan, 0) + coalesce(s.idx_scan, 0)) - b.scans > 0
  ORDER BY 2 DESC, 3 DESC LIMIT 25;
SQL
    echo "--- postgres"; docker logs --tail 40 "$PG_CONTAINER" 2>&1 | grep -E 'ERROR|FATAL|STATEMENT' || true
    echo "--- platform"; compose logs --no-color --tail 40 platform nginx 2>&1 || true
  fi
}
node_field() { # log field
  sed -n "s/.*awaiting_assignment.*$2=\([^ \"]*\).*/\1/p" "$1" | tail -n 1
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
    sudo -u "$SERVICE_USER" env "${NODE_GAMEPLAY_ENV[@]}" "$BASE/bin/oteryn-game-server" serve --config "$BASE/node/$config.toml" 2>> "$WORK/negative.log"
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
  local boot_started=$SECONDS
  start_node
  remember NODE_PID "$NODE_PID"
  await_log awaiting_assignment 120
  local node revision
  node="$(node_field "$node_log" node_id)"; revision="$(node_field "$node_log" registration_revision)"
  remember node_a "$node"
  if ops assignment assign --request assign-ungranted.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --node-id "$node" --revision "$revision"; then
    fail "assignment without an exact-scope grant"
  fi
  # #935: the node activates the scope's current native entry Content issuance before its
  # Channel runtime exists. Issuance requires an exact-scope operation-4 grant.
  local code=0
  ops content activate --request content-ungranted.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --sequence 1 --previous empty || code=$?
  [[ $code == 6 ]] || fail "content activation without an exact-scope grant exit=$code"
  psql_admin oteryn_node_boot <<SQL
INSERT INTO game_control_scope_grants SELECT 'nb_control', '$WORLD_ID', '$CHANNEL_ID', op FROM generate_series(1, 4) op;
INSERT INTO game_control_field_policy_grants VALUES ('nb_control', '$WORLD_ID', 'policy-s3b-1');
SQL
  # Match the readiness revision with explicit control-owned non-PvP facts.
  # Authenticate as the granted login: SET ROLE cannot satisfy the insert guard.
  docker exec -i -e PGPASSWORD="$CONTROL_PASSWORD" "$PG_CONTAINER" \
    psql -v ON_ERROR_STOP=1 -qAt -h 127.0.0.1 -U nb_control -d oteryn_node_boot <<SQL
INSERT INTO game_spell_field_world_policies (world_id, world_policy_revision, world_type, protection_level, in_fight_ms, source_pin, control_role, decision_identity)
VALUES ('$WORLD_ID', 'policy-s3b-1', 1, 8, 60000, '99902524e052f37574194466c2949c576e4ab269', session_user, 'node-boot-qualification');
SQL
  ops content activate --request content-1.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --sequence 1 --previous empty
  ops content activate --request content-1.json
  ops assignment assign --request assign-a.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --node-id "$node" --revision "$revision"
  await_log "event=content_activated .*activation_sequence=1 " "$NODE_ACTIVATION_WAIT_SECONDS"
  await_log "event=readiness .*ready=true" 60
  # The canonical manifest makes each validation substantial. Prove the same
  # stale-predecessor rejection against the now-active sequence 1, without
  # spending the unassigned node's unchanged wait on this independent negative.
  code=0
  ops content activate --request content-stale.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --sequence 2 --previous empty || code=$?
  [[ $code == 6 ]] || fail "stale content activation predecessor exit=$code"
  [[ "$(echo 'SELECT count(*) FROM game_content_activations' | psql_admin oteryn_node_boot)" == 1 ]] || fail "content activation rows"
  if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
    # World admission prepares Cyclopedia/boost state against the admitted epoch.
    # A fresh database has none: initialize through the existing Control owner,
    # with the actual bundle and issued content, before any Character admission.
    local epoch_result
    epoch_result="$({
      echo 'PREPARE node_boot_world_epoch(UUID, UUID, TEXT) AS'
      cat "$GAME_SOURCE/tools/qualification/node_boot/world_epoch.sql"
      printf "EXECUTE node_boot_world_epoch('%s', '%s', '%s');\n" "$WORLD_ID" "$CHANNEL_ID" "$WORLD_DIGEST"
      printf "EXECUTE node_boot_world_epoch('%s', '%s', '%s');\n" "$WORLD_ID" "$CHANNEL_ID" "$WORLD_DIGEST"
    } | psql_admin oteryn_node_boot)"
    [[ "$epoch_result" == $'t\nf' ]] || fail "World epoch initialization/replay"
    evidence "world_epoch=0 source=control_initializer bundle=compiled content=issued replay=idempotent"
  fi
  [[ "$(sudo stat -c '%u %a' "$BASE/run/control.sock")" == "$SERVICE_UID 600" ]] || fail "control socket mode"
  [[ "$(echo 'SELECT count(*) FROM game_node_registrations' | psql_admin oteryn_node_boot)" == 1 ]] || fail "operator registered an incarnation"
  evidence "world_boot_seconds=$((SECONDS - boot_started)) world_bundle=$NODE_BOOT_WORLD"
  evidence "node registered=1 assignment=operator ungranted=refused content_activation=sequence_1 content_ungranted=refused content_replay=exact content_stale=refused readiness=true control_socket=uid${SERVICE_UID}_0600"
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
  local index=0 account operation name failed=0
  for account in "${BOOTSTRAP_ACCOUNTS[@]}"; do
    operation="${INTENT_OPERATIONS[$index]}"
    name="${INTENT_NAMES[$index]}"
    index=$((index + 1))
    if ! php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); $id=Illuminate\Support\Facades\DB::table("identities")->where("account_id","'"$account"'")->value("id"); $code=Illuminate\Support\Facades\Artisan::call("game-auth:character-bootstrap-intent:issue",["--identity-id"=>(string)$id,"--operation-id"=>"'"$operation"'","--target-world-id"=>"'"$WORLD_ID"'","--requested-name"=>"'"$name"'","--requested-sex"=>"Female","--profile-revision"=>"'"${INTERPRETATION[0]}"'","--ruleset-revision"=>"'"${INTERPRETATION[1]}"'","--content-revision"=>"'"${INTERPRETATION[2]}"'","--starter-template-revision"=>"'"${INTERPRETATION[3]}"'"]); fwrite(STDERR, Illuminate\Support\Facades\Artisan::output()); exit($code);'; then
      echo "platform intent issue failed for operation $operation"
      failed=1
      continue
    fi
    ops character bootstrap --socket "$BASE/run/control.sock" --operation-id "$operation" || { failed=1; continue; }
    # An exact retry of the same operation id is idempotent.
    ops character bootstrap --socket "$BASE/run/control.sock" --operation-id "$operation" || failed=1
  done
  [[ $failed == 0 ]]
  evidence "characters=${#BOOTSTRAP_ACCOUNTS[@]} source=platform_intents path=control_socket retry=idempotent"
}

seam_stages() { # §4.6 every #823 stage against the node's own port
  # The bootstrap evidence ages past five seconds, so each admission fetches its own.
  sleep 6
  local qualification=node_boot_seam_against_running_node
  local test_env=()
  if [[ "$NODE_BOOT_SPELLS" == 1 ]]; then
    qualification=node_boot_spells_against_running_node
    test_env=("RUST_MIN_STACK=${RUST_MIN_STACK:-16777216}")
  fi
  if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
    test_env+=("NODE_BOOT_WORLD_BUNDLE=$BASE/world/world.otbundle" "NODE_BOOT_WORLD_PIN=$BASE/world/world.json" "NODE_BOOT_MAP_REVISION=$MAP_REVISION" "RUST_MIN_STACK=${RUST_MIN_STACK:-67108864}")
    test_env+=("NODE_BOOT_PACKET_ACCOUNT_ID=$PACKET_ACCOUNT_ID" "NODE_BOOT_PACKET_SECOND_ACCOUNT_ID=$SECOND_PACKET_ACCOUNT_ID")
  fi
  local staged_manifest="${NODE_GAMEPLAY_ENV[0]:-}"
  staged_manifest="${staged_manifest#OTERYN_NATIVE_GAMEPLAY_MANIFEST=}"
  env "${NODE_GAMEPLAY_ENV[@]}" "${test_env[@]}" \
  NODE_BOOT_SPELL_MANIFEST="$staged_manifest" \
  NODE_BOOT_CONTROL_SOCKET="$BASE/run/control.sock" \
  NODE_BOOT_ADDRESS="127.0.0.1:${NODE_BOOT_GAME_PORT:-17181}" \
  NODE_BOOT_GAMEPLAY_CERT="$WP5_PKI/gameplay.crt" \
  NODE_BOOT_DATABASE_URL="$ADMIN_URL" \
  NODE_BOOT_WORLD_ID="$WORLD_ID" NODE_BOOT_CHANNEL_ID="$CHANNEL_ID" \
  WP5_S3A_PORT="$WP5_PORT" WP5_S3A_CA_CERT="$WP5_PKI/server-ca.crt" \
  WP5_S3A_CLIENT_CERT="$WP5_PKI/client.crt" WP5_S3A_CLIENT_KEY="$WP5_PKI/client.key" \
  WP5_S3A_ACCOUNT_ID="$ACCOUNT_ID" WP5_S3B_SECOND_ACCOUNT_ID="$SECOND_ACCOUNT_ID" \
  WP5_S3B_FRESH_KEY_ID="$FRESH_KEY_ID" WP5_S3B_FRESH_KEY_SEED="$FRESH_SEED_HEX" \
  WP5_S3B_RECOVERY_KEY_ID="$RECOVERY_KEY_ID" WP5_S3B_RECOVERY_KEY_SEED="$RECOVERY_SEED_HEX" \
    cargo +1.99.1 test --locked -p oteryn-game-server --lib "${NODE_CLOCK_FEATURES[@]}" \
    "qualification::$qualification" -- --ignored --exact --nocapture
}

graceful_shutdown() { # ready=false first, socket removed
  sudo kill -TERM "$NODE_PID"
  local code=0
  wait "$NODE_PID" 2>/dev/null || code=$?
  # A process started by another stage's subshell is not our child; wait for it.
  for _ in $(seq 1 30); do sudo kill -0 "$NODE_PID" 2>/dev/null || break; sleep 1; done
  grep -q "event=readiness .*ready=false" "$node_log" || fail "no ready=false on shutdown"
  grep -q "event=shutdown .*state=complete" "$node_log" || fail "no clean shutdown"
  [[ ! -e "$BASE/run/control.sock" ]] || fail "socket left after graceful shutdown"
  [[ "$(echo 'SELECT ready FROM game_durability_admission_runtime_guards' | psql_admin oteryn_node_boot)" == f ]] || fail "guard still ready"
  evidence "shutdown ready=false socket=removed"
}

restart_without_supersession() { # fails at S2 custody, never ready
  ops authorization issue --file launch-b.json --binding node-boot-b
  write_node_config launch-b.json
  local code=0
  sudo -u "$SERVICE_USER" env "${NODE_GAMEPLAY_ENV[@]}" timeout 120 "$BASE/bin/oteryn-game-server" serve --config "$BASE/node/node.toml" 2> "$WORK/restart.log" || code=$?
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
  remember node_c "$node"
  ops assignment replace --request replace-c.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --node-id "$node" --revision "$revision"
  # A new incarnation starts with no active Content and reactivates only the current issuance.
  await_log "event=content_activated .*activation_sequence=1 " "$NODE_ACTIVATION_WAIT_SECONDS"
  await_log "event=readiness .*ready=true" 60
  sudo kill -TERM "$NODE_PID"
  for _ in $(seq 1 30); do sudo kill -0 "$NODE_PID" 2>/dev/null || break; sleep 1; done
  grep -q "event=shutdown .*state=complete" "$node_log" || fail "replacement did not shut down cleanly"
  evidence "replacement=superseding custody=claimed content=reactivated_current_issuance readiness=cas"
}

signal_before_ready() { # SIGTERM in the assignment wait: clean exit, never ready
  ops authorization issue --file launch-e.json --binding node-boot-e --supersedes "$node_c"
  write_node_config launch-e.json
  : > "$node_log"
  start_node
  await_log awaiting_assignment 120
  sudo kill -TERM "$NODE_PID"
  local code=0
  wait "$NODE_PID" 2>/dev/null || code=$?
  cat "$node_log"
  [[ $code == 0 ]] || fail "signal in the assignment wait exit=$code"
  grep -q 'event=shutdown .*signal before ready' "$node_log" || fail "no pre-ready shutdown event"
  ! grep -q "event=readiness .*ready=true" "$node_log" || fail "ready published after the signal"
  [[ ! -e "$BASE/run/control.sock" ]] || fail "socket left after a pre-ready signal"
  [[ "$(echo 'SELECT ready FROM game_durability_admission_runtime_guards' | psql_admin oteryn_node_boot)" == f ]] || fail "guard became ready"
  evidence "signal_in_assignment_wait exit=0 ready=never socket=absent"
}

stage operator_setup -- operator_setup
stage configuration_negatives operator_setup -- configuration_negatives
stage node_assigned_ready operator_setup -- node_assigned_ready
stage control_socket_peer node_assigned_ready -- control_socket_peer
stage character_bootstrap node_assigned_ready -- character_bootstrap
if [[ "$NODE_BOOT_SPELLS" == 1 ]]; then
  stage spell_scenarios character_bootstrap -- seam_stages
else
  stage seam_stages character_bootstrap -- seam_stages
fi
stage graceful_shutdown node_assigned_ready -- graceful_shutdown
stage restart_without_supersession graceful_shutdown -- restart_without_supersession
stage superseding_replacement graceful_shutdown -- superseding_replacement
stage signal_before_ready superseding_replacement -- signal_before_ready
NODE_PID=""

echo "NODE_BOOT_STAGES"
failures=0
for name in "${ORDER[@]}"; do
  printf 'NODE_BOOT_STAGE %-30s %s\n' "$name" "${STATUS[$name]}"
  [[ ${STATUS[$name]} == PASS ]] || failures=$((failures + 1))
done
unset FRESH_SEED_HEX RECOVERY_SEED_HEX
if [[ $failures == 0 ]]; then
  result=NODE_BOOT_PASS
else
  exit 1
fi
