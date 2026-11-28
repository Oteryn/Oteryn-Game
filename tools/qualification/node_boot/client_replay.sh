w01c_capture_matches_log() {
  local log=$1 captures=$2 expected="$2/w01c-expected.txt" frame checkpoint
  frame=$(sha256sum < "$captures/w01c-vitals.bin") || return 1
  checkpoint=$(sha256sum < "$captures/w01c-before.bin") || return 1
  grep -Fxq "frame_sha256=${frame%% *}" "$expected" &&
    grep -Fxq "checkpoint_sha256=${checkpoint%% *}" "$expected" &&
    grep -Fxq "SEAM_EVIDENCE e2e_capture scenario=W01c.client status=CAPTURED evidence=\"$(tr '\n' ';' < "$expected")\"" "$log"
}

replay_client() {
  local log=$1 captures=$2 row=$3 package=$4 case_name=$5 started=$SECONDS
  local status=FAIL
  # The journey owns its failure verdict. A second result would invalidate
  # the evidence consumed by e2e_all's duplicate-scenario check.
  if grep -qF "e2e_all scenario=$row status=FAIL " "$log"; then
    return 1
  fi
  # The live journey and the exact client replay must both reach their assertions.
  if [[ $(grep -cF "e2e_capture scenario=$row status=CAPTURED " "$log") == 1 ]] \
    && { [[ "$row" != W01c.client ]] || w01c_capture_matches_log "$log" "$captures"; } \
    && E2E_ALL_CAPTURE_DIR="$captures" "${CARGO:-cargo}" test --locked -p "$package" --lib "$case_name" \
      > "$captures/$row.replay.log" 2>&1 \
    && grep -qE '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$captures/$row.replay.log"; then
    status=PASS
  fi
  printf 'SEAM_EVIDENCE e2e_all scenario=%s status=%s seconds=%s.000 evidence=live-capture-replay\n' \
    "$row" "$status" "$((SECONDS - started))"
  [[ "$status" == PASS ]]
}

selected_client_replays() {
  local start length index row package case_name
  local batch=${OTERYN_E2E_ROW_BATCH:-0:15}
  if [[ $batch == all ]]; then
    start=0
    length=197
  elif [[ $batch =~ ^([0-9]{1,7}):([0-9]{1,7})$ ]]; then
    start=$((10#${BASH_REMATCH[1]}))
    length=$((10#${BASH_REMATCH[2]}))
  else
    return 0
  fi
  while read -r index row package case_name; do
    if (( index >= start && index - start < length )); then
      printf '%s %s %s\n' "$row" "$package" "$case_name"
    fi
  done <<'REPLAYS'
187 W01c.client oteryn-session e2e_w01c_creature_attack_vitals_replay
190 W04.client oteryn-client e2e_w04_replays_raid_schedule_state_and_refusal
192 W05c.client oteryn-client-ui w05c_rashid_payloads_replay_into_service_panel
193 W10.client oteryn-client-ui e2e_w10_replays_hazard_state_selection_and_locked_refusal
196 W13.client oteryn-client-ui e2e_w13_replays_daily_boost_selection_and_refusal
REPLAYS
}

node_boot_client_replays() {
  local log=$1 row package case_name result=0
  while read -r row package case_name; do
    replay_client "$log" "$E2E_ALL_CAPTURE_DIR" "$row" "$package" "$case_name" || result=1
  done < <(selected_client_replays)
  return "$result"
}
