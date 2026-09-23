#!/usr/bin/env bash
set -Eeuo pipefail
umask 077

readonly GAME_ADMISSION_SHA=cf5c5f35476559450b6bbaf87dce519f7eead9d0
readonly PLATFORM_SHA=623435ec1b907d6d9770b767806c90300252a71c
readonly TOPOLOGY_REVISION=wp5-s3a-v1
readonly ACCOUNT_ID=01934f10-7c00-7000-8000-000000000001
readonly CAPACITY_ACCOUNT_ID=01934f10-7c00-7000-8000-000000000002
readonly FRESH_KEY_ID=fresh-key-1
readonly RECOVERY_KEY_ID=recovery-key-1
readonly FRESH_KEY_BYTE=1
readonly RECOVERY_KEY_BYTE=2
readonly SUCCESSOR_FRESH_KEY_BYTE=3
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
# The composed topology is executed by the runner's Docker Engine and Compose;
# record their exact versions with the result.
docker_engine_version="$(docker version --format '{{.Server.Version}}')"
docker_compose_version="$(docker compose version --short)"
[[ -n "$docker_engine_version" && -n "$docker_compose_version" ]]
evidence "docker_engine=$docker_engine_version docker_compose=$docker_compose_version"

compose up --detach --wait db platform nginx
witness_owner="$(compose exec --no-TTY platform stat -c '%U:%G' /var/lib/oteryn-witness)"
[[ "$witness_owner" == www-data:www-data ]]
evidence "witness_owner=$witness_owner"
base_url="https://source.test:$WP5_PORT$ROUTE"
curl_base=(--silent --show-error --http1.1 --connect-timeout 2 --max-time 10 \
  --resolve "source.test:$WP5_PORT:127.0.0.1" --cacert "$WP5_PKI/server-ca.crt")
auth=(--cert "$WP5_PKI/client.crt" --key "$WP5_PKI/client.key")
account_payload='{"version":1,"operation":"ReadAccountSecurityV1","account_id":"01934f10-7c00-7000-8000-000000000001","purpose":"platform_security","scope":"fresh_admission"}'
recovery_account_payload='{"version":2,"operation":"ReadRecoveryAccountSecurityV2","account_id":"01934f10-7c00-7000-8000-000000000001","purpose":"platform_security","scope":"existing_actor_recovery"}'
capacity_account_payload='{"version":2,"operation":"ReadRecoveryAccountSecurityV2","account_id":"01934f10-7c00-7000-8000-000000000002","purpose":"platform_security","scope":"existing_actor_recovery"}'
fresh_trust_payload='{"version":1,"operation":"ReadFreshSigningTrustV1","issuer":"urn:oteryn:platform:game-admission","profile":"oteryn-pre-admission-v1","key_purpose":"fresh_admission","key_id":"fresh-key-1"}'

php_exec() {
  compose exec --no-TTY --user www-data platform php -r "$1" >/dev/null
}
php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); foreach ([["'"$ACCOUNT_ID"'","s3a-synthetic@example.invalid"],["'"$CAPACITY_ACCOUNT_ID"'","s3a-capacity@example.invalid"]] as [$accountId,$email]) { if (!Illuminate\Support\Facades\DB::table("identities")->where("account_id",$accountId)->exists()) { Illuminate\Support\Facades\DB::table("identities")->insert(["email"=>$email,"password"=>password_hash(bin2hex(random_bytes(24)),PASSWORD_BCRYPT),"account_id"=>$accountId,"native_security_generation"=>1,"created_at"=>now(),"updated_at"=>now()]); } } $r=app(App\GameAuth\NativeEvidence\NativeSigningTrustRegistry::class); $r->publishTrustedKey(App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_ISSUER,App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_PROFILE,"fresh_admission","'"$FRESH_KEY_ID"'",str_repeat(chr('"$FRESH_KEY_BYTE"'),32)); $r->publishTrustedKey(App\GameAuth\NativeEvidence\NativeEvidenceContract::RECOVERY_ISSUER,App\GameAuth\NativeEvidence\NativeEvidenceContract::RECOVERY_PROFILE,App\GameAuth\NativeEvidence\NativeEvidenceContract::RECOVERY_KEY_PURPOSE,"'"$RECOVERY_KEY_ID"'",str_repeat(chr('"$RECOVERY_KEY_BYTE"'),32));'

