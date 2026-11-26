# Sourced by run.sh for the indexed S01.client capture and replay.
s01_client_selected() {
  [[ "$NODE_BOOT_WORLD/$NODE_BOOT_SPELLS/$NODE_BOOT_D150_LEGACY" == 1/0/0 ]] || return 3
  python3 - "${NODE_BOOT_E2E_ONLY:-}" "${OTERYN_E2E_ROW_BATCH-0:15}" <<'PY_S01_SELECTED'
import re
import sys

groups, batch = sys.argv[1:]
if groups.strip() and "rows" not in {group.strip() for group in groups.split(",")}:
    sys.exit(3)
if batch == "all":
    sys.exit(0)
match = re.fullmatch(r"\+?([0-9]+):\+?([0-9]+)", batch)
if not match:
    sys.exit(3)
first, count = (digits.lstrip("0") or "0" for digits in match.groups())
usize_max = 2 * sys.maxsize + 1
if any(len(digits) > len(str(usize_max)) for digits in (first, count)):
    sys.exit(3)
first, count = int(first), int(count)
# Keep this index aligned with e2e_row_selected in client_phases/world.rs.
sys.exit(0 if count <= usize_max and 0 <= 179 - first < count else 3)
PY_S01_SELECTED
}

s01_client_replay() {
  local seam_log="$WORK/stage-seam_stages.log"
  local started=$SECONDS output reason
  local test_name=party::e2e_s01::party_replay_invite_join_and_leadership_pass
  if [[ $(grep -c '^SEAM_EVIDENCE e2e_capture scenario=S01\.client status=CAPTURED ' "$seam_log") != 1 ]]; then
    reason=live-capture-missing-or-duplicate
  elif output=$(E2E_ALL_CAPTURE_DIR="$NODE_BOOT_S01_CAPTURE_DIR" cargo +1.100.0 test --locked \
      -p oteryn-client-ui --lib "$test_name" -- --exact 2>&1); then
    printf '%s\n' "$output"
    if grep -Fxq "test $test_name ... ok" <<< "$output" &&
      grep -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' <<< "$output"; then
      printf 'SEAM_EVIDENCE e2e_all scenario=S01.client status=PASS seconds=%s evidence=live-party-ui-replay\n' \
        "$((SECONDS - started))"
      return 0
    fi
    reason=replay-test-did-not-run
  else
    printf '%s\n' "$output"
    reason=replay-test-failed
  fi
  printf 'SEAM_EVIDENCE e2e_all scenario=S01.client status=FAIL seconds=%s evidence=%s\n' \
    "$((SECONDS - started))" "$reason"
  return 1
}
