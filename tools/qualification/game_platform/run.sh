#!/usr/bin/env bash
# Separately pinned portal-rs identity lane; never delegates to the PHP topology.
set -Eeuo pipefail
set +x
umask 077
readonly GAME_PLATFORM_SHA=f73244933228f399f1a5e9174ef064d7ffe70785
readonly GAME_SOURCE="$(git rev-parse --show-toplevel)"
fail() { printf 'GAME_PLATFORM_RESULT=FAIL flow=preflight reason=%s\n' "$1" > "$GAME_PLATFORM_RESULT_FILE"; exit 1; }
scratch="$(mktemp -d "${TMPDIR:-/tmp}/game-platform.XXXXXX")"
export GAME_PLATFORM_RESULT_FILE="$scratch/result"
export NODE_BOOT_FAILURE_LOG_DIR="$scratch/diagnostics"
mkdir -m 700 "$NODE_BOOT_FAILURE_LOG_DIR"
: > "$scratch/node-boot.log"
cleanup() {
  local code=$?
  trap - EXIT
  if [[ -f "$GAME_PLATFORM_RESULT_FILE" ]]; then
    cat "$GAME_PLATFORM_RESULT_FILE"
  else
    printf 'GAME_PLATFORM_RESULT=FAIL flow=lifecycle reason=node_boot_exit_%s\n' "$code"
  fi
  if (( code != 0 )); then
    local diagnostics
    diagnostics="$(mktemp -d "${TMPDIR:-/tmp}/game-platform-diagnostics.XXXXXX")"
    chmod 700 "$diagnostics"
    cp "$scratch/node-boot.log" "$diagnostics/node-boot.log"
    cp -R "$scratch/diagnostics/." "$diagnostics/"
    printf 'GAME_PLATFORM_DIAGNOSTICS=%s\n' "$diagnostics"
  fi
  rm -rf -- "$scratch"
}
trap cleanup EXIT
[[ -n "${PLATFORM_SOURCE:-}" ]] || fail platform_source_missing
[[ "$(git -C "$PLATFORM_SOURCE" rev-parse HEAD 2>/dev/null)" == "$GAME_PLATFORM_SHA" ]] || fail wrong_platform_sha
for file in portal-rs/e2e/start.sh portal-rs/e2e/harness.mjs \
  portal-rs/crates/identity/src/login_http.rs portal-rs/crates/gameauth/src/oauth/oauth_http.rs; do
  if [[ ! -f "$PLATFORM_SOURCE/$file" ]]; then
    printf 'GAME_PLATFORM_RESULT=BLOCKED owner=Platform what=%s:1\n' "$file"
    exit 2
  fi
done
[[ -z "$(git -C "$PLATFORM_SOURCE" status --porcelain --untracked-files=no)" ]] || fail modified_platform_source
printf 'GAME_PLATFORM_EVIDENCE pins game=%s platform=%s\n' "$(git rev-parse HEAD)" "$GAME_PLATFORM_SHA"
for program in docker python3 node openssl cargo sudo psql setsid timeout; do
  command -v "$program" >/dev/null || fail "missing_binary_$program"
done
docker info >/dev/null 2>&1 || fail docker_unavailable
# node_boot uses the same shipping binaries, Game TLS PostgreSQL and World bundle.
# This identity pass deliberately never assigns/admit Characters. The missing
# T19c route seed is reported after identity, never replaced with a manual route.
export NODE_BOOT_LIFECYCLE=game_platform NODE_BOOT_SKIP_BASIC=1 NODE_BOOT_WORLD=1
export NODE_BOOT_KEEP_STACK=0 NODE_BOOT_REUSE_STACK=0 OTERYN_NODE_BOOT_REAL_CLOCK=1
export NODE_BOOT_PROJECT="gameplatform${BASHPID}" NODE_BOOT_SERVICE_USER="oteryn-node-boot-gpe2e"
export NODE_BOOT_BASE="/srv/oteryn-game-platform-${BASHPID}"
export NODE_BOOT_PG_PORT="${GAME_PLATFORM_GAME_PG_PORT:-15435}"
export NODE_BOOT_GAME_PORT="${GAME_PLATFORM_GAME_PORT:-17183}"
# Keep raw diagnostics private; only the closed evidence vocabulary is published.
set +e
bash "$GAME_SOURCE/tools/qualification/node_boot/run.sh" > "$scratch/node-boot.log" 2>&1
code=$?
set -e
sed -n -E '/^(NODE_BOOT_EVIDENCE |GAME_PLATFORM_EVIDENCE )/p' "$scratch/node-boot.log"
exit "$code"