expect_status() {
  local expected=$1 payload=$2 output=$3
  local status
  status="$(curl "${curl_base[@]}" "${auth[@]}" -H 'Content-Type: application/json' -o "$output" -w '%{http_code}' --data-binary "$payload" "$base_url")"
  [[ "$status" == "$expected" ]] || { echo "unexpected_http_status=$status expected=$expected" >&2; return 1; }
}
# The complete response body must equal the exact closed unavailable object for
# the originating request; no extra line, byte or whitespace is accepted.
expect_exact_unavailable_body() {
  local output=$1 operation=$2 version=$3
  printf '{"version":%s,"operation":"%s","result":"unavailable"}' "$version" "$operation" | cmp -s - "$output"
}
expect_unavailable() {
  local payload=$1 output=$2 operation=$3 version=$4
  expect_status 200 "$payload" "$output"
  expect_exact_unavailable_body "$output" "$operation" "$version"
}
expect_observed_account() {
  local payload=$1 output=$2 operation=$3 version=$4 account_id=$5 purpose=$6 scope=$7
  local revision_floor=${8:-0} revision_relation=${9:-positive}
  expect_status 200 "$payload" "$output"
  validate_observed_account "$output" "$operation" "$version" "$account_id" "$purpose" "$scope" \
    "$revision_floor" "$revision_relation"
}
validate_observed_account() {
  local output=$1 operation=$2 version=$3 account_id=$4 purpose=$5 scope=$6
  local revision_floor=${7:-0} revision_relation=${8:-positive}
  python3 - "$output" "$operation" "$version" "$account_id" "$purpose" "$scope" \
    "$revision_floor" "$revision_relation" <<'PY'
import json
import pathlib
import sys

path, operation, version, account_id, purpose, scope, revision_floor, revision_relation = sys.argv[1:]
body = pathlib.Path(path).read_bytes()
if not body or len(body) > 8192:
    raise SystemExit("observed response body is empty or oversized")

def closed_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate response member: {key}")
        result[key] = value
    return result

try:
    response = json.loads(body.decode("utf-8"), object_pairs_hook=closed_object)
except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
    raise SystemExit(f"malformed observed response: {error}") from error

expected_keys = {
    "version", "operation", "result", "source_authority", "source_revision",
    "decision_identity", "source_observed_at", "clock_uncertainty_seconds",
    "account_id", "purpose", "scope", "allowed", "minimum_valid_generation",
}
if not isinstance(response, dict) or set(response) != expected_keys:
    raise SystemExit("observed response has an unknown or missing member")
expected = {
    "version": int(version), "operation": operation, "result": "observed",
    "source_authority": "platform", "account_id": account_id,
    "purpose": purpose, "scope": scope, "allowed": True,
}
# Exact JSON types: Python equates 1/True, which the Game decoder rejects.
if any(type(response.get(key)) is not type(value) or response.get(key) != value
       for key, value in expected.items()):
    raise SystemExit("observed response has wrong operation, binding, authority, or result")
# Mirror the exact Game decoder: text members are at most 256 bytes, numeric
# tokens are canonical ASCII digits bounded by u64 (source_observed_at: at most
# 19 digits and i64), and positive where the decoder requires it.
if any(isinstance(value, str) and len(value.encode("utf-8")) > 256 for value in response.values()):
    raise SystemExit("observed response has an oversized text member")
for key, allow_zero, maximum, digits in (
    ("source_revision", False, 2**64 - 1, 20), ("source_observed_at", False, 2**63 - 1, 19),
    ("clock_uncertainty_seconds", True, 2**64 - 1, 20),
    ("minimum_valid_generation", False, 2**64 - 1, 20),
):
    value = response.get(key)
    if (not isinstance(value, str) or not value.isascii() or not value.isdigit()
            or len(value) > digits):
        raise SystemExit(f"observed response has malformed {key}")
    parsed = int(value)
    if str(parsed) != value or (parsed == 0 and not allow_zero) or parsed > maximum:
        raise SystemExit(f"observed response has non-canonical {key}")
if response["decision_identity"] != response["source_revision"]:
    raise SystemExit("observed response has invalid decision provenance")
revision = int(response["source_revision"])
floor = int(revision_floor)
if revision_relation == "at_least" and revision < floor:
    raise SystemExit("observed response regressed below the pre-fault source revision")
if revision_relation == "greater" and revision <= floor:
    raise SystemExit("observed response did not create the required successor decision")
if revision_relation not in {"positive", "at_least", "greater"}:
    raise SystemExit("unknown source revision relation")
if int(response["clock_uncertainty_seconds"]) > 5:
    raise SystemExit("observed response exceeds clock uncertainty bound")
PY
}
observed_revision() {
  python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["source_revision"])' "$1"
}
expect_observed_trust() {
  local payload=$1 output=$2 expected_trusted=$3 expected_key_byte=$4 revision_floor=$5 revision_relation=$6
  expect_status 200 "$payload" "$output"
  python3 - "$output" "$expected_trusted" "$expected_key_byte" "$revision_floor" "$revision_relation" <<'PY'
import base64
import json
import pathlib
import sys

path, expected_trusted, expected_key_byte, revision_floor, revision_relation = sys.argv[1:]
body = pathlib.Path(path).read_bytes()
if not body or len(body) > 8192:
    raise SystemExit("observed trust response body is empty or oversized")

def closed_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate response member: {key}")
        result[key] = value
    return result

try:
    response = json.loads(body.decode("utf-8"), object_pairs_hook=closed_object)
except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
    raise SystemExit(f"malformed observed trust response: {error}") from error
expected_keys = {
    "version", "operation", "result", "source_authority", "source_revision",
    "decision_identity", "source_observed_at", "clock_uncertainty_seconds",
    "issuer", "profile", "key_purpose", "key_id", "trusted", "public_key",
}
if not isinstance(response, dict) or set(response) != expected_keys:
    raise SystemExit("observed trust response has an unknown or missing member")
expected = {
    "version": 1, "operation": "ReadFreshSigningTrustV1", "result": "observed",
    "source_authority": "platform", "issuer": "urn:oteryn:platform:game-admission",
    "profile": "oteryn-pre-admission-v1", "key_purpose": "fresh_admission",
    "key_id": "fresh-key-1", "trusted": expected_trusted == "true",
}
# Exact JSON types: Python equates 1/True, which the Game decoder rejects.
if any(type(response.get(key)) is not type(value) or response.get(key) != value
       for key, value in expected.items()):
    raise SystemExit("observed trust response has wrong operation, binding, authority, or result")
# Mirror the exact Game decoder bounds (see the account validator).
if any(isinstance(value, str) and len(value.encode("utf-8")) > 256 for value in response.values()):
    raise SystemExit("observed trust response has an oversized text member")
for key, allow_zero, maximum, digits in (
    ("source_revision", False, 2**64 - 1, 20), ("source_observed_at", False, 2**63 - 1, 19),
    ("clock_uncertainty_seconds", True, 2**64 - 1, 20),
):
    value = response.get(key)
    if (not isinstance(value, str) or not value.isascii() or not value.isdigit()
            or len(value) > digits):
        raise SystemExit(f"observed trust response has malformed {key}")
    parsed = int(value)
    if str(parsed) != value or (parsed == 0 and not allow_zero) or parsed > maximum:
        raise SystemExit(f"observed trust response has non-canonical {key}")
if response["decision_identity"] != response["source_revision"]:
    raise SystemExit("observed trust response has invalid decision provenance")
revision = int(response["source_revision"])
floor = int(revision_floor)
if revision_relation == "at_least" and revision < floor:
    raise SystemExit("observed trust response regressed below the pre-fault source revision")
if revision_relation == "greater" and revision <= floor:
    raise SystemExit("observed trust response did not create the required successor decision")
if revision_relation not in {"positive", "at_least", "greater"}:
    raise SystemExit("unknown source revision relation")
if int(response["clock_uncertainty_seconds"]) > 5:
    raise SystemExit("observed trust response exceeds clock uncertainty bound")
# The Game decoder accepts only the exact 43-character unpadded URL-safe
# encoding of 32 bytes (URL_SAFE_NO_PAD, canonical trailing bits).
encoded_key = response["public_key"]
if (not isinstance(encoded_key, str) or len(encoded_key) != 43
        or any(ch not in "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
               for ch in encoded_key)):
    raise SystemExit("observed trust response has malformed key material")
try:
    public_key = base64.urlsafe_b64decode(encoded_key + "=")
except Exception as error:
    raise SystemExit("observed trust response has malformed key material") from error
if len(public_key) != 32 or base64.urlsafe_b64encode(public_key).decode().rstrip("=") != encoded_key:
    raise SystemExit("observed trust response has non-canonical key material")
if public_key != bytes([int(expected_key_byte)]) * 32:
    raise SystemExit("observed trust response has wrong key material")
PY
}
# A transport negative must fail at the TLS boundary itself: either the
# handshake fails, or nginx's client-certificate verification returns its own
# 400 page before any FastCGI/producer processing. Producer 4xx never counts.
expect_transport_rejected() {
  local label=$1 expected=$2
  shift 2
  local status rc output="$WP5_SCRATCH/transport-$label"
  set +e
  status="$(curl "${curl_base[@]}" "$@" -H 'Content-Type: application/json' -o "$output" -w '%{http_code}' --data-binary "$account_payload" "$base_url" 2>/dev/null)"
  rc=$?
  set -e
  if [[ "$expected" == handshake ]]; then
    [[ "$rc" -ne 0 && "$status" == 000 ]] || { echo "transport_negative=$label rc=$rc status=$status" >&2; return 1; }
  else
    [[ "$rc" -eq 0 && "$status" == 400 ]] && grep -Fq "$expected" "$output" \
      || { echo "transport_negative=$label rc=$rc status=$status" >&2; return 1; }
  fi
  evidence "transport_negative=$label rejected=true boundary=$([[ "$expected" == handshake ]] && echo tls_handshake || echo nginx_client_verify)"
}

