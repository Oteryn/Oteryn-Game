#!/usr/bin/env bash
# One fresh S18.client capture and its mandatory Session replay.
set -euo pipefail
umask 077
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.."
unset RUNNER_TEMP
[[ -z "$(git status --porcelain)" ]]
export S18_CAPTURE_HEAD="$(git rev-parse HEAD)"
export S18_CAPTURE_RUN="$(python3 -c 'import secrets; print(secrets.token_hex(16))')"
export E2E_ALL_CAPTURE_DIR="$(mktemp -d /tmp/s18-capture.XXXXXXXX)"
export NODE_BOOT_UNPRIVILEGED=1 NODE_BOOT_SKIP_BASIC=1 NODE_BOOT_E2E_ONLY=rows
export OTERYN_E2E_ROW_BATCH=184:1
printf 'S18 capture directory %s\n' "$E2E_ALL_CAPTURE_DIR"
boot_log="$E2E_ALL_CAPTURE_DIR/wrapper.log"
timeout 4500 bash tools/qualification/node_boot/cloud_run.sh rows 2>&1 | tee "$boot_log"
boot_dir="$(sed -n 's/^Node boot logs are in //p' "$boot_log")"
boot_log="$boot_dir/1-rows.log"
grep -Fx "NODE_BOOT_RESULT=NODE_BOOT_PASS mode=unprivileged" "$boot_log"
grep -E "^NODE_BOOT_EVIDENCE pins .* game=$S18_CAPTURE_HEAD$" "$boot_log"
grep -Ex "SEAM_EVIDENCE e2e_capture scenario=S18\.client status=CAPTURED seconds=[0-9.]+ evidence=head=$S18_CAPTURE_HEAD;run=$S18_CAPTURE_RUN;.*" "$boot_log"
[[ "$(grep -c '^SEAM_EVIDENCE e2e_capture scenario=S18\.client status=CAPTURED ' "$boot_log")" == 1 ]]
[[ -z "$(git status --porcelain)" && "$(git rev-parse HEAD)" == "$S18_CAPTURE_HEAD" ]]

export OTERYN_AUTO_START=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
source /workspace/oteryn-env/env.sh >/dev/null 2>&1
cargo test --locked -p oteryn-session --lib e2e_s18_say_and_whisper_echo_replay -- --ignored --nocapture |
    tee "$E2E_ALL_CAPTURE_DIR/replay.log"
grep -Fx 'test tests::e2e_s18::e2e_s18_say_and_whisper_echo_replay ... ok' "$E2E_ALL_CAPTURE_DIR/replay.log"
printf 'SEAM_EVIDENCE e2e_all scenario=S18.client status=PASS seconds=%s evidence=head=%s;run=%s;live-and-replay=verified\n' "$SECONDS" "$S18_CAPTURE_HEAD" "$S18_CAPTURE_RUN"
