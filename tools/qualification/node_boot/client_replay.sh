replay_client() {
  local log=$1 captures=$2 row=$3 package=$4 case_name=$5 started=$SECONDS
  local status=FAIL capture_sha= actual_sha=
  local replay_args=()
  # The journey owns its failure verdict. A second result would invalidate
  # the evidence consumed by e2e_all's duplicate-scenario check.
  if grep -qF "e2e_all scenario=$row status=FAIL " "$log"; then
    return 1
  fi
  if [[ "$row" == W04.client ]]; then
    replay_args=(-- --ignored)
    actual_sha=$(sha256sum < "$captures/w04-exchange.json") || actual_sha=
    actual_sha=${actual_sha%% *}
    capture_sha=$(sed -nE 's/^SEAM_EVIDENCE e2e_capture scenario=W04\.client status=CAPTURED evidence=([a-f0-9]{64})$/\1/p' "$log")
  fi
  # The live journey and the exact client replay must both reach their assertions.
  if [[ $(grep -cF "e2e_capture scenario=$row status=CAPTURED " "$log") == 1 ]] \
    && [[ "$row" != W04.client || ( "$capture_sha" =~ ^[a-f0-9]{64}$ && "$actual_sha" == "$capture_sha" ) ]] \
    && W04_CAPTURE_SHA256="$capture_sha" E2E_ALL_CAPTURE_DIR="$captures" "${CARGO:-cargo}" test --locked -p "$package" --lib "$case_name" "${replay_args[@]}" \
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
  else
    local bounds
    bounds=$(python3 - "$batch" <<'PY_BATCH'
import re
import sys

match = re.fullmatch(r"\+?([0-9]+):\+?([0-9]+)", sys.argv[1])
if not match:
    sys.exit(1)
bounds = [part.lstrip("0") or "0" for part in match.groups()]
if any(len(part) > 20 or int(part) > sys.maxsize * 2 + 1 for part in bounds):
    sys.exit(1)
print(*(min(int(part), 197) for part in bounds))
PY_BATCH
    ) || return 0
    read -r start length <<< "$bounds"
  fi
  while read -r index row package case_name; do
    if (( index >= start && index - start < length )); then
      printf '%s %s %s\n' "$row" "$package" "$case_name"
    fi
  done <<'REPLAYS'
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