# Real transport provenance negatives.
expect_transport_rejected no_client_cert 'No required SSL certificate was sent'
expect_transport_rejected wrong_root 'The SSL certificate error' --cert "$WP5_PKI/wrong-client.crt" --key "$WP5_PKI/wrong-client.key"
expect_transport_rejected lower_tls handshake "${auth[@]}" --tlsv1.2 --tls-max 1.2
wrong_status="$(curl "${curl_base[@]}" --cert "$WP5_PKI/wrong-identity.crt" --key "$WP5_PKI/wrong-identity.key" \
  -H 'SSL_CLIENT_VERIFY: SUCCESS' -H 'SSL_PROTOCOL: TLSv1.3' -H 'SSL_CLIENT_S_DN: CN=oteryn-game-native-evidence' \
  -o "$WP5_SCRATCH/wrong-identity" -w '%{http_code}' --data-binary "$account_payload" "$base_url")"
[[ "$wrong_status" == 401 ]]
evidence 'mtls=tls1.3_required no_cert=rejected wrong_root=rejected lower_tls=rejected wrong_identity=rejected public_header_substitution=erased'

# Closed parser and HTTP bounds. Each structural probe starts from the valid
# account request and mutates only the condition under test.
duplicate_payload="${account_payload%\}},\"purpose\":\"platform_security\"}"
unknown_payload="${account_payload%\}},\"unknown\":\"x\"}"
nested_payload="${account_payload/'"operation":"ReadAccountSecurityV1"'/'"operation":{"nested":true}'}"
[[ "$duplicate_payload" != "$account_payload" && "$unknown_payload" != "$account_payload" && "$nested_payload" != "$account_payload" ]]
for bad in '{' "$duplicate_payload" "$unknown_payload" "$nested_payload"; do
  expect_status 400 "$bad" "$WP5_SCRATCH/bad-response"
