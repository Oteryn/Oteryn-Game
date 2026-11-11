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
# Collect every registered batch before running capture-dependent replay tests.
# A partial node-boot batch cannot supply the evidence required by the full matrix.
row_count="$(python3 - <<'PYROWS'
from pathlib import Path
import re
source = Path("apps/game-server/src/qualification/client_phases/world.rs").read_text()
print(max(map(int, re.findall(r"e2e_row_selected\((\d+)\)", source))) + 1)
PYROWS
)" || exit 1
for ((start = 0; start < row_count; start += 15)); do
    OTERYN_E2E_ROW_BATCH="$start:15" NODE_BOOT_WORLD=1 NODE_BOOT_SPELLS=0 \
        bash tools/qualification/node_boot/run.sh 2>&1 | tee -a "$RUN/node.log" || node_status=$?
done

# Missing captures are an error. A failed boot cannot silently use unit fixtures.
"$CARGO" test --offline --locked -p oteryn-client-runtime -p oteryn-client-domain -p oteryn-client-ui \
    -p oteryn-client -p oteryn-session -p oteryn-dev-client --lib --quiet 2>&1 | tee "$RUN/client.log" || client_status=$?

python3 tools/qualification/e2e_all/report.py \
    --matrix apps/game-server/src/qualification/SCENARIO_MATRIX.md \
    --log "$RUN/node.log" --node-status "$node_status" --client-status "$client_status"
result=$?
printf 'Evidence: %s\n' "$RUN"
exit "$result"
