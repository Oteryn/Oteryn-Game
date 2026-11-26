#!/usr/bin/env bash
# Own the P04 replay inside the seam stage, before node boot can publish success.
p04_selected() {
  python3 - "$1" "${OTERYN_E2E_ROW_BATCH-0:15}" <<'PY_SELECT'
import re
import sys

groups, batch = sys.argv[1:]
if groups.strip() and "rows" not in {name.strip() for name in groups.split(",")}:
    raise SystemExit(1)
if batch == "all":
    raise SystemExit(0)
if not re.fullmatch(r"[+]?[0-9]+:[+]?[0-9]+", batch):
    raise SystemExit(1)
start, count = map(int, batch.split(":"))
limit = 2 * sys.maxsize + 1
raise SystemExit(0 if start <= 159 and 159 - start < count <= limit else 1)
PY_SELECT
}

replay_p04_client() {
  local log=$1 captures=$2 replay_log="$1.p04-replay"
  local test_name=task_board::e2e_p04::e2e_p04_replays_query_and_refused_start_weekly
  local verdict=FAIL session
  session=$(sed -nE 's/^SEAM_EVIDENCE e2e_capture scenario=P04\.client status=CAPTURED evidence=session=([0-9a-f-]{36});.*/\1/p' "$log")
  if [[ "$session" =~ ^[0-9a-f-]{36}$ && $(grep -ac '^SEAM_EVIDENCE e2e_capture scenario=P04.client status=CAPTURED evidence=' "$log") == 1 ]] &&
     ! grep -aqE '^(SEAM_EVIDENCE e2e_all scenario=P04.client status=FAIL|SEAM_EVIDENCE P04_CLIENT_CAPTURE_WRITE_ERROR)' "$log" &&
     E2E_P04_CAPTURE_DIR="$captures" E2E_P04_SESSION="$session" cargo +1.100.0 test --locked -p oteryn-client-ui --lib "$test_name" \
       -- --ignored --exact > "$replay_log" 2>&1 &&
     grep -Fxq "test $test_name ... ok" "$replay_log"; then
    verdict=PASS
  fi
  printf 'SEAM_EVIDENCE e2e_all scenario=P04.client status=%s evidence=live_client_replay\n' "$verdict" | tee -a "$log"
  [[ "$verdict" == PASS ]]
}

p04_seam_with_replay() {
  local captures=$1 log=$2
  shift 2
  mkdir -- "$captures" || return
  local status=0
  E2E_P04_CAPTURE_DIR="$captures" "$@" 2>&1 | tee "$log" || status=$?
  if [[ "$status" != 0 ]]; then
    printf 'SEAM_EVIDENCE e2e_all scenario=P04.client status=FAIL evidence=seam_exit\n'
    return "$status"
  fi
  replay_p04_client "$log" "$captures"
}