done
oversize="$(printf '%*s' 1025 '' | tr ' ' x)"
expect_status 413 "$oversize" "$WP5_SCRATCH/oversize-response"
expect_observed_account "$account_payload" "$WP5_SCRATCH/account-response" \
  ReadAccountSecurityV1 1 "$ACCOUNT_ID" platform_security fresh_admission
evidence 'request_bound=1024 response_bound=8192 malformed=closed duplicate=closed nested=closed unknown=closed'

export WP5_S3A_PORT="$WP5_PORT"
export WP5_S3A_CA_CERT="$WP5_PKI/server-ca.crt"
export WP5_S3A_CLIENT_CERT="$WP5_PKI/client.crt"
export WP5_S3A_CLIENT_KEY="$WP5_PKI/client.key"
export WP5_S3A_ACCOUNT_ID="$ACCOUNT_ID"
export WP5_S3A_CAPACITY_ACCOUNT_ID="$CAPACITY_ACCOUNT_ID"
export WP5_S3A_FRESH_KEY_ID="$FRESH_KEY_ID"
export WP5_S3A_RECOVERY_KEY_ID="$RECOVERY_KEY_ID"
export WP5_S3A_FRESH_KEY_BYTE="$FRESH_KEY_BYTE"
export WP5_S3A_RECOVERY_KEY_BYTE="$RECOVERY_KEY_BYTE"
cargo +1.94.0 test --locked -p oteryn-game-server --test native_admission_source_real_interop real_platform_producer_decodes_all_four_operations -- --ignored --exact --nocapture

# Keep the Game-side two-active proof separate from the producer-side boundary.
compose exec --no-TTY -e MYSQL_PWD="$WP5_DB_ROOT_PASSWORD" db \
  mariadb -uroot oteryn_s3a -e 'LOCK TABLES identities WRITE; DO SLEEP(2); UNLOCK TABLES' >/dev/null &
locker=$!
sleep 0.2
cargo +1.94.0 test --locked -p oteryn-game-server --test native_admission_source_real_interop real_capacity_two_inflight_rejects_third -- --ignored --exact --nocapture
wait "$locker"
evidence 'game_capacity=two_active third=immediate_reject client_profile=two_active_eight_queued'

# Hold two authenticated requests after they acquire the producer's two atomic
# capacity locks. A third authenticated request must traverse nginx/FastCGI and
# return unavailable before the held database reads can complete.
compose exec --no-TTY -e MYSQL_PWD="$WP5_DB_ROOT_PASSWORD" db \
  mariadb -uroot oteryn_s3a -e 'LOCK TABLES identities WRITE; DO SLEEP(8); UNLOCK TABLES' >/dev/null &
