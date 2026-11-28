#!/usr/bin/env bash
# Run the selected P12a client row and replay this invocation's live capture.
set -euo pipefail
umask 077
unset RUNNER_TEMP
export OTERYN_AUTO_START=0
source /workspace/oteryn-env/env.sh >/dev/null
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.."
export E2E_ALL_CAPTURE_DIR
E2E_ALL_CAPTURE_DIR=$(mktemp -d /tmp/p12a-client.XXXXXX)
echo "P12a evidence: $E2E_ALL_CAPTURE_DIR"
wrapper_log="$E2E_ALL_CAPTURE_DIR/wrapper.log"
boot_exit=0
NODE_BOOT_UNPRIVILEGED=1 NODE_BOOT_SKIP_BASIC=1 NODE_BOOT_E2E_ONLY=rows \
    OTERYN_E2E_ROW_BATCH=167:1 timeout 4500 bash tools/qualification/node_boot/cloud_run.sh rows \
    >"$wrapper_log" 2>&1 || boot_exit=$?
cat "$wrapper_log"
log_dir=$(sed -n 's/^Node boot logs are in //p' "$wrapper_log")
log="$log_dir/1-rows.log"
if [[ -n "$log_dir" && -f "$log" ]]; then
    grep -E '^NODE_BOOT_RESULT=|^SEAM_EVIDENCE .*scenario=P12a\.client ' "$log" || true
fi
if (( boot_exit != 0 )); then
    if [[ -z "$log_dir" ]] || ! grep -q '^SEAM_EVIDENCE e2e_all scenario=P12a\.client status=FAIL' "$log"; then
        printf 'SEAM_EVIDENCE e2e_all scenario=P12a.client status=FAIL cause=node_boot\n'
    fi
    exit "$boot_exit"
fi
mapfile -t sessions < <(sed -nE 's/^SEAM_EVIDENCE e2e_capture scenario=P12a\.client status=CAPTURED .*;session=([0-9a-f-]{36});.*/\1/p' "$log")
if (( ${#sessions[@]} != 1 )); then
    printf 'SEAM_EVIDENCE e2e_all scenario=P12a.client status=FAIL cause=missing_or_duplicate_capture\n'
    exit 1
fi
export OTERYN_P12A_REPLAY_SESSION="${sessions[0]}"
replay_log="$E2E_ALL_CAPTURE_DIR/replay.log"
if ! cargo test --locked -p oteryn-client --lib \
    e2e_p12a_replays_titles_and_refused_unearned_select -- --nocapture >"$replay_log" 2>&1; then
    cat "$replay_log"
    printf 'SEAM_EVIDENCE e2e_all scenario=P12a.client status=FAIL cause=replay\n'
    exit 1
fi
cat "$replay_log"
test "$(grep -c '^SEAM_EVIDENCE e2e_all scenario=P12a\.client status=PASS ' "$replay_log")" = 1
