#!/usr/bin/env bash
# Replay the native client against captures from this selected live seam only.
set -euo pipefail
work=$1
shift

groups=${NODE_BOOT_E2E_ONLY:-}
if [[ -n "${groups//[[:space:]]/}" && ",${groups//[[:space:]]/}," != *,rows,* ]]; then
  exec "$@"
fi
batch=${OTERYN_E2E_ROW_BATCH:-0:15}
if [[ "$batch" != all ]]; then
  if ! python3 - "$batch" <<'PY'
import re
import sys

match = re.fullmatch(r"\+?([0-9]+):\+?([0-9]+)", sys.argv[1])
if not match:
    sys.exit(1)
start, count = (part.lstrip("0") or "0" for part in match.groups())
if len(start) > 3 or len(count) > 20:
    sys.exit(1)
sys.exit(not (0 <= 197 - int(start) < int(count) <= 2**64 - 1))
PY
  then
    exec "$@"
  fi
fi

started=$SECONDS
capture=$(mktemp -d "$work/s06-client.XXXXXX")
failed() {
  printf 'SEAM_EVIDENCE e2e_all scenario=S06.client status=FAIL seconds=%d evidence=%s\n' "$((SECONDS - started))" "$1"
  exit 1
}

NODE_BOOT_S06_CAPTURE_DIR="$capture" "$@" | tee "$capture/live.log"
grep -q '^SEAM_EVIDENCE e2e_capture scenario=S06\.client status=CAPTURED ' "$capture/live.log" \
  || failed live-capture-missing
if grep -Eq '^SEAM_EVIDENCE (e2e_all scenario=S06\.client status=FAIL |QUALIFICATION_CAUSE row=S06\.client |S06_CLIENT_CAPTURE_WRITE_ERROR )' "$capture/live.log"; then
  failed live-capture-failed
fi
for file in s06-before.bin s06-after.bin s06-commit.bin s06-refusal.bin s06-expected.txt s06-request.bin s06-refused-request.bin; do
  [[ -s "$capture/$file" ]] || failed "missing-$file"
done

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)
test_name=play::feature_0025_appearance::protocol::e2e_s06::e2e_s06_outfit_commit_and_unearned_refusal_replay
if ! E2E_ALL_CAPTURE_DIR="$capture" cargo +1.100.0 test --manifest-path "$root/Cargo.toml" --locked -p oteryn-client --lib \
  "$test_name" -- --exact | tee "$capture/replay.log"; then
  failed client-replay-failed
fi
grep -Fxq "test $test_name ... ok" "$capture/replay.log" || failed client-replay-not-executed
printf 'SEAM_EVIDENCE e2e_all scenario=S06.client status=PASS seconds=%d evidence=live-outfit-commit-and-addon-refusal-replayed\n' "$((SECONDS - started))"