producer_locker=$!
sleep 0.2
producer_request() {
  local payload=$1 output=$2 status_output=$3
  curl "${curl_base[@]}" "${auth[@]}" -H 'Content-Type: application/json' \
    -o "$output" -w '%{http_code}' --data-binary "$payload" "$base_url" > "$status_output"
}
producer_request "$account_payload" "$WP5_SCRATCH/producer-first-response" "$WP5_SCRATCH/producer-first-status" &
producer_first=$!
producer_request "$capacity_account_payload" "$WP5_SCRATCH/producer-second-response" "$WP5_SCRATCH/producer-second-status" &
producer_second=$!
producer_slots_locked=false
for _ in {1..50}; do
  if compose exec --no-TTY --user www-data platform php -r \
    '$d="/var/lib/oteryn-witness"; foreach ([0,1] as $s) { $h=fopen("$d/pipeline-$s.lock","c+b"); if ($h===false || flock($h,LOCK_EX|LOCK_NB)) { if (is_resource($h)) { flock($h,LOCK_UN); fclose($h); } exit(1); } fclose($h); }' >/dev/null; then
    producer_slots_locked=true
    break
  fi
  sleep 0.1
done
[[ "$producer_slots_locked" == true ]]
third_status="$(curl "${curl_base[@]}" "${auth[@]}" -H 'Content-Type: application/json' \
  -o "$WP5_SCRATCH/producer-third-response" -w '%{http_code} %{time_total}' \
  --data-binary "$account_payload" "$base_url")"
read -r third_http third_seconds <<< "$third_status"
[[ "$third_http" == 200 ]]
expect_exact_unavailable_body "$WP5_SCRATCH/producer-third-response" ReadAccountSecurityV1 1
python3 - "$third_seconds" <<'PY'
import sys
if float(sys.argv[1]) >= 2.0:
    raise SystemExit("producer capacity rejection was not immediate and bounded")
PY
wait "$producer_locker"
wait "$producer_first" "$producer_second"
[[ "$(cat "$WP5_SCRATCH/producer-first-status")" == 200 ]]
[[ "$(cat "$WP5_SCRATCH/producer-second-status")" == 200 ]]
validate_observed_account "$WP5_SCRATCH/producer-first-response" \
  ReadAccountSecurityV1 1 "$ACCOUNT_ID" platform_security fresh_admission
validate_observed_account "$WP5_SCRATCH/producer-second-response" \
  ReadRecoveryAccountSecurityV2 2 "$CAPACITY_ACCOUNT_ID" platform_security existing_actor_recovery
evidence "producer_capacity=two_inflight third=immediate_unavailable application_queue=none rejection_seconds=$third_seconds"

# Relational outage is bounded and does not escape the closed failure shape.
pre_db_fault_revision="$(observed_revision "$WP5_SCRATCH/producer-first-response")"
compose stop db >/dev/null
expect_unavailable "$account_payload" "$WP5_SCRATCH/db-down-response" ReadAccountSecurityV1 1
compose start db >/dev/null
until compose exec --no-TTY -e MYSQL_PWD="$WP5_DB_ROOT_PASSWORD" db mariadb-admin ping -h 127.0.0.1 -uroot --silent >/dev/null 2>&1; do sleep 1; done
expect_observed_account "$account_payload" "$WP5_SCRATCH/db-recovered-response" \
  ReadAccountSecurityV1 1 "$ACCOUNT_ID" platform_security fresh_admission \
  "$pre_db_fault_revision" greater
db_recovered_revision="$(observed_revision "$WP5_SCRATCH/db-recovered-response")"
evidence "relational_failure=bounded_unavailable recovery_revision=${pre_db_fault_revision}->${db_recovered_revision}"

store_digest_before="$(compose exec --no-TTY platform sh -c 'sha256sum /var/lib/oteryn-witness/witness-store.id' | awk '{print $1}')"
compose restart platform nginx >/dev/null
compose up --detach --wait platform nginx >/dev/null
expect_observed_account "$account_payload" "$WP5_SCRATCH/restart-response" \
  ReadAccountSecurityV1 1 "$ACCOUNT_ID" platform_security fresh_admission \
  "$db_recovered_revision" greater
restart_revision="$(observed_revision "$WP5_SCRATCH/restart-response")"
store_digest_after="$(compose exec --no-TTY platform sh -c 'sha256sum /var/lib/oteryn-witness/witness-store.id' | awk '{print $1}')"
[[ "$store_digest_before" == "$store_digest_after" ]]
evidence "process_restart=retained witness_store_digest=$store_digest_after revision=${db_recovered_revision}->${restart_revision}"

