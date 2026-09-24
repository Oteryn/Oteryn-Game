#!/usr/bin/env bash
# WP5 S3-B: sealed composition of the real protected owners.
#
# Reuses the protected S3-A Platform topology (exact Platform producer behind
# TLS 1.3 mTLS nginx/FastCGI, MariaDB, retained witness) with the S3-B overlay
# for the Platform Character bootstrap-intent producer, and a PostgreSQL 17.6
# service for the Game owners (S2, #414, #415 and the durable admission store).
# Both Characters are bootstrapped from intents issued by the real Platform
# operator command immediately before the qualification run.
# The fresh grant is signed with an ephemeral Ed25519 key generated here; only
# its public half is published into the Platform trust registry, and the Game
# consumes that trust through the real S1 transport and S2 durable floor.
set -Eeuo pipefail
umask 077

readonly PLATFORM_SHA=9147bfd3a771762a6646fd87b9172cdb3a6c9a01
readonly TOPOLOGY_REVISION=wp5-s3b-v2
readonly ACCOUNT_ID=01934f10-7c00-7000-8000-000000000001
readonly SECOND_ACCOUNT_ID=01934f10-7c00-7000-8000-000000000002
readonly FRESH_KEY_ID=s3b-fresh-key-1
readonly COMPOSE_FILE=tools/qualification/wp5_s3a/compose.yml
readonly COMPOSE_OVERLAY=tools/qualification/wp5_s3b/compose.override.yml
# Must equal the harness constants (v7 identities and INTERPRETATION).
readonly WORLD_ID=01934f10-7c02-7001-805b-3b1122334401
readonly INTENT_OPERATIONS=(01934f10-7c04-7001-805b-3b1122334401 01934f10-7c04-7002-805b-3b1122334402)
readonly INTERPRETATION=(s3b-profile-1 s3b-ruleset-1 s3b-content-1 s3b-starter-1)

GAME_SOURCE="$(git rev-parse --show-toplevel)"
PLATFORM_SOURCE="${PLATFORM_SOURCE:-$GAME_SOURCE/_platform}"
if [[ ! -d "$PLATFORM_SOURCE/.git" ]]; then
  echo 'S3B_RESULT=BLOCKED reason=exact_platform_checkout_missing'
  exit 2
fi
if [[ "$(git -C "$PLATFORM_SOURCE" rev-parse HEAD)" != "$PLATFORM_SHA" ]]; then
  echo 'S3B_RESULT=BLOCKED reason=platform_revision_mismatch'
  exit 2
fi
if [[ -z "${OTERYN_TEST_POSTGRES_ADMIN_URL:-}" ]]; then
  echo 'S3B_RESULT=BLOCKED reason=postgres_17_6_service_missing'
  exit 2
fi

WORK="$(mktemp -d "${RUNNER_TEMP:-/tmp}/wp5-s3b.XXXXXX")"
WP5_PKI="$WORK/pki"
WP5_SCRATCH="$WORK/scratch"
mkdir -p "$WP5_PKI" "$WP5_SCRATCH"
WP5_PORT="${WP5_S3B_PORT:-18453}"
WP5_PROJECT="wp5s3b${GITHUB_RUN_ID:-local}${GITHUB_RUN_ATTEMPT:-1}"
WP5_DB_PASSWORD="$(openssl rand -hex 24)"
WP5_DB_ROOT_PASSWORD="$(openssl rand -hex 24)"
WP5_APP_KEY="base64:$(openssl rand -base64 32 | tr -d '\n')"
WP5_TOPOLOGY_REVISION="$TOPOLOGY_REVISION"
WP5_FSYNC_FAULT=none
export GAME_SOURCE PLATFORM_SOURCE WP5_PKI WP5_SCRATCH WP5_PORT WP5_PROJECT
export WP5_DB_PASSWORD WP5_DB_ROOT_PASSWORD WP5_APP_KEY WP5_TOPOLOGY_REVISION WP5_FSYNC_FAULT

result=COMPOSED_FAIL
cleaned=false
compose() { docker compose --project-name "$WP5_PROJECT" --file "$GAME_SOURCE/$COMPOSE_FILE" --file "$GAME_SOURCE/$COMPOSE_OVERLAY" "$@"; }
evidence() { printf 'S3B_EVIDENCE %s\n' "$*"; }
cleanup() {
  local rc=$?
  if [[ "$cleaned" != true ]]; then
    compose down --volumes --remove-orphans --timeout 15 >/dev/null 2>&1 || true
    cleaned=true
  fi
  find "$WORK" -type f -exec chmod 600 {} + 2>/dev/null || true
  find "$WORK" -type f -delete 2>/dev/null || true
  find "$WORK" -depth -type d -empty -delete 2>/dev/null || true
  evidence "cleanup=complete containers=removed volumes=removed ephemeral_keys=removed"
  echo "S3B_RESULT=$result"
  exit "$rc"
}
trap cleanup EXIT INT TERM

