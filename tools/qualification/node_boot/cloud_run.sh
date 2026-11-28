#!/usr/bin/env bash
set +x
umask 077

blocked() {
  printf 'CLOUD_RUN=BLOCKED reason=%s\n' "$1"
  exit 2
}

select_work_parent() {
  local candidate parent ancestor owner
  for candidate in "$@"; do
    [[ -n "$candidate" ]] || continue
    parent="$(realpath -e -- "$candidate" 2>/dev/null)" || continue
    [[ -d "$parent" && -w "$parent" && -x "$parent" && "$parent" != /srv && "$parent" != /srv/* ]] || continue
    ancestor="$parent"
    while :; do
      owner="$(stat -c %u -- "$ancestor")" || break
      [[ "$owner" == 0 || "$owner" == "$EUID" ]] || break
      if [[ "$ancestor" == / ]]; then
        printf '%s\n' "$parent"
        return 0
      fi
      ancestor="${ancestor%/*}"
      ancestor="${ancestor:-/}"
    done
  done
  return 1
}

summarize() {
  local log=$1 group=$2 seconds=${3:-0} status=${4:-0} result rows row
  result=$(awk '/^NODE_BOOT_RESULT=(NODE_BOOT_[A-Z0-9_]+|BLOCKED)([[:space:]]|$)/ {
    sub(/^NODE_BOOT_RESULT=/, "", $1)
    result = $1
  } END { print result }' < "$log") || return 1
  # A recursive basic pass can succeed before the outer run exits without a result.
  if [[ -z "$result" || ( "$status" != 0 && "$result" == NODE_BOOT_PASS ) ]]; then
    result=NODE_BOOT_FAIL
  fi

  rows=$(awk '
    $1 == "SEAM_EVIDENCE" && $2 == "e2e_all" &&
      ($3 ~ /^scenario=[[:alnum:]_.-]+\.full$/ || $3 == "scenario=W24.client") &&
      $4 ~ /^status=(PASS|FAIL)$/ {
      row = $3
      sub(/^scenario=/, "", row)
      sub(/\.full$/, "", row)
      verdict = substr($4, 8)
      cause = evidence = ""
      for (i = 5; i <= NF; i++) {
        if ($i ~ /^cause=/) cause = substr($i, 7)
        if ($i ~ /^evidence=/) evidence = substr($i, 10)
      }
      reason = (cause != "" ? cause : evidence)
      if (tolower(reason) ~ /password|postgres:\/\/|postgresql:\/\/|secret|token|key=/) reason = ""
      gsub(/\|/, "_", reason)
      printf "%s|%s|%s\n", row, verdict, reason
    }' < "$log") || return 1

  grep -aE '^NODE_BOOT_PORTS ([A-Z_]+=[0-9]+ ?)+$' "$log" | sort -u || true
  printf 'CLOUD_RUN group=%s result=%s seconds=%s\nROWS-BEGIN\n' "$group" "$result" "$seconds"
  [[ -z "$rows" ]] || printf '%s\n' "$rows"
  printf 'ROWS-END\n'

  while IFS= read -r row; do
    grep -aFw -- "$row" "$log" | grep -Ei 'GAP|Error|refus|DIAG' \
      | grep -vEi 'password|postgres://|postgresql://|secret|token|key=' | head -n 6
  done < <(awk -F '|' '$2 == "FAIL" && !seen[$1]++ { print $1 }' <<< "$rows")
}

w24_client_selected() {
  [[ $1 == rows ]] || return 1
  python3 - "${OTERYN_E2E_ROW_BATCH:-}" "${2:-222}" <<'PY'
import re
import sys

batch = sys.argv[1]
if batch == "all":
    sys.exit(0)
parsed = re.fullmatch(r"\+?([0-9]+):\+?([0-9]+)", batch)
if not parsed:
    sys.exit(1)
values = [digits.lstrip("0") or "0" for digits in parsed.groups()]
if any(len(value) > 20 for value in values):
    sys.exit(1)
start, length = map(int, values)
index = int(sys.argv[2])
sys.exit(not (start <= index and index - start < length <= 2**64 - 1))
PY
}

w24_full_playback() {
  local capture_dir=$1 playback_log=$1.playback.log
  E2E_ALL_W24_PLAYBACK_DIR="$capture_dir" cargo +1.100.0 test --locked -p oteryn-client \
    --features native-audio --lib w24_ambient_capture_reaches_native_mixer \
    -- --nocapture > "$playback_log" 2>&1 || return 1
  grep -Eq '^test .*::w24_ambient_capture_reaches_native_mixer \.\.\. ok$' "$playback_log" &&
    grep -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$playback_log"
}

w24_client_replay() {
  local capture_dir=$1 log=$2 mode=${3:-client} replay_log=$1.replay.log
  [[ $mode != full ]] || replay_log=$1.full.replay.log
  local test=tests::e2e_w24::e2e_w24_ambient_zone_sound_replay
  [[ $(grep -c "^SEAM_EVIDENCE e2e_capture scenario=W24[.]${mode} status=CAPTURED " "$log") == 1 ]] || return 1
  E2E_ALL_W24_MODE="$mode" E2E_ALL_CAPTURE_DIR="$capture_dir" cargo +1.100.0 test --locked -p oteryn-session --lib \
    "$test" -- --exact --nocapture > "$replay_log" 2>&1 || return 1
  grep -Fx "test $test ... ok" "$replay_log" >/dev/null &&
    grep -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$replay_log"
}

if [[ ${1:-} == --summarize ]]; then
  [[ $# == 3 && -r "$2" && "$3" =~ ^[a-z0-9_]+$ ]] || blocked invalid_summary_arguments
  summarize "$2" "$3"
  exit $?
fi

(( $# > 0 )) || blocked missing_group
for group in "$@"; do
  [[ "$group" =~ ^[a-z0-9_]+$ ]] || blocked invalid_group
done

uid=$(id -u 2>/dev/null) && [[ "$uid" =~ ^[0-9]+$ ]] || blocked uid_unavailable
[[ "$uid" != 0 ]] || blocked running_as_root
[[ $(stat -c %u / 2>/dev/null) == 0 ]] \
  || blocked need_escalated_sandbox

export OTERYN_AUTO_START=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
# shellcheck source=/dev/null
source /workspace/oteryn-env/env.sh >/dev/null 2>&1 || blocked cloud_env_unavailable
export OTERYN_AUTO_START=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
RUNNER_TEMP="$(select_work_parent /tmp /var/tmp /dev/shm "${RUNNER_TEMP:-}" /workspace)" \
  || blocked work_dir_unavailable
export RUNNER_TEMP
docker info >/dev/null 2>&1 || blocked docker_unavailable

script_dir=$(dirname -- "${BASH_SOURCE[0]}")
cd -- "$script_dir/../../.." 2>/dev/null || blocked checkout_unavailable
work_dir=$(mktemp -d "$RUNNER_TEMP/node-boot-cloud.XXXXXX" 2>/dev/null) || blocked work_dir_unavailable
printf 'Node boot logs are in %s\n' "$work_dir" >&2

child_pid=
interrupt_run() {
  trap '' INT TERM
  if [[ -n "$child_pid" ]]; then
    kill -TERM "$child_pid" 2>/dev/null || true
    wait "$child_pid" || true
  fi
  exit "$1"
}
trap 'interrupt_run 130' INT
trap 'interrupt_run 143' TERM

failed=0
index=0
for group in "$@"; do
  index=$((index + 1))
  log="$work_dir/$index-$group.log"
  started=$SECONDS
  status=0
  capture_dir=
  w24_modes=()
  if w24_client_selected "$group" 19; then w24_modes+=(full); fi
  if w24_client_selected "$group"; then w24_modes+=(client); fi
  if (( ${#w24_modes[@]} )); then
    capture_dir="$work_dir/$index-w24-captures"
    mkdir -- "$capture_dir" || blocked work_dir_unavailable
  fi
  (
    [[ -z $capture_dir ]] || export E2E_ALL_W24_CAPTURE_DIR="$capture_dir"
    NODE_BOOT_UNPRIVILEGED=1 NODE_BOOT_E2E_ONLY="$group" \
      exec bash tools/qualification/node_boot/run.sh
  ) > "$log" 2>&1 &
  child_pid=$!
  wait "$child_pid" || status=$?
  child_pid=
  if [[ -n $capture_dir && $status == 0 ]]; then
    for w24_mode in "${w24_modes[@]}"; do
      if w24_client_replay "$capture_dir" "$log" "$w24_mode" &&
        { [[ $w24_mode != full ]] || w24_full_playback "$capture_dir"; }; then
        printf 'SEAM_EVIDENCE e2e_all scenario=W24.%s status=PASS evidence=live-capture-replayed\n' "$w24_mode" >> "$log"
      else
        status=1
        printf 'SEAM_EVIDENCE e2e_all scenario=W24.%s status=FAIL evidence=live-capture-replay-failed\n' "$w24_mode" >> "$log"
        printf '%s\n' 'NODE_BOOT_RESULT=NODE_BOOT_FAIL mode=unprivileged' >> "$log"
      fi
    done
  fi
  summarize "$log" "$group" "$((SECONDS - started))" "$status" || failed=1
  [[ "$status" == 0 ]] || failed=1
done
exit "$failed"
