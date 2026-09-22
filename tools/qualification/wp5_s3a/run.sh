#!/usr/bin/env bash
set -Eeuo pipefail
umask 077

readonly GAME_ADMISSION_SHA=cf5c5f35476559450b6bbaf87dce519f7eead9d0
readonly PLATFORM_SHA=623435ec1b907d6d9770b767806c90300252a71c
readonly TOPOLOGY_REVISION=wp5-s3a-v1
readonly ACCOUNT_ID=01934f10-7c00-7000-8000-000000000001
readonly FRESH_KEY_ID=fresh-key-1
readonly RECOVERY_KEY_ID=recovery-key-1
readonly ROUTE=/internal/v1/game-auth/native-evidence
readonly COMPOSE_FILE=tools/qualification/wp5_s3a/compose.yml

GAME_SOURCE="$(git rev-parse --show-toplevel)"
PLATFORM_SOURCE="${PLATFORM_SOURCE:-$GAME_SOURCE/_platform}"
if [[ ! -d "$PLATFORM_SOURCE/.git" ]]; then
  echo 'S3_RESULT=BLOCKED reason=exact_platform_checkout_missing'
  exit 2
fi
if [[ "$(git -C "$PLATFORM_SOURCE" rev-parse HEAD)" != "$PLATFORM_SHA" ]]; then
  echo 'S3_RESULT=BLOCKED reason=platform_revision_mismatch'
  exit 2
fi
if ! git merge-base --is-ancestor "$GAME_ADMISSION_SHA" HEAD; then
  echo 'S3_RESULT=BLOCKED reason=game_admission_not_ancestor'
  exit 2
fi
for path in apps/game-server/src/admission_evidence.rs apps/game-server/src/native_admission_source/mod.rs apps/game-server/src/native_admission_source/descriptor.rs apps/game-server/src/native_admission_source/http1_mtls.rs; do
  [[ "$(git rev-parse "$GAME_ADMISSION_SHA:$path")" == "$(git rev-parse "HEAD:$path")" ]] || {
    echo "S3_RESULT=BLOCKED reason=protected_game_source_moved path=$path"
    exit 2
  }
done

WORK="$(mktemp -d "${RUNNER_TEMP:-/tmp}/wp5-s3a.XXXXXX")"
WP5_PKI="$WORK/pki"
WP5_SCRATCH="$WORK/scratch"
mkdir -p "$WP5_PKI" "$WP5_SCRATCH"
WP5_PORT="${WP5_S3A_PORT:-18443}"
WP5_PROJECT="wp5s3a${GITHUB_RUN_ID:-local}${GITHUB_RUN_ATTEMPT:-1}"
WP5_DB_PASSWORD="$(openssl rand -hex 24)"
WP5_DB_ROOT_PASSWORD="$(openssl rand -hex 24)"
WP5_APP_KEY="base64:$(openssl rand -base64 32 | tr -d '\n')"
WP5_TOPOLOGY_REVISION="$TOPOLOGY_REVISION"
WP5_FSYNC_FAULT=none
export GAME_SOURCE PLATFORM_SOURCE WP5_PKI WP5_SCRATCH WP5_PORT WP5_PROJECT
export WP5_DB_PASSWORD WP5_DB_ROOT_PASSWORD WP5_APP_KEY WP5_TOPOLOGY_REVISION WP5_FSYNC_FAULT

