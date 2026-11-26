replay_client() {
  local log=$1 captures=$2 row=$3 package=$4 case_name=$5 started=$SECONDS
  local status=FAIL provenance=''
  local -a cargo_args=() test_args=()
  if [[ "$row" == W13.client ]]; then
    provenance=$(sed -n 's/^SEAM_EVIDENCE e2e_capture scenario=W13\.client status=CAPTURED evidence=//p' "$log")
    if [[ -n "$provenance" ]]; then
      cargo_args=("${NODE_CLOCK_FEATURES[@]}" "${NODE_SPAWN_FEATURES[@]}")
      test_args=(-- --ignored --exact)
    fi
  fi
  # The journey owns its failure verdict. A second result would invalidate
  # the evidence consumed by e2e_all's duplicate-scenario check.
  if grep -qF "e2e_all scenario=$row status=FAIL " "$log"; then
    return 1
  fi
  # The live journey and the exact client replay must both reach their assertions.
  if [[ $(grep -cF "e2e_capture scenario=$row status=CAPTURED " "$log") == 1 ]] \
    && [[ "$row" != W13.client || -n "$provenance" ]] \
    && E2E_ALL_CAPTURE_DIR="$captures" W13_CAPTURE_PROVENANCE="$provenance" \
      "${CARGO:-cargo}" test --locked -p "$package" --lib "${cargo_args[@]}" "$case_name" "${test_args[@]}" \
      > "$captures/$row.replay.log" 2>&1 \
    && grep -qE '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$captures/$row.replay.log"; then
    status=PASS
  fi
  printf 'SEAM_EVIDENCE e2e_all scenario=%s status=%s seconds=%s.000 evidence=live-capture-replay\n' \
    "$row" "$status" "$((SECONDS - started))"
  [[ "$status" == PASS ]]
}

selected_client_replays() {
  local start length index row package case_name value
  local batch=${OTERYN_E2E_ROW_BATCH-0:15}
  if [[ $batch == all ]]; then
    start=0
    length=197
  elif [[ $batch =~ ^[+]?0*([0-9]+):[+]?0*([0-9]+)$ ]]; then
    start=${BASH_REMATCH[1]}
    length=${BASH_REMATCH[2]}
    # Match the 64-bit Rust selector's overflow check before bounding lengths.
    local LC_ALL=C
    for value in "$start" "$length"; do
      if (( ${#value} > 20 )) ||
        { (( ${#value} == 20 )) && [[ $value > 18446744073709551615 ]]; }; then
        return 0
      fi
    done
    # All registered replays are below 197. Avoid signed shell overflow.
    (( ${#start} <= 3 )) || return 0
    if (( ${#length} > 3 )); then
      length=999
    fi
    start=$((10#$start))
    length=$((10#$length))
  else
    return 0
  fi
  while read -r index row package case_name; do
    if [[ $row == W13.client && ${NODE_BOOT_W13_CLOUD_REPLAY:-0} == 1 ]]; then
      continue
    fi
    if (( index >= start && index - start < length )); then
      printf '%s %s %s\n' "$row" "$package" "$case_name"
    fi
  done <<'REPLAYS'
190 W04.client oteryn-client e2e_w04_replays_raid_schedule_state_and_refusal
192 W05c.client oteryn-client-ui w05c_rashid_payloads_replay_into_service_panel
193 W10.client oteryn-client-ui e2e_w10_replays_hazard_state_selection_and_locked_refusal
196 W13.client oteryn-game-server qualification::client_world::client_row_w13::e2e_w13_replays_daily_boost_selection_and_refusal
REPLAYS
}

node_boot_client_replays() {
  local log=$1 row package case_name result=0
  while read -r row package case_name; do
    replay_client "$log" "$E2E_ALL_CAPTURE_DIR" "$row" "$package" "$case_name" || result=1
  done < <(selected_client_replays)
  return "$result"
}