# Restore a synthetic database snapshot with only the witness-store binding omitted.
compose exec --no-TTY -e MYSQL_PWD="$WP5_DB_ROOT_PASSWORD" db mariadb-dump -uroot \
  --single-transaction --skip-comments --skip-dump-date oteryn_s3a > "$WP5_SCRATCH/database.sql"
awk '
  /^INSERT INTO `native_game_evidence_witness_stores`/ { omitting = 1 }
  !omitting { print }
  omitting && /;$/ { omitting = 0 }
' "$WP5_SCRATCH/database.sql" > "$WP5_SCRATCH/database-no-binding.sql"
grep -q '^CREATE TABLE `native_game_evidence_witness_stores`' "$WP5_SCRATCH/database-no-binding.sql"
! grep -q '^INSERT INTO `native_game_evidence_witness_stores`' "$WP5_SCRATCH/database-no-binding.sql"
compose stop nginx platform >/dev/null
compose exec --no-TTY -e MYSQL_PWD="$WP5_DB_ROOT_PASSWORD" db mariadb -uroot \
  -e 'DROP DATABASE oteryn_s3a; CREATE DATABASE oteryn_s3a CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci; GRANT ALL ON oteryn_s3a.* TO "oteryn_s3a"@"%"' >/dev/null
compose exec --no-TTY -e MYSQL_PWD="$WP5_DB_ROOT_PASSWORD" db mariadb -uroot oteryn_s3a < "$WP5_SCRATCH/database-no-binding.sql"
compose start platform >/dev/null
compose up --detach --wait platform nginx >/dev/null
expect_observed_account "$account_payload" "$WP5_SCRATCH/restore-response" \
  ReadAccountSecurityV1 1 "$ACCOUNT_ID" platform_security fresh_admission \
  "$restart_revision" greater
restore_revision="$(observed_revision "$WP5_SCRATCH/restore-response")"
rebound="$(compose exec --no-TTY -e MYSQL_PWD="$WP5_DB_ROOT_PASSWORD" db mariadb -N -uroot oteryn_s3a -e 'SELECT COUNT(*) FROM native_game_evidence_witness_stores WHERE id=1')"
[[ "$rebound" == 1 ]]
evidence "database_restore=retained_witness_rebound revision=${restart_revision}->${restore_revision}"

# Empty replacement witness is rejected while relational history remains; restore retained files afterward.
compose exec --no-TTY platform sh -c 'tar -C /var/lib/oteryn-witness -cf /run/wp5/witness.tar . && find /var/lib/oteryn-witness -mindepth 1 -maxdepth 1 -delete'
expect_unavailable "$account_payload" "$WP5_SCRATCH/replacement-response" ReadAccountSecurityV1 1
compose exec --no-TTY platform sh -c 'tar -C /var/lib/oteryn-witness -xf /run/wp5/witness.tar'
expect_observed_account "$account_payload" "$WP5_SCRATCH/replacement-restored-response" \
  ReadAccountSecurityV1 1 "$ACCOUNT_ID" platform_security fresh_admission \
  "$restore_revision" greater
replacement_restored_revision="$(observed_revision "$WP5_SCRATCH/replacement-restored-response")"
evidence "replacement_witness=rejected retained_witness=restored revision=${restore_revision}->${replacement_restored_revision}"

# Deterministically prove that the recovery guard rejects an otherwise-valid
# response whose revision and decision identity regress below the pre-fault high-water.
python3 - "$WP5_SCRATCH/replacement-restored-response" "$WP5_SCRATCH/regressed-response" "$replacement_restored_revision" <<'PY'
import json
import sys
source, destination, floor = sys.argv[1:]
response = json.load(open(source))
response["source_revision"] = str(int(floor) - 1)
response["decision_identity"] = response["source_revision"]
with open(destination, "w") as output:
    json.dump(response, output, separators=(",", ":"))
PY
if validate_observed_account "$WP5_SCRATCH/regressed-response" \
  ReadAccountSecurityV1 1 "$ACCOUNT_ID" platform_security fresh_admission \
  "$replacement_restored_revision" at_least 2>/dev/null; then
  echo 'regressed recovery revision was accepted' >&2
  exit 1
fi
evidence "revision_regression_self_test=rejected floor=$replacement_restored_revision"

# Deterministically prove that a type-confused or out-of-range body (valid
# except for "allowed":1, "version":true, a source_revision above u64 or a
# source_observed_at above i64) is rejected, as the exact Game decoder does.
for mutation in allowed version revision_overflow observed_at_overflow; do
  python3 - "$WP5_SCRATCH/replacement-restored-response" "$WP5_SCRATCH/type-confused-response" "$mutation" <<'PY'