make_ca() {
  local prefix=$1 subject=$2
  openssl req -x509 -newkey rsa:2048 -sha256 -nodes -days 1 \
    -subj "/CN=$subject" -keyout "$WP5_PKI/$prefix.key" -out "$WP5_PKI/$prefix.crt" >/dev/null 2>&1
}
make_leaf() {
  local prefix=$1 subject=$2 ca=$3 usage=$4 san=${5:-}
  openssl req -new -newkey rsa:2048 -nodes -sha256 -subj "/CN=$subject" \
    -keyout "$WP5_PKI/$prefix.key" -out "$WP5_PKI/$prefix.csr" >/dev/null 2>&1
  local ext="$WP5_PKI/$prefix.ext"
  printf 'basicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=%s\n' "$usage" > "$ext"
  [[ -z "$san" ]] || printf 'subjectAltName=%s\n' "$san" >> "$ext"
  openssl x509 -req -sha256 -days 1 -in "$WP5_PKI/$prefix.csr" \
    -CA "$WP5_PKI/$ca.crt" -CAkey "$WP5_PKI/$ca.key" -CAcreateserial \
    -extfile "$ext" -out "$WP5_PKI/$prefix.crt" >/dev/null 2>&1
}
make_ca server-ca wp5-s3b-server-ca
make_ca client-ca wp5-s3b-client-ca
make_leaf server source.test server-ca serverAuth DNS:source.test
make_leaf client oteryn-game-native-evidence client-ca clientAuth
make_leaf intent-client oteryn-game-character-authority client-ca clientAuth
find "$WP5_PKI" -type f -name '*.key' -exec chmod 600 {} +

# Ephemeral fresh-admission signing key; the private seed never leaves $WORK
# except through the environment of the single qualification test process.
openssl genpkey -algorithm ed25519 -out "$WP5_PKI/fresh-signing.pem" >/dev/null 2>&1
raw_hex() { tail -c 32 | od -An -v -tx1 | tr -d ' \n'; }
FRESH_PUBLIC_HEX="$(openssl pkey -in "$WP5_PKI/fresh-signing.pem" -pubout -outform DER | raw_hex)"
FRESH_SEED_HEX="$(openssl pkey -in "$WP5_PKI/fresh-signing.pem" -outform DER | raw_hex)"
[[ ${#FRESH_PUBLIC_HEX} == 64 && ${#FRESH_SEED_HEX} == 64 ]]
evidence "pins platform=$PLATFORM_SHA topology=$TOPOLOGY_REVISION game=$(git rev-parse HEAD)"

compose config --quiet
compose build --pull platform
compose up --detach --wait db platform nginx

php_exec() {
  compose exec --no-TTY --user www-data platform php -r "$1" >/dev/null
}
php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); foreach ([["'"$ACCOUNT_ID"'","s3b-one@example.invalid"],["'"$SECOND_ACCOUNT_ID"'","s3b-two@example.invalid"]] as [$accountId,$email]) { Illuminate\Support\Facades\DB::table("identities")->insert(["email"=>$email,"password"=>password_hash(bin2hex(random_bytes(24)),PASSWORD_BCRYPT),"account_id"=>$accountId,"native_security_generation"=>1,"created_at"=>now(),"updated_at"=>now()]); } app(App\GameAuth\NativeEvidence\NativeSigningTrustRegistry::class)->publishTrustedKey(App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_ISSUER,App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_PROFILE,"fresh_admission","'"$FRESH_KEY_ID"'",hex2bin("'"$FRESH_PUBLIC_HEX"'"));'
evidence "platform_seed=synthetic_accounts=2 fresh_trust=published_public_key_only"

export WP5_S3A_PORT="$WP5_PORT"
export WP5_S3A_CA_CERT="$WP5_PKI/server-ca.crt"
export WP5_S3A_CLIENT_CERT="$WP5_PKI/client.crt"
export WP5_S3A_CLIENT_KEY="$WP5_PKI/client.key"
export WP5_S3A_ACCOUNT_ID="$ACCOUNT_ID"
export WP5_S3B_SECOND_ACCOUNT_ID="$SECOND_ACCOUNT_ID"
export WP5_S3B_FRESH_KEY_ID="$FRESH_KEY_ID"
export WP5_S3B_FRESH_PUBLIC_KEY="$FRESH_PUBLIC_HEX"
export WP5_S3B_INTENT_CLIENT_CERT="$WP5_PKI/intent-client.crt"
export WP5_S3B_INTENT_CLIENT_KEY="$WP5_PKI/intent-client.key"
export WP5_S3B_CHARACTER_FENCE_DIR="$WORK/character-fence"
mkdir -p "$WP5_S3B_CHARACTER_FENCE_DIR"

# Build the harness first so the 300 s intent validity is not spent compiling.
cargo +1.94.0 test --locked -p oteryn-game-server --test wp5_s3b_composition --no-run

# Real Platform operator command: one bootstrap intent per synthetic account.
index=0
for account in "$ACCOUNT_ID" "$SECOND_ACCOUNT_ID"; do
  operation="${INTENT_OPERATIONS[$index]}"
  php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); $id=Illuminate\Support\Facades\DB::table("identities")->where("account_id","'"$account"'")->value("id"); $code=Illuminate\Support\Facades\Artisan::call("game-auth:character-bootstrap-intent:issue",["--identity-id"=>(string)$id,"--operation-id"=>"'"$operation"'","--target-world-id"=>"'"$WORLD_ID"'","--profile-revision"=>"'"${INTERPRETATION[0]}"'","--ruleset-revision"=>"'"${INTERPRETATION[1]}"'","--content-revision"=>"'"${INTERPRETATION[2]}"'","--starter-template-revision"=>"'"${INTERPRETATION[3]}"'"]); exit($code);'
  index=$((index + 1))
done
evidence "platform_intents=issued count=2 path=operator_command ttl_seconds=300"
WP5_S3B_FRESH_KEY_SEED="$FRESH_SEED_HEX" \
  cargo +1.94.0 test --locked -p oteryn-game-server --test wp5_s3b_composition \
  real_owners_compose_fresh_admission_and_fence_replacement -- --ignored --exact --nocapture
unset FRESH_SEED_HEX
result=COMPOSED_PASS
