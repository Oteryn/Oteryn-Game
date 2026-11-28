#!/usr/bin/env bash
# Replay the native client against captures from this selected live seam only.
set -euo pipefail
work=$1
shift

# Rust str::trim uses Unicode White_Space, including NBSP and excluding U+001C–U+001F.
if ! python3 - "${NODE_BOOT_E2E_ONLY:-}" <<'PYGROUPS'
import sys

space = " \t\n\v\f\r\x85\xa0\u1680" + "".join(map(chr, range(0x2000, 0x200b))) + "\u2028\u2029\u202f\u205f\u3000"
value = sys.argv[1]
sys.exit(bool(value.strip(space)) and "rows" not in {part.strip(space) for part in value.split(",")})
PYGROUPS
then
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
capture=$(mktemp -d "$work/s06-client.XXXXXXXXXXXX")
run=${capture##*/}
failed() {
  printf 'SEAM_EVIDENCE e2e_all scenario=S06.client status=FAIL seconds=%d evidence=%s\n' "$((SECONDS - started))" "$1"
  exit 1
}

NODE_BOOT_S06_CAPTURE_DIR="$capture" NODE_BOOT_S06_CAPTURE_RUN="$run" "$@" | tee "$capture/live.log"
grep -Fxq "SEAM_EVIDENCE e2e_capture scenario=S06.client status=CAPTURED evidence=run-$run" "$capture/live.log" \
  || failed live-capture-missing
if grep -Eq '^SEAM_EVIDENCE (e2e_all scenario=S06\.client status=FAIL |QUALIFICATION_CAUSE row=S06\.client |S06_CLIENT_CAPTURE_WRITE_ERROR )' "$capture/live.log"; then
  failed live-capture-failed
fi
for file in s06-before.bin s06-catalogue-event.bin s06-commit.bin s06-request.bin s06-expected.txt s06-refusal-{1,2}.bin s06-refused-request-{1,2}.bin; do
  [[ -s "$capture/$file" ]] || failed "missing-$file"
done

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)
test_name=play::feature_0025_appearance::protocol::e2e_s06::e2e_s06_outfit_commit_and_unearned_refusal_replay
if ! NODE_BOOT_S06_CAPTURE_DIR="$capture" NODE_BOOT_S06_CAPTURE_RUN="$run" cargo +1.100.0 test --manifest-path "$root/Cargo.toml" --locked -p oteryn-client --lib \
  "$test_name" -- --exact --ignored | tee "$capture/replay.log"; then
  failed client-replay-failed
fi
grep -Fxq "test $test_name ... ok" "$capture/replay.log" || failed client-replay-not-executed
printf 'SEAM_EVIDENCE e2e_all scenario=S06.client status=PASS seconds=%d evidence=live-outfit-commit-and-addon-refusal-replayed\n' "$((SECONDS - started))"