import json
import sys
source, destination, mutation = sys.argv[1:]
response = json.load(open(source))
if mutation == "allowed":
    response["allowed"] = 1
elif mutation == "version":
    response["version"] = True
elif mutation == "revision_overflow":
    # One above u64::MAX; also exceeds every recovery floor.
    response["source_revision"] = str(2**64)
    response["decision_identity"] = response["source_revision"]
else:
    # One above i64::MAX.
    response["source_observed_at"] = str(2**63)
with open(destination, "w") as output:
    json.dump(response, output, separators=(",", ":"))
PY
  if validate_observed_account "$WP5_SCRATCH/type-confused-response" \
    ReadAccountSecurityV1 1 "$ACCOUNT_ID" platform_security fresh_admission \
    "$replacement_restored_revision" at_least 2>/dev/null; then
    echo "type-confused recovery body ($mutation) was accepted" >&2
    exit 1
  fi
done
evidence "json_type_self_test=rejected allowed_int=true version_bool=true u64_overflow=true i64_overflow=true"

# Account witness-ahead rollback and explicit forward-only reconciliation.
observed_generation() {
  python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["minimum_valid_generation"])' "$1"
}
account_generation_before="$(observed_generation "$WP5_SCRATCH/replacement-restored-response")"
php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); $id=App\Identity\Models\Identity::query()->where("account_id","'"$ACCOUNT_ID"'")->firstOrFail(); Illuminate\Support\Facades\DB::beginTransaction(); try { app(App\Identity\Actions\RevokeIdentityGameAuthorizations::class)->execute($id); } finally { Illuminate\Support\Facades\DB::rollBack(); }'
expect_unavailable "$account_payload" "$WP5_SCRATCH/account-ambiguous-response" ReadAccountSecurityV1 1
account_ambiguous_store_digest="$(compose exec --no-TTY platform sh -c 'sha256sum /var/lib/oteryn-witness/witness-store.id' | awk '{print $1}')"
compose restart platform nginx >/dev/null
compose up --detach --wait platform nginx >/dev/null
expect_unavailable "$account_payload" "$WP5_SCRATCH/account-ambiguous-after-restart-response" ReadAccountSecurityV1 1
[[ "$(compose exec --no-TTY platform sh -c 'sha256sum /var/lib/oteryn-witness/witness-store.id' | awk '{print $1}')" == "$account_ambiguous_store_digest" ]]
compose exec --no-TTY --user www-data platform php artisan game-auth:native-evidence:reconcile --account-id="$ACCOUNT_ID" --no-interaction >/dev/null
expect_observed_account "$account_payload" "$WP5_SCRATCH/account-reconciled-response" \
  ReadAccountSecurityV1 1 "$ACCOUNT_ID" platform_security fresh_admission \
  "$replacement_restored_revision" greater
# The rolled-back revocation advanced only the witness; reconciliation must move
# canonical state forward to it (never roll the witness back to the database).
account_generation_after="$(observed_generation "$WP5_SCRATCH/account-reconciled-response")"
account_reconciled_revision="$(observed_revision "$WP5_SCRATCH/account-reconciled-response")"
account_generation_database="$(compose exec --no-TTY -e MYSQL_PWD="$WP5_DB_ROOT_PASSWORD" db mariadb -N -uroot oteryn_s3a -e "SELECT native_security_generation FROM identities WHERE account_id='$ACCOUNT_ID'")"
(( account_generation_after == account_generation_before + 1 ))
[[ "$account_generation_database" == "$account_generation_after" ]]
evidence "account_rollback=unavailable_after_restart witness_ahead=retained account_reconcile=forward_only generation=${account_generation_before}->${account_generation_after} database_generation=$account_generation_database revision=${replacement_restored_revision}->${account_reconciled_revision}"

# Signing witness-ahead rollback, conservative revocation, then a fresh successor key/profile.
expect_observed_trust "$fresh_trust_payload" "$WP5_SCRATCH/trust-before-fault-response" \
  true "$FRESH_KEY_BYTE" 0 positive
