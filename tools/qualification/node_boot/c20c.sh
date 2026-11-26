# The row's packet verification and native client replay must pass in the same run.
c20c_replay() {
  local capture=$1 groups=${NODE_BOOT_E2E_ONLY:-} batch=${OTERYN_E2E_ROW_BATCH-0:15}
  local start length
  if [[ -n "${groups//[[:space:]]/}" && ",${groups//[[:space:]]/}," != *,rows,* ]]; then
    return
  fi
  if [[ "$batch" != all ]]; then
    [[ "$batch" =~ ^\+?([0-9]+):\+?([0-9]+)$ ]] || return 0
    start=${BASH_REMATCH[1]}
    length=${BASH_REMATCH[2]}
    # Strip leading zeroes and avoid shell arithmetic overflow for usize inputs.
    start=${start#"${start%%[!0]*}"}
    length=${length#"${length%%[!0]*}"}
    [[ ${#start} -le 3 && ${start:-0} -le 111 ]] || return 0
    [[ ${#length} -gt 3 || ${length:-0} -gt $((111 - ${start:-0})) ]] || return 0
  fi
  if grep -q '^SEAM_EVIDENCE e2e_all scenario=C20c.client status=FAIL ' "$capture/seam.log"; then
    return 1
  fi
  local -a files=(c20c-death-protection.bin c20c-expected.txt c20c-binding.txt)
  local binding payload_hash expected_hash
  if [[ ! -s "$capture/${files[0]}" || ! -s "$capture/${files[1]}" ]]; then
    echo 'SEAM_EVIDENCE e2e_all scenario=C20c.client status=FAIL evidence=fresh-capture-missing'
    return 1
  fi
  if [[ ! -s "$capture/${files[2]}" ]]; then
    echo 'SEAM_EVIDENCE e2e_all scenario=C20c.client status=FAIL evidence=selected-packet-verification-missing'
    return 1
  fi
  binding=$(cat "$capture/${files[2]}")
  payload_hash=$(sha256sum < "$capture/${files[0]}")
  expected_hash=$(sha256sum < "$capture/${files[1]}")
  local binding_format='^session=[0-9a-f-]{36} sequence=[1-9][0-9]* payload_sha256=[0-9a-f]{64} expected_sha256=[0-9a-f]{64}$'
  if [[ ! "$binding" =~ $binding_format
        || "$binding" != *" payload_sha256=${payload_hash%% *} expected_sha256=${expected_hash%% *}" ]] ||
     ! grep -Fxq "SEAM_EVIDENCE C20c_CHECK packet-and-fresh-admission PASS $binding" "$capture/seam.log"; then
    echo 'SEAM_EVIDENCE e2e_all scenario=C20c.client status=FAIL evidence=capture-binding-mismatch'
    return 1
  fi
  local proof
  if ! proof=$(E2E_ALL_CAPTURE_DIR="$capture" cargo +1.100.0 test --locked -p oteryn-client-ui --lib \
    death_protection::e2e_c20c::replays_real_death_protection_receipt_and_rejects_invalid_payload_unchanged \
    -- --exact); then
    printf '%s\n' "$proof"
    echo 'SEAM_EVIDENCE e2e_all scenario=C20c.client status=FAIL evidence=native-client-replay-failed'
    return 1
  fi
  printf '%s\n' "$proof"
  if ! grep -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' <<< "$proof"; then
    echo 'SEAM_EVIDENCE e2e_all scenario=C20c.client status=FAIL evidence=native-client-replay-not-executed'
    return 1
  fi
  if [[ -n ${E2E_ALL_CAPTURE_DIR:-} ]]; then
    local file
    for file in "${files[@]}"; do
      (set -o noclobber; cat "$capture/$file" > "$E2E_ALL_CAPTURE_DIR/$file") || return 1
    done
  fi
  echo "SEAM_EVIDENCE e2e_all scenario=C20c.client status=PASS evidence=twist-only-pvp-relog-and-native-hud-replay $binding"
}