result=COMPOSED_FAIL
cleaned=false
compose() { docker compose --project-name "$WP5_PROJECT" --file "$GAME_SOURCE/$COMPOSE_FILE" "$@"; }
evidence() { printf 'S3_EVIDENCE %s\n' "$*"; }
cleanup() {
  local rc=$?
  if [[ "$cleaned" != true ]]; then
    compose down --volumes --remove-orphans --timeout 15 >/dev/null 2>&1 || true
    cleaned=true
  fi
  find "$WORK" -type f -exec chmod 600 {} + 2>/dev/null || true
  find "$WORK" -type f -delete 2>/dev/null || true
  find "$WORK" -depth -type d -empty -delete 2>/dev/null || true
  evidence "cleanup=complete containers=removed volumes=removed ephemeral_pki=removed"
  echo "S3_RESULT=$result"
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
make_ca server-ca wp5-s3a-server-ca
make_ca client-ca wp5-s3a-client-ca
make_ca wrong-ca wp5-s3a-wrong-ca
make_leaf server source.test server-ca serverAuth DNS:source.test
make_leaf client oteryn-game-native-evidence client-ca clientAuth
make_leaf wrong-client wrong-root-client wrong-ca clientAuth
make_leaf wrong-identity wrong-game-identity client-ca clientAuth
find "$WP5_PKI" -type f -name '*.key' -exec chmod 600 {} +
evidence "pins game_admission=$GAME_ADMISSION_SHA platform=$PLATFORM_SHA topology=$TOPOLOGY_REVISION"

compose config --quiet
compose pull --quiet db nginx
docker pull --platform linux/amd64 --quiet \
  'php:8.5.6-fpm-bookworm@sha256:1afa03a1445c747fc2448219aa136727b7365a9566a896a4c0d05968093e4bab'
docker pull --platform linux/amd64 --quiet \
  'composer:2.8@sha256:0d264a0f1e5be23ba363447768df7b30c33d542711ea12e37770ed7b13bf4eaa'
compose build --pull platform
for image in \
  'php:8.5.6-fpm-bookworm@sha256:1afa03a1445c747fc2448219aa136727b7365a9566a896a4c0d05968093e4bab' \
  'mariadb:11.8.8-noble@sha256:e564bcaefb87c3d2b8b09b539ea791d34f4b62c05e2e7cb03f5d6388bdc54f2c' \
  'nginx:1.29-bookworm@sha256:17ae566734b63632e543c907ba74757e0c1a25d812ab9f10a07a6bed98dd199c' \
  'composer:2.8@sha256:0d264a0f1e5be23ba363447768df7b30c33d542711ea12e37770ed7b13bf4eaa'; do
  arch="$(docker image inspect --format '{{.Os}}/{{.Architecture}}' "$image" 2>/dev/null || true)"
  [[ "$arch" == linux/amd64 ]] || { echo "S3_RESULT=BLOCKED reason=image_architecture image=${image%%@*}"; exit 2; }
done
evidence 'images=immutable linux_amd64=true php=8.5.6 mariadb=11.8.8 nginx=1.29 composer=2.8'

compose up --detach --wait db platform nginx
base_url="https://source.test:$WP5_PORT$ROUTE"
curl_base=(--silent --show-error --http1.1 --connect-timeout 2 --max-time 10 \
  --resolve "source.test:$WP5_PORT:127.0.0.1" --cacert "$WP5_PKI/server-ca.crt")
auth=(--cert "$WP5_PKI/client.crt" --key "$WP5_PKI/client.key")
account_payload='{"version":1,"operation":"ReadAccountSecurityV1","account_id":"01934f10-7c00-7000-8000-000000000001","purpose":"platform_security","scope":"fresh_admission"}'
recovery_account_payload='{"version":2,"operation":"ReadRecoveryAccountSecurityV2","account_id":"01934f10-7c00-7000-8000-000000000001","purpose":"platform_security","scope":"existing_actor_recovery"}'
fresh_trust_payload='{"version":1,"operation":"ReadFreshSigningTrustV1","issuer":"urn:oteryn:platform:game-admission","profile":"oteryn-pre-admission-v1","key_purpose":"fresh_admission","key_id":"fresh-key-1"}'

php_exec() {
  compose exec --no-TTY platform php -r "$1" >/dev/null
}
php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\\Contracts\\Console\\Kernel::class)->bootstrap(); if (!Illuminate\\Support\\Facades\\DB::table("identities")->where("account_id","'"$ACCOUNT_ID"'")->exists()) { Illuminate\\Support\\Facades\\DB::table("identities")->insert(["email"=>"s3a-synthetic@example.invalid","password"=>password_hash(bin2hex(random_bytes(24)),PASSWORD_BCRYPT),"account_id"=>"'"$ACCOUNT_ID"'","native_security_generation"=>1,"created_at"=>now(),"updated_at"=>now()]); } $r=app(App\\GameAuth\\NativeEvidence\\NativeSigningTrustRegistry::class); $r->publishTrustedKey(App\\GameAuth\\NativeEvidence\\NativeEvidenceContract::FRESH_ISSUER,App\\GameAuth\\NativeEvidence\\NativeEvidenceContract::FRESH_PROFILE,"fresh_admission","'"$FRESH_KEY_ID"'",str_repeat(chr(1),32)); $r->publishTrustedKey(App\\GameAuth\\NativeEvidence\\NativeEvidenceContract::RECOVERY_ISSUER,App\\GameAuth\\NativeEvidence\\NativeEvidenceContract::RECOVERY_PROFILE,App\\GameAuth\\NativeEvidence\\NativeEvidenceContract::RECOVERY_KEY_PURPOSE,"'"$RECOVERY_KEY_ID"'",str_repeat(chr(2),32));'

