#!/usr/bin/env bash
# Reuse node_boot's pinned PostgreSQL, Platform, node and real-bundle lifecycle.
# This command is deliberately red while SCENARIO_MATRIX.md has coverage gaps.
set -uo pipefail
umask 077

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd -- "$ROOT" || exit 1
RUN="$(mktemp -d "$ROOT/.e2e-all.XXXXXX")" || exit 1
mkdir "$RUN/captures" || exit 1
export E2E_ALL_CAPTURE_DIR="$RUN/captures"
readonly CARGO="${CARGO:-cargo}"
node_status=0
client_status=0

# Keep the fixture pass that node_boot composes before its real-world pass.
# Neither database bootstrap nor node lifecycle is duplicated here.
NODE_BOOT_WORLD=1 NODE_BOOT_SPELLS=0 \
    bash tools/qualification/node_boot/run.sh 2>&1 | tee "$RUN/node.log" || node_status=$?

# Missing captures are an error. A failed boot cannot silently use unit fixtures.
"$CARGO" test --offline --locked -p oteryn-client-runtime -p oteryn-client-domain -p oteryn-client-ui \
    -p oteryn-client -p oteryn-session -p oteryn-dev-client --lib --quiet 2>&1 | tee "$RUN/client.log" || client_status=$?

python3 tools/qualification/e2e_all/report.py \
    --matrix apps/game-server/src/qualification/SCENARIO_MATRIX.md \
    --log "$RUN/node.log" --node-status "$node_status" --client-status "$client_status"
result=$?
printf 'Evidence: %s\n' "$RUN"
exit "$result"
