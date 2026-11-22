#!/usr/bin/env bash
# NATIVE_ENTRY_ROOM_CI_ISSUED_WORLD_V1 (#822 path, owner decision in #162): the native entry room
# is bound to a canonical WorldId issued for this run by the owning Platform World Registry
# disposable issuer (Oteryn-Platform #1417). Nothing here is production, deployment or custody
# evidence; the SQLite fixture is created and destroyed by this runner.
set -euo pipefail

readonly PLATFORM_SHA=eb65ce453949a352fdc0e3ed0132d2cde0084fb7
readonly REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
readonly PLATFORM_SOURCE="${PLATFORM_SOURCE:-$REPO_ROOT/_platform}"
readonly IMAGE=oteryn-native-entry-room-platform:local

if [[ "$(git -C "$PLATFORM_SOURCE" rev-parse HEAD 2>/dev/null)" != "$PLATFORM_SHA" ]]; then
  echo "NATIVE_ENTRY_ROOM_RESULT=BLOCKED reason=exact_platform_checkout_missing" >&2
  exit 1
fi

native_build_pid=""
native_build_root=""
cleanup_native_build() {
  if [[ -n "$native_build_pid" ]]; then
    kill -- "-$native_build_pid" 2>/dev/null || true
    wait "$native_build_pid" 2>/dev/null || true
  fi
  if [[ -n "$native_build_root" ]]; then
    rm -rf -- "${native_build_root:?}"
  fi
}
trap cleanup_native_build EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

if [[ -z "${OTERYN_NATIVE_ENTRY_PREBUILT+x}" && "${OTERYN_NATIVE_ENTRY_PARALLEL_BUILD:-0}" == 1 ]]; then
  # When no applicable shared producer exists, overlap the original focused Rust
  # compilation with the independent real Platform issuance. A process group
  # prevents an interrupted issuer from leaving Cargo/rustc descendants running.
  native_build_root="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/oteryn-native-entry.XXXXXX")"
  setsid python3 "$REPO_ROOT/tools/qualification/native_entry_room/prebuilt.py" \
    build "$native_build_root/bundle" &
  native_build_pid=$!
fi

docker build --quiet \
  -f "$REPO_ROOT/tools/qualification/wp5_s3a/platform-fpm.Dockerfile" \
  --build-context "platform=$PLATFORM_SOURCE" \
  -t "$IMAGE" "$REPO_ROOT/tools/qualification/wp5_s3a" >/dev/null

# The issuer accepts only a retained regular SQLite file inside
# sys_get_temp_dir()/oteryn-native-topology-<hex>/ under APP_ENV=preproduction.
fixture="oteryn-native-topology-$(od -An -N8 -tx1 /dev/urandom | tr -d ' \n')"
receipt="$(docker run --rm -e APP_ENV=preproduction -e DB_CONNECTION=sqlite \
  -e "DB_DATABASE=/tmp/$fixture/oteryn-native-topology.sqlite" \
  -e "APP_KEY=base64:$(head -c 32 /dev/urandom | base64)" \
  -e LOG_CHANNEL=stderr "$IMAGE" sh -ec "
    mkdir -m 700 /tmp/$fixture
    touch /tmp/$fixture/oteryn-native-topology.sqlite
    php artisan migrate --force --no-interaction >&2
    php -r '\$db = new PDO(\"sqlite:/tmp/$fixture/oteryn-native-topology.sqlite\");
      \$db->exec(\"INSERT INTO game_worlds (slug, name, region, game_host, game_port) VALUES (\\\"entry\\\", \\\"Entry\\\", \\\"local\\\", \\\"127.0.0.1\\\", 7171)\") === 1 || exit(1);'
    php artisan game-auth:native-topology:issue --world-row-id=1 --channel-key=entry-1
  " | tail -n 1)"

echo "native topology receipt: $receipt"
cd "$REPO_ROOT"
if [[ -n "$native_build_pid" ]]; then
  if ! wait "$native_build_pid"; then
    echo "NATIVE_ENTRY_ROOM_RESULT=BLOCKED reason=native_test_build_failed" >&2
    exit 1
  fi
  native_build_pid=""
  OTERYN_NATIVE_ENTRY_PREBUILT="$native_build_root/bundle"
fi
if [[ -n "${OTERYN_NATIVE_ENTRY_PREBUILT+x}" ]]; then
  native_entry_test="$(python3 tools/qualification/native_entry_room/prebuilt.py executable "$OTERYN_NATIVE_ENTRY_PREBUILT")"
  # Match Cargo's integration-test working directory. The real, per-run issuer
  # receipt and all three assertions (including the ignored issuance case) remain.
  cd "$REPO_ROOT/apps/game-server"
  OTERYN_NATIVE_ENTRY_TOPOLOGY_RECEIPT="$receipt" "$native_entry_test" --include-ignored
else
  OTERYN_NATIVE_ENTRY_TOPOLOGY_RECEIPT="$receipt" cargo +1.100.0 test --locked -p oteryn-game-server \
    --test content_native_entry_room -- --include-ignored
fi
echo "NATIVE_ENTRY_ROOM_RESULT=PASS"