expect_status() {
  local expected=$1 payload=$2 output=$3
  local status
  status="$(curl "${curl_base[@]}" "${auth[@]}" -H 'Content-Type: application/json' -o "$output" -w '%{http_code}' --data-binary "$payload" "$base_url")"
  [[ "$status" == "$expected" ]] || { echo "unexpected_http_status=$status expected=$expected" >&2; return 1; }
}
expect_unavailable() {
  local payload=$1 output=$2
  expect_status 200 "$payload" "$output"
  grep -Eq '^\{"version":[12],"operation":"[A-Za-z0-9]+","result":"unavailable"\}$' "$output"
  [[ "$(wc -c < "$output")" -le 8192 ]]
}

# Real transport provenance negatives.
if curl "${curl_base[@]}" -o /dev/null "$base_url" >/dev/null 2>&1; then exit 1; fi
if curl "${curl_base[@]}" --cert "$WP5_PKI/wrong-client.crt" --key "$WP5_PKI/wrong-client.key" -o /dev/null "$base_url" >/dev/null 2>&1; then exit 1; fi
if curl "${curl_base[@]}" "${auth[@]}" --tlsv1.2 --tls-max 1.2 -o /dev/null "$base_url" >/dev/null 2>&1; then exit 1; fi
wrong_status="$(curl "${curl_base[@]}" --cert "$WP5_PKI/wrong-identity.crt" --key "$WP5_PKI/wrong-identity.key" \
  -H 'SSL_CLIENT_VERIFY: SUCCESS' -H 'SSL_PROTOCOL: TLSv1.3' -H 'SSL_CLIENT_S_DN: CN=oteryn-game-native-evidence' \
  -o "$WP5_SCRATCH/wrong-identity" -w '%{http_code}' --data-binary "$account_payload" "$base_url")"
[[ "$wrong_status" == 401 ]]
evidence 'mtls=tls1.3_required no_cert=rejected wrong_root=rejected lower_tls=rejected wrong_identity=rejected public_header_substitution=erased'

# Closed parser and HTTP bounds.
for bad in \
  '{' \
  '{"version":1,"operation":"ReadAccountSecurityV1","operation":"ReadAccountSecurityV1"}' \
  '{"version":1,"operation":"ReadAccountSecurityV1","unknown":"x"}' \
  '{"version":1,"operation":{"nested":true}}'; do
  expect_status 400 "$bad" "$WP5_SCRATCH/bad-response"
done
oversize="$(printf '%*s' 1025 '' | tr ' ' x)"
expect_status 413 "$oversize" "$WP5_SCRATCH/oversize-response"
expect_status 200 "$account_payload" "$WP5_SCRATCH/account-response"
[[ "$(wc -c < "$WP5_SCRATCH/account-response")" -le 8192 ]]
evidence 'request_bound=1024 response_bound=8192 malformed=closed duplicate=closed nested=closed unknown=closed'

export WP5_S3A_PORT="$WP5_PORT"
export WP5_S3A_CA_CERT="$WP5_PKI/server-ca.crt"
export WP5_S3A_CLIENT_CERT="$WP5_PKI/client.crt"
export WP5_S3A_CLIENT_KEY="$WP5_PKI/client.key"
export WP5_S3A_ACCOUNT_ID="$ACCOUNT_ID"
export WP5_S3A_FRESH_KEY_ID="$FRESH_KEY_ID"
export WP5_S3A_RECOVERY_KEY_ID="$RECOVERY_KEY_ID"
cargo +1.94.0 test --locked -p oteryn-game-server --test native_admission_source_real_interop real_platform_producer_decodes_all_four_operations -- --ignored --exact --nocapture

