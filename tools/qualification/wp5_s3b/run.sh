#!/usr/bin/env bash
# WP5 S3-B: sealed composition of the real protected owners.
#
# Reuses the node-boot Platform lifecycle for its mTLS producer and real
# Character bootstrap-intent issuer. The caller supplies Game's PostgreSQL.
# Both Characters are bootstrapped from intents issued by the real Platform
# operator command immediately before the qualification run.
# The fresh grant is signed with an ephemeral Ed25519 key generated here; only
# its public half is published into the Platform trust registry, and the Game
# consumes that trust through the real S1 transport and S2 durable floor.
set -Eeuo pipefail
umask 077

readonly PLATFORM_SHA=6d04b52a16145723eedb0c56640ad052dbd26e94
readonly TOPOLOGY_REVISION=portal-rs-game-platform-v1
readonly ACCOUNT_ID=01934f10-7c00-7000-8000-000000000001
readonly SECOND_ACCOUNT_ID=01934f10-7c00-7000-8000-000000000002
readonly FRESH_KEY_ID=s3b-fresh-key-1
readonly RECOVERY_KEY_ID=s3b-recovery-key-1
readonly PG_IMAGE=postgres:18.6-bookworm@sha256:bbf83594798e144bff365a9c191c5d779f181fb3135902064ab8ff11918020fc
# Must equal the harness constants (v7 identities and INTERPRETATION).
readonly WORLD_ID=01934f10-7c02-7001-805b-3b1122334401
readonly CHANNEL_ID=01934f10-7c03-7001-805b-3b1122334401
readonly INTENT_OPERATIONS=(01934f10-7c04-7001-805b-3b1122334401 01934f10-7c04-7002-805b-3b1122334402)
readonly INTENT_NAMES=("Qualification One" "Qualification Two")
readonly INTERPRETATION=(s3b-profile-1 s3b-ruleset-1 s3b-content-1 s3b-starter-1)

# WP5_QUALIFICATION selects the harness run inside the same topology:
# `s3b` is the sealed composition, `seam` the entry room, `spell-seam` full spell content.
case "${WP5_QUALIFICATION:-s3b}" in
  s3b)
    TEST_TARGET=(--test wp5_s3b_composition)
    TEST_NAME=real_owners_compose_fresh_admission_and_fence_replacement
    PASS_RESULT=COMPOSED_PASS
    ;;
  seam)
    TEST_TARGET=(--lib)
    TEST_NAME=qualification::server_seam_real_owners_over_tcp_tls
    PASS_RESULT=SEAM_PASS
    ;;
  spell-seam)
    TEST_TARGET=(--lib)
    TEST_NAME=qualification::server_seam_real_owners_over_tcp_tls
    PASS_RESULT=SPELL_SEAM_PASS
    ;;
  *)
    echo 'S3B_RESULT=BLOCKED reason=unknown_qualification'
    exit 2
    ;;
esac

GAME_SOURCE="$(git rev-parse --show-toplevel)"
if [[ "${WP5_QUALIFICATION:-s3b}" == spell-seam ]]; then
  OTERYN_SEAM_SPELL_MANIFEST="${OTERYN_SEAM_SPELL_MANIFEST:-$GAME_SOURCE/docs/reference/spells/r21-local-candidate/active-artifact/manifest.json}"
  OTERYN_SEAM_SPELL_MANIFEST="$(realpath -e -- "$OTERYN_SEAM_SPELL_MANIFEST" 2>/dev/null)" || { echo 'S3B_RESULT=BLOCKED reason=spell_manifest_missing'; exit 2; }
  [[ -f "$OTERYN_SEAM_SPELL_MANIFEST" ]] || { echo 'S3B_RESULT=BLOCKED reason=spell_manifest_missing'; exit 2; }
  export OTERYN_SEAM_SPELL_MANIFEST
  # Existing composed-content test stack; production stack remains unchanged.
  export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"
