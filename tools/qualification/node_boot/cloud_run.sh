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
      $3 ~ /^scenario=[[:alnum:]_.-]+\.full$/ && $4 ~ /^status=(PASS|FAIL)$/ {
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

replay_n07a() {
  local log=$1 capture=$2 sessions replay_log="$2/replay.log"
  sessions=$(sed -nE 's/^SEAM_EVIDENCE e2e_(capture|all) scenario=N07a\.client status=CAPTURED evidence=session=([0-9a-f-]{36}) relog=([0-9a-f-]{36}) .*/\2 \3/p' "$log")
  [[ "$sessions" =~ ^[0-9a-f-]{36}\ [0-9a-f-]{36}$ ]] || return 1
  N07A_LIVE_SESSIONS="$sessions" E2E_ALL_CAPTURE_DIR="$capture" \
    cargo test --locked -p oteryn-client --lib \
    play::feature_0097_character_info::protocol::e2e_n07a::e2e_n07a_live_replays_captured_title_change \
    -- --exact --ignored --nocapture > "$replay_log" 2>&1 || return 1
  grep -Fxq "N07A_LIVE_REPLAY session=${sessions% *} relog=${sessions#* }" "$replay_log"
}

failed=0
index=0
for group in "$@"; do
  index=$((index + 1))
  log="$work_dir/$index-$group.log"
  started=$SECONDS
  status=0
  n07a_capture=
  capture_env=()
  if [[ "$group/${OTERYN_E2E_ROW_BATCH:-}" == rows/223:1 ]]; then
    n07a_capture="$work_dir/$index-n07a"
    mkdir "$n07a_capture" || blocked work_dir_unavailable
    capture_env+=("E2E_ALL_CAPTURE_DIR=$n07a_capture")
  fi
  env "${capture_env[@]}" NODE_BOOT_UNPRIVILEGED=1 NODE_BOOT_E2E_ONLY="$group" \
    bash tools/qualification/node_boot/run.sh > "$log" 2>&1 &
  child_pid=$!
  wait "$child_pid" || status=$?
  child_pid=
  if [[ -n "$n07a_capture" ]]; then
    if [[ "$status" == 0 ]] && replay_n07a "$log" "$n07a_capture"; then
      line="SEAM_EVIDENCE e2e_all scenario=N07a.client status=PASS evidence=title-change-refusal-relog-live-replay"
      printf '%s\n' "$line" | tee -a "$log"
    else
      line="SEAM_EVIDENCE e2e_all scenario=N07a.client status=FAIL evidence=live-capture-or-replay-failed"
      grep -E '^SEAM_EVIDENCE e2e_all scenario=N07a\.client status=FAIL ' "$log" || printf '%s\n' "$line" | tee -a "$log"
      status=1
    fi
  fi
  summarize "$log" "$group" "$((SECONDS - started))" "$status" || failed=1
  [[ "$status" == 0 ]] || failed=1
done
exit "$failed"