# Hold the relational read briefly while two genuine S1 requests occupy both Game slots.
compose exec --no-TTY -e MARIADB_PWD="$WP5_DB_ROOT_PASSWORD" db \
  mariadb -uroot oteryn_s3a -e 'LOCK TABLES identities WRITE; DO SLEEP(2); UNLOCK TABLES' >/dev/null &
locker=$!
sleep 0.2
cargo +1.94.0 test --locked -p oteryn-game-server --test native_admission_source_real_interop real_capacity_two_inflight_rejects_third -- --ignored --exact --nocapture
wait "$locker"
evidence 'capacity=two_inflight third=immediate_reject application_queue=none client_profile=two_active_eight_queued'

# Relational outage is bounded and does not escape the closed failure shape.
compose stop db >/dev/null
expect_unavailable "$account_payload" "$WP5_SCRATCH/db-down-response"
compose start db >/dev/null
until compose exec --no-TTY -e MARIADB_PWD="$WP5_DB_ROOT_PASSWORD" db mariadb-admin ping -h 127.0.0.1 -uroot --silent >/dev/null 2>&1; do sleep 1; done
evidence 'relational_failure=bounded_unavailable'

store_digest_before="$(compose exec --no-TTY platform sh -c 'sha256sum /var/lib/oteryn-witness/witness-store.id' | awk '{print $1}')"
compose restart platform nginx >/dev/null
compose up --detach --wait platform nginx >/dev/null
expect_status 200 "$account_payload" "$WP5_SCRATCH/restart-response"
store_digest_after="$(compose exec --no-TTY platform sh -c 'sha256sum /var/lib/oteryn-witness/witness-store.id' | awk '{print $1}')"
[[ "$store_digest_before" == "$store_digest_after" ]]
evidence "process_restart=retained witness_store_digest=$store_digest_after"

# Restore a synthetic database snapshot with only the witness-store binding omitted.
compose exec --no-TTY -e MARIADB_PWD="$WP5_DB_ROOT_PASSWORD" db mariadb-dump -uroot \
  --single-transaction --skip-comments --skip-dump-date oteryn_s3a > "$WP5_SCRATCH/database.sql"
sed '/^INSERT INTO `native_game_evidence_witness_stores`/d' "$WP5_SCRATCH/database.sql" > "$WP5_SCRATCH/database-no-binding.sql"
compose stop nginx platform >/dev/null
compose exec --no-TTY -e MARIADB_PWD="$WP5_DB_ROOT_PASSWORD" db mariadb -uroot \
  -e 'DROP DATABASE oteryn_s3a; CREATE DATABASE oteryn_s3a CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci; GRANT ALL ON oteryn_s3a.* TO "oteryn_s3a"@"%"' >/dev/null
compose exec --no-TTY -e MARIADB_PWD="$WP5_DB_ROOT_PASSWORD" db mariadb -uroot oteryn_s3a < "$WP5_SCRATCH/database-no-binding.sql"
compose start platform >/dev/null
compose up --detach --wait platform nginx >/dev/null
expect_status 200 "$account_payload" "$WP5_SCRATCH/restore-response"
rebound="$(compose exec --no-TTY -e MARIADB_PWD="$WP5_DB_ROOT_PASSWORD" db mariadb -N -uroot oteryn_s3a -e 'SELECT COUNT(*) FROM native_game_evidence_witness_stores WHERE id=1')"
[[ "$rebound" == 1 ]]
evidence 'database_restore=retained_witness_rebound'

# Empty replacement witness is rejected while relational history remains; restore retained files afterward.
compose exec --no-TTY platform sh -c 'tar -C /var/lib/oteryn-witness -cf /run/wp5/witness.tar . && find /var/lib/oteryn-witness -mindepth 1 -maxdepth 1 -delete'
expect_unavailable "$account_payload" "$WP5_SCRATCH/replacement-response"
compose exec --no-TTY platform sh -c 'tar -C /var/lib/oteryn-witness -xf /run/wp5/witness.tar'
expect_status 200 "$account_payload" "$WP5_SCRATCH/replacement-restored-response"
evidence 'replacement_witness=rejected retained_witness=restored'

