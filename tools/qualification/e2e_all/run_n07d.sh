#!/usr/bin/env bash
# Exercise N07d against fresh captures while the client replay lacks live envelopes.
set -euo pipefail
umask 077

cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.."
run_dir=$(mktemp -d /tmp/qualification-n07d.XXXXXX)
export E2E_ALL_CAPTURE_DIR="$run_dir/captures"
mkdir -- "$E2E_ALL_CAPTURE_DIR"
printf 'N07d evidence is in %s\n' "$run_dir" >&2
started=$SECONDS

env -u RUNNER_TEMP NODE_BOOT_UNPRIVILEGED=1 NODE_BOOT_SKIP_BASIC=1 \
    NODE_BOOT_E2E_ONLY=rows OTERYN_E2E_ROW_BATCH=226:1 \
    timeout 4500 bash tools/qualification/node_boot/cloud_run.sh rows 2>&1 \
    | tee "$run_dir/cloud.log"

boot_dir=$(sed -n 's/^Node boot logs are in //p' "$run_dir/cloud.log")
boot_log="$boot_dir/1-rows.log"
grep -x 'NODE_BOOT_RESULT=NODE_BOOT_PASS mode=unprivileged' "$boot_log"
grep '^SEAM_EVIDENCE e2e_all scenario=N07d\.client ' "$boot_log" > "$run_dir/snapshot.log"
[[ $(wc -l < "$run_dir/snapshot.log") == 1 ]]
grep -q '^SEAM_EVIDENCE e2e_all scenario=N07d\.client status=MISSING-CLIENT .* evidence=exact-minimap-replay-required;' \
    "$run_dir/snapshot.log"
test -s "$E2E_ALL_CAPTURE_DIR/n07d-snapshot.bin"
test -s "$E2E_ALL_CAPTURE_DIR/n07d-expected.txt"

replay=play::feature_local_minimap::state::e2e_n07d::e2e_n07d_replays_admitted_map_into_the_explored_minimap
CARGO_PROFILE_TEST_DEBUG=0 "${CARGO:-cargo}" test --locked -p oteryn-client --lib "$replay" -- --exact 2>&1 \
    | tee "$run_dir/client.log"
grep -Fx "test $replay ... ok" "$run_dir/client.log" >/dev/null
grep -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$run_dir/client.log"
printf 'SEAM_EVIDENCE e2e_all scenario=N07d.client status=MISSING-CLIENT seconds=%s evidence=exact-terrain-replay-passed;live-admission-envelope-and-relog-required\n' \
    "$((SECONDS - started))"
