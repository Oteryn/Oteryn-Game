#!/usr/bin/env bash
# Qualify only I18b.full, then replay its untouched browse-field payloads in the client.
set -euo pipefail
umask 077
unset RUNNER_TEMP

cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.."
test "$(id -u)" = 1000
test "$(stat -c '%u %a %n' /)" = '0 755 /'
test "$(stat -c '%u %a %n' /tmp)" = '0 1777 /tmp'

export OTERYN_AUTO_START=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
source /workspace/oteryn-env/env.sh
export CARGO_BUILD_JOBS=2
export E2E_ALL_CAPTURE_DIR
E2E_ALL_CAPTURE_DIR=$(mktemp -d /tmp/i18b-capture.XXXXXXXX)
printf 'I18b captures are in %s\n' "$E2E_ALL_CAPTURE_DIR"

wrapper_log="$E2E_ALL_CAPTURE_DIR/node-wrapper.log"
if ! NODE_BOOT_UNPRIVILEGED=1 NODE_BOOT_SKIP_BASIC=1 NODE_BOOT_E2E_ONLY=rows \
    OTERYN_E2E_ROW_BATCH=275:1 timeout 4500 bash tools/qualification/node_boot/cloud_run.sh rows \
    2>&1 | tee "$wrapper_log"; then
    exit 1
fi
node_log="$(sed -n 's/^Node boot logs are in //p' "$wrapper_log")/1-rows.log"
grep '^SEAM_EVIDENCE e2e_all scenario=I18b.full status=PASS ' "$node_log"
grep '^NODE_BOOT_RESULT=NODE_BOOT_PASS ' "$node_log"

replay_log="$E2E_ALL_CAPTURE_DIR/client.log"
cargo +1.100.0 test --locked -p oteryn-client --lib --features bevy-shell i18b_live_browse_field_replay \
    -- --ignored --nocapture 2>&1 | tee "$replay_log"
grep -Fqx "I18B_CLIENT_REPLAY PASS run=${E2E_ALL_CAPTURE_DIR##*/} phases=3" "$replay_log"
grep -q '^test result: ok\. 1 passed; 0 failed;' "$replay_log"