# Account witness-ahead rollback and explicit forward-only reconciliation.
php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\\Contracts\\Console\\Kernel::class)->bootstrap(); $id=App\\Identity\\Models\\Identity::query()->where("account_id","'"$ACCOUNT_ID"'")->firstOrFail(); Illuminate\\Support\\Facades\\DB::beginTransaction(); try { app(App\\Identity\\Actions\\RevokeIdentityGameAuthorizations::class)->execute($id); } finally { Illuminate\\Support\\Facades\\DB::rollBack(); }'
expect_unavailable "$account_payload" "$WP5_SCRATCH/account-ambiguous-response"
compose exec --no-TTY platform php artisan game-auth:native-evidence:reconcile --account-id="$ACCOUNT_ID" --no-interaction >/dev/null
expect_status 200 "$account_payload" "$WP5_SCRATCH/account-reconciled-response"
evidence 'account_rollback=unavailable account_reconcile=forward_only'

# Signing witness-ahead rollback, conservative revocation, then a fresh successor key/profile.
php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\\Contracts\\Console\\Kernel::class)->bootstrap(); $r=app(App\\GameAuth\\NativeEvidence\\NativeSigningTrustRegistry::class); Illuminate\\Support\\Facades\\DB::beginTransaction(); try { $r->revokeProfile(App\\GameAuth\\NativeEvidence\\NativeEvidenceContract::FRESH_ISSUER,App\\GameAuth\\NativeEvidence\\NativeEvidenceContract::FRESH_PROFILE,"fresh_admission"); } finally { Illuminate\\Support\\Facades\\DB::rollBack(); }'
expect_unavailable "$fresh_trust_payload" "$WP5_SCRATCH/trust-ambiguous-response"
compose exec --no-TTY platform php artisan game-auth:native-evidence:reconcile --trust=fresh --no-interaction >/dev/null
php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\\Contracts\\Console\\Kernel::class)->bootstrap(); app(App\\GameAuth\\NativeEvidence\\NativeSigningTrustRegistry::class)->publishNextProfileVersion(App\\GameAuth\\NativeEvidence\\NativeEvidenceContract::FRESH_ISSUER,App\\GameAuth\\NativeEvidence\\NativeEvidenceContract::FRESH_PROFILE,"fresh_admission","fresh-key-2",str_repeat(chr(3),32));'
WP5_S3A_FRESH_KEY_ID=fresh-key-2 cargo +1.94.0 test --locked -p oteryn-game-server --test native_admission_source_real_interop real_platform_producer_decodes_all_four_operations -- --ignored --exact --nocapture
evidence 'trust_rollback=unavailable trust_reconcile=revoked successor_profile=fresh_key'

# The path-scoped interposer runs in the exact PHP ABI and distinguishes file and directory fsync.
for mode in file directory; do
  WP5_FSYNC_FAULT="$mode"; export WP5_FSYNC_FAULT
  compose up --detach --wait --force-recreate --no-deps platform >/dev/null
  expect_unavailable "$account_payload" "$WP5_SCRATCH/fsync-$mode-response"
  WP5_FSYNC_FAULT=none; export WP5_FSYNC_FAULT
  compose up --detach --wait --force-recreate --no-deps platform >/dev/null
  expect_status 200 "$account_payload" "$WP5_SCRATCH/fsync-$mode-recovered-response"
  evidence "fsync_fault=$mode result=unavailable recovery=observed"
done

history="$(compose exec --no-TTY -e MARIADB_PWD="$WP5_DB_ROOT_PASSWORD" db mariadb -N -uroot oteryn_s3a -e 'SELECT COUNT(*) FROM native_game_evidence_observations')"
floors="$(compose exec --no-TTY platform sh -c 'find /var/lib/oteryn-witness -maxdepth 1 -type f -name "*.floor" | wc -l')"
(( history > 0 && floors > 0 ))
compose down --volumes --remove-orphans --timeout 15 >/dev/null
cleaned=true
evidence 'simultaneous_database_and_witness_loss=BLOCKED qualified_restore=false authority_recovery_not_attempted=true'
result=COMPOSED_PASS
evidence "result=COMPOSED_PASS game=$(git rev-parse HEAD) platform=$PLATFORM_SHA topology=$TOPOLOGY_REVISION"