trust_before_fault_revision="$(observed_revision "$WP5_SCRATCH/trust-before-fault-response")"
php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); $r=app(App\GameAuth\NativeEvidence\NativeSigningTrustRegistry::class); Illuminate\Support\Facades\DB::beginTransaction(); try { $r->revokeProfile(App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_ISSUER,App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_PROFILE,"fresh_admission"); } finally { Illuminate\Support\Facades\DB::rollBack(); }'
expect_unavailable "$fresh_trust_payload" "$WP5_SCRATCH/trust-ambiguous-response" ReadFreshSigningTrustV1 1
trust_ambiguous_store_digest="$(compose exec --no-TTY platform sh -c 'sha256sum /var/lib/oteryn-witness/witness-store.id' | awk '{print $1}')"
compose restart platform nginx >/dev/null
compose up --detach --wait platform nginx >/dev/null
expect_unavailable "$fresh_trust_payload" "$WP5_SCRATCH/trust-ambiguous-after-restart-response" ReadFreshSigningTrustV1 1
[[ "$(compose exec --no-TTY platform sh -c 'sha256sum /var/lib/oteryn-witness/witness-store.id' | awk '{print $1}')" == "$trust_ambiguous_store_digest" ]]
compose exec --no-TTY --user www-data platform php artisan game-auth:native-evidence:reconcile --trust=fresh --no-interaction >/dev/null
# Before any successor exists, the reconciled old key must be observed as
# untrusted with its exact seeded key material through the real Game decoder.
expect_observed_trust "$fresh_trust_payload" "$WP5_SCRATCH/trust-reconciled-response" \
  false "$FRESH_KEY_BYTE" "$trust_before_fault_revision" greater
trust_reconciled_revision="$(observed_revision "$WP5_SCRATCH/trust-reconciled-response")"
cargo +1.94.0 test --locked -p oteryn-game-server --test native_admission_source_real_interop real_platform_producer_reports_revoked_fresh_key -- --ignored --exact --nocapture
php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); app(App\GameAuth\NativeEvidence\NativeSigningTrustRegistry::class)->publishNextProfileVersion(App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_ISSUER,App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_PROFILE,"fresh_admission","fresh-key-2",str_repeat(chr('"$SUCCESSOR_FRESH_KEY_BYTE"'),32));'
WP5_S3A_FRESH_KEY_ID=fresh-key-2 WP5_S3A_FRESH_KEY_BYTE="$SUCCESSOR_FRESH_KEY_BYTE" cargo +1.94.0 test --locked -p oteryn-game-server --test native_admission_source_real_interop real_platform_producer_decodes_all_four_operations -- --ignored --exact --nocapture
evidence "trust_rollback=unavailable_after_restart witness_ahead=retained trust_reconcile=revoked revision=${trust_before_fault_revision}->${trust_reconciled_revision} successor_profile=fresh_key"

# The path-scoped interposer runs in the exact PHP ABI and distinguishes file and directory fsync.
for mode in file directory; do
  # Intervening harness reads advance the producer revision, so bind the floor
  # to an authoritative observation taken immediately before this fault.
  expect_observed_account "$account_payload" "$WP5_SCRATCH/fsync-$mode-pre-fault-response" \
    ReadAccountSecurityV1 1 "$ACCOUNT_ID" platform_security fresh_admission \
    "$account_reconciled_revision" greater
  fsync_pre_fault_revision="$(observed_revision "$WP5_SCRATCH/fsync-$mode-pre-fault-response")"
  WP5_FSYNC_FAULT="$mode"; export WP5_FSYNC_FAULT
  compose up --detach --wait --force-recreate --no-deps platform >/dev/null
  expect_unavailable "$account_payload" "$WP5_SCRATCH/fsync-$mode-response" ReadAccountSecurityV1 1
  WP5_FSYNC_FAULT=none; export WP5_FSYNC_FAULT
  compose up --detach --wait --force-recreate --no-deps platform >/dev/null
  expect_observed_account "$account_payload" "$WP5_SCRATCH/fsync-$mode-recovered-response" \
    ReadAccountSecurityV1 1 "$ACCOUNT_ID" platform_security fresh_admission \
    "$fsync_pre_fault_revision" greater
  account_reconciled_revision="$(observed_revision "$WP5_SCRATCH/fsync-$mode-recovered-response")"
  evidence "fsync_fault=$mode result=unavailable recovery=observed revision=${fsync_pre_fault_revision}->${account_reconciled_revision}"
done

history="$(compose exec --no-TTY -e MYSQL_PWD="$WP5_DB_ROOT_PASSWORD" db mariadb -N -uroot oteryn_s3a -e 'SELECT COUNT(*) FROM native_game_evidence_observations')"
floors="$(compose exec --no-TTY platform sh -c 'find /var/lib/oteryn-witness -maxdepth 1 -type f -name "*.floor" | wc -l')"
(( history > 0 && floors > 0 ))
compose down --volumes --remove-orphans --timeout 15 >/dev/null
cleaned=true
evidence 'simultaneous_database_and_witness_loss=BLOCKED qualified_restore=false authority_recovery_not_attempted=true'
result=COMPOSED_PASS
evidence "result=COMPOSED_PASS game=$(git rev-parse HEAD) platform=$PLATFORM_SHA topology=$TOPOLOGY_REVISION"
