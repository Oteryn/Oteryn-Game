#!/usr/bin/env bash
set +x
umask 077

blocked() {
  printf 'CLOUD_RUN=BLOCKED reason=%s\n' "$1"
  exit 2
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
[[ $(stat -c %u / 2>/dev/null) == 0 && $(stat -c %u /tmp 2>/dev/null) == 0 ]] \
  || blocked need_escalated_sandbox

export OTERYN_AUTO_START=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
# shellcheck source=/dev/null
source /workspace/oteryn-env/env.sh >/dev/null 2>&1 || blocked cloud_env_unavailable
export OTERYN_AUTO_START=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
unset RUNNER_TEMP
docker info >/dev/null 2>&1 || blocked docker_unavailable

script_dir=$(dirname -- "${BASH_SOURCE[0]}")
cd -- "$script_dir/../../.." 2>/dev/null || blocked checkout_unavailable
work_dir=$(mktemp -d /tmp/node-boot-cloud.XXXXXX 2>/dev/null) || blocked work_dir_unavailable
printf 'Node boot logs are in %s\n' "$work_dir" >&2

failed=0
index=0
for group in "$@"; do
  index=$((index + 1))
  log="$work_dir/$index-$group.log"
  started=$SECONDS
  status=0
  NODE_BOOT_UNPRIVILEGED=1 NODE_BOOT_E2E_ONLY="$group" \
    bash tools/qualification/node_boot/run.sh > "$log" 2>&1 || status=$?
  summarize "$log" "$group" "$((SECONDS - started))" "$status" || failed=1
  [[ "$status" == 0 ]] || failed=1
done
exit "$failed"
