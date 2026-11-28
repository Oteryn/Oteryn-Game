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
      $3 ~ /^scenario=([[:alnum:]_.-]+\.full|P13\.client|P23c\.client)$/ && $4 ~ /^status=(PASS|FAIL)$/ {
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

selected_loyalty_replay() {
  [[ "$1" == rows ]] || return 1
  local batch=${OTERYN_E2E_ROW_BATCH:-} first length
  [[ "$batch" =~ ^([0-9]{1,7}):([0-9]{1,7})$ ]] || return 1
  first=$((10#${BASH_REMATCH[1]}))
  length=$((10#${BASH_REMATCH[2]}))
  local loyalty_row=169 # client_phases/world.rs, P13.client
  (( first <= loyalty_row && loyalty_row < first + length ))
}

replay_loyalty_capture() {
  local log=$1 capture_dir=$2 replay_log=$3
  local test=loyalty_skills::e2e_p13::e2e_p13_replays_snapshot_and_malformed_query_rejection
  local started=$SECONDS
  if grep -q '^SEAM_EVIDENCE e2e_all scenario=P13\.client status=FAIL ' "$log"; then
    return 1
  fi
  if grep -q '^SEAM_EVIDENCE e2e_capture scenario=P13\.client status=CAPTURED ' "$log" &&
    E2E_ALL_CAPTURE_DIR="$capture_dir" cargo +1.100.0 test --locked -p oteryn-client-ui --lib "$test" -- --exact >"$replay_log" 2>&1 &&
    grep -Fqx "test $test ... ok" "$replay_log"; then
    printf 'SEAM_EVIDENCE e2e_all scenario=P13.client status=PASS seconds=%s evidence=live-capture-replayed\n' "$((SECONDS - started))" >>"$log"
    return 0
  fi
  printf 'SEAM_EVIDENCE e2e_all scenario=P13.client status=FAIL seconds=%s evidence=client-replay-failed\n' "$((SECONDS - started))" >>"$log"
  return 1
}

selected_p23c_replay() {
  [[ "$1" == rows ]] || return 1
  local batch=${OTERYN_E2E_ROW_BATCH:-}
  [[ "$batch" =~ ^([0-9]{1,7}):([0-9]{1,7})$ ]] || return 1
  local first=$((10#${BASH_REMATCH[1]})) length=$((10#${BASH_REMATCH[2]}))
  (( first <= 178 && 178 < first + length ))
}

replay_p23c_capture() {
  local log=$1 capture_dir=$2 replay_log=$3
  local test=task_board::e2e_p23c::e2e_p23c_replays_hunting_task_query_and_select
  local started=$SECONDS
  if grep -q '^SEAM_EVIDENCE e2e_all scenario=P23c\.client status=FAIL ' "$log"; then
    return 1
  fi
  if [[ $(grep -Fxc "SEAM_EVIDENCE e2e_capture scenario=P23c.client status=CAPTURED run=${capture_dir##*/}" "$log") == 1 ]] &&
    OTERYN_P23C_CAPTURE_DIR="$capture_dir" cargo +1.100.0 test --locked -p oteryn-client-ui --lib "$test" -- --exact --nocapture >"$replay_log" 2>&1 &&
    grep -Fqx "P23C_REPLAY verified run=${capture_dir##*/}" "$replay_log"; then
    printf 'SEAM_EVIDENCE e2e_all scenario=P23c.client status=PASS seconds=%s evidence=live-capture-replayed\n' "$((SECONDS - started))" >>"$log"
    return 0
  fi
  printf 'SEAM_EVIDENCE e2e_all scenario=P23c.client status=FAIL seconds=%s evidence=client-replay-failed\n' "$((SECONDS - started))" >>"$log"
  return 1
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
  if selected_loyalty_replay "$group"; then
    capture_dir=$(mktemp -d "$work_dir/$index-loyalty.XXXXXX") || blocked work_dir_unavailable
  fi
  p23c_capture_dir=
  if selected_p23c_replay "$group"; then
    p23c_capture_dir=$(mktemp -d "$work_dir/$index-p23c.XXXXXX") || blocked work_dir_unavailable
  fi
  (
    [[ -z "$p23c_capture_dir" ]] || export OTERYN_P23C_CAPTURE_DIR="$p23c_capture_dir"
    [[ -z "$capture_dir" ]] || export E2E_ALL_CAPTURE_DIR="$capture_dir"
    export NODE_BOOT_UNPRIVILEGED=1 NODE_BOOT_E2E_ONLY="$group"
    exec bash tools/qualification/node_boot/run.sh
  ) > "$log" 2>&1 &
  child_pid=$!
  wait "$child_pid" || status=$?
  child_pid=
  if [[ -n "$capture_dir" ]] && ! replay_loyalty_capture "$log" "$capture_dir" "$work_dir/$index-loyalty.log"; then
    status=1
  fi
  if [[ -n "$p23c_capture_dir" ]] && ! replay_p23c_capture "$log" "$p23c_capture_dir" "$work_dir/$index-p23c.log"; then
    status=1
  fi
  summarize "$log" "$group" "$((SECONDS - started))" "$status" || failed=1
  [[ "$status" == 0 ]] || failed=1
done
exit "$failed"