fi
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
  echo 'S3B_RESULT=BLOCKED reason=postgres_18_6_service_missing'
  exit 2
fi

# shellcheck source=tools/qualification/game_platform/lifecycle.sh
source "$GAME_SOURCE/tools/qualification/game_platform/lifecycle.sh"
PG_PORT="$(python3 -c 'import os, urllib.parse; print(urllib.parse.urlsplit(os.environ["OTERYN_TEST_POSTGRES_ADMIN_URL"]).port or 5432)')"

WORK="$(mktemp -d "${RUNNER_TEMP:-/tmp}/wp5-s3b.XXXXXX")"
WP5_PKI="$WORK/pki"
mkdir -p "$WP5_PKI"
WP5_PROJECT="wp5s3b${GITHUB_RUN_ID:-local}${GITHUB_RUN_ATTEMPT:-1}"
# Platform seeds this disposable Game route. Game's own database is external.
export NODE_BOOT_GAME_PORT="${NODE_BOOT_GAME_PORT:-17181}"
export GAME_PLATFORM_INTERNAL_PORT="${WP5_S3B_PORT:-18453}"

result=COMPOSED_FAIL
cleaned=false
evidence() { printf 'S3B_EVIDENCE %s\n' "$*"; }
cleanup() {
  local rc=$?
  if [[ "$cleaned" != true ]]; then
    game_platform_cleanup || true
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

# Ephemeral fresh-admission signing key; the private seed never leaves $WORK
# except through the environment of the single qualification test process.
openssl genpkey -algorithm ed25519 -out "$WP5_PKI/fresh-signing.pem" >/dev/null 2>&1
raw_hex() { tail -c 32 | od -An -v -tx1 | tr -d ' \n'; }
FRESH_PUBLIC_HEX="$(openssl pkey -in "$WP5_PKI/fresh-signing.pem" -pubout -outform DER | raw_hex)"
FRESH_SEED_HEX="$(openssl pkey -in "$WP5_PKI/fresh-signing.pem" -outform DER | raw_hex)"
[[ ${#FRESH_PUBLIC_HEX} == 64 && ${#FRESH_SEED_HEX} == 64 ]]
# Separate ephemeral Recovery key; fixed issuer/profile/purpose is not Fresh.
openssl genpkey -algorithm ed25519 -out "$WP5_PKI/recovery-signing.pem" >/dev/null 2>&1
RECOVERY_PUBLIC_HEX="$(openssl pkey -in "$WP5_PKI/recovery-signing.pem" -pubout -outform DER | raw_hex)"
RECOVERY_SEED_HEX="$(openssl pkey -in "$WP5_PKI/recovery-signing.pem" -outform DER | raw_hex)"
[[ ${#RECOVERY_PUBLIC_HEX} == 64 && ${#RECOVERY_SEED_HEX} == 64 ]]
evidence "pins platform=$PLATFORM_SHA topology=$TOPOLOGY_REVISION game=$(git rev-parse HEAD)"
if [[ "${WP5_QUALIFICATION:-s3b}" == spell-seam ]]; then
  evidence "spell_input=full_manifest manifest_sha256=$(sha256sum "$OTERYN_SEAM_SPELL_MANIFEST" | cut -d ' ' -f 1)"
fi

# The early lib-only producer runs independently of the Node boot binary matrix
# and the Platform producer. Local runs retain their ordinary Cargo path.
prepare_seam_test() {
  SEAM_TEST_CWD="$GAME_SOURCE"
  SEAM_TEST_COMMAND=(cargo +1.100.0 test --locked -p oteryn-game-server "${TEST_TARGET[@]}"
    "$TEST_NAME" -- --ignored --exact --nocapture)
  if [[ -n "${SERVER_SEAM_PREBUILT_DIR:-}" ]]; then
    [[ "${WP5_QUALIFICATION:-s3b}" == seam || "${WP5_QUALIFICATION:-s3b}" == spell-seam ]] \
      || { echo 'S3B_RESULT=BLOCKED reason=prebuilt_variant_mismatch'; return 2; }
    python3 "$GAME_SOURCE/tools/qualification/node_boot/prebuilt.py" verify server-seam "$SERVER_SEAM_PREBUILT_DIR" \
      || { echo 'S3B_RESULT=BLOCKED reason=invalid_prebuilt_seam'; return 2; }
    SEAM_TEST_CWD="$GAME_SOURCE/apps/game-server"
    SEAM_TEST_COMMAND=("$SERVER_SEAM_PREBUILT_DIR/server-seam/node-boot-libtest"
      "$TEST_NAME" --ignored --exact --nocapture)
  else
    # Compile before services and short-lived bootstrap intents are created.
    cargo +1.100.0 test --locked -p oteryn-game-server "${TEST_TARGET[@]}" --no-run
  fi
}
prepare_seam_test

game_platform_start
evidence "platform_seed=synthetic_accounts=2 fresh_trust=published_public_key_only"
evidence "recovery_trust=published_public_key_only purpose=existing_actor_recovery"

export WP5_S3A_PORT="$WP5_PORT"
export WP5_S3A_SERVER_NAME="$WP5_SERVER_NAME"
export WP5_S3A_SOURCE_AUTHORITY="$WP5_SOURCE_AUTHORITY"
export WP5_S3A_ROUTE_REVISION="$WP5_ROUTE_REVISION"
export WP5_S3A_CA_CERT="$WP5_PKI/server-ca.crt"
export WP5_S3A_CLIENT_CERT="$WP5_PKI/client.crt"
export WP5_S3A_CLIENT_KEY="$WP5_PKI/client.key"
export WP5_S3A_ACCOUNT_ID="$ACCOUNT_ID"
export WP5_S3B_SECOND_ACCOUNT_ID="$SECOND_ACCOUNT_ID"
export WP5_S3B_FRESH_KEY_ID="$FRESH_KEY_ID"
export WP5_S3B_FRESH_PUBLIC_KEY="$FRESH_PUBLIC_HEX"
export WP5_S3B_RECOVERY_KEY_ID="$RECOVERY_KEY_ID"
export WP5_S3B_RECOVERY_PUBLIC_KEY="$RECOVERY_PUBLIC_HEX"
WP5_S3B_INTENT_CLIENT_CERT="$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["clients"]["bootstrap-intents"]["cert"])' "$WORK/portal-report.json")"
WP5_S3B_INTENT_CLIENT_KEY="$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["clients"]["bootstrap-intents"]["key"])' "$WORK/portal-report.json")"
export WP5_S3B_INTENT_CLIENT_CERT WP5_S3B_INTENT_CLIENT_KEY
export WP5_S3B_CHARACTER_FENCE_DIR="$WORK/character-fence"
mkdir -p "$WP5_S3B_CHARACTER_FENCE_DIR"

# Real Platform operator command: one bootstrap intent per synthetic account.
index=0
for account in "$ACCOUNT_ID" "$SECOND_ACCOUNT_ID"; do
  operation="${INTENT_OPERATIONS[$index]}"
  name="${INTENT_NAMES[$index]}"
  python3 "$GAME_SOURCE/tools/qualification/game_platform/composition.py" bootstrap \
    "$WORK" "$account" "$WORLD_ID" "$operation" "${INTERPRETATION[@]}" "$name" Sorcerer
  index=$((index + 1))
done
evidence "platform_intents=issued count=2 path=operator_command ttl_seconds=300"
(
  cd "$SEAM_TEST_CWD"
  WP5_S3B_FRESH_KEY_SEED="$FRESH_SEED_HEX" \
  WP5_S3B_RECOVERY_KEY_SEED="$RECOVERY_SEED_HEX" \
    "${SEAM_TEST_COMMAND[@]}"
)
unset FRESH_SEED_HEX RECOVERY_SEED_HEX
result="$PASS_RESULT"
