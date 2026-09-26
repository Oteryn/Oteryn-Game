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
OTERYN_NATIVE_ENTRY_TOPOLOGY_RECEIPT="$receipt" cargo test --locked -p oteryn-game-server \
  --test content_native_entry_room -- --include-ignored
echo "NATIVE_ENTRY_ROOM_RESULT=PASS"
