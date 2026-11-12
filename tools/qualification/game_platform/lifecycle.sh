#!/usr/bin/env bash
# Sourced only by node_boot's fixed-path lifecycle hook. No Platform boot copy.
set +x
readonly GAME_PLATFORM_SHA=ed9b33729865a77051566182a17aaf8577abf347
GP_HARNESS_PID=''
GP_PG_CONTAINER=''
game_platform_retain_test_logs() {
  [[ -n "${NODE_BOOT_FAILURE_LOG_DIR:-}" && -n "${WORK:-}" ]] || return 0
  local log
  for log in identity-build.log identity.log native_ticket_and_gateway_fail_closed.log native_device_rotation_replay.log; do
    [[ -f "$WORK/$log" ]] || continue
    python3 "$GAME_SOURCE/tools/qualification/game_platform/redact.py" \
      "$WORK/$log" "$NODE_BOOT_FAILURE_LOG_DIR/$log" || return 1
  done
}
game_platform_fail() {
  game_platform_retain_test_logs || set -- "$1" diagnostic_redaction_failed
  if [[ -n "${GAME_PLATFORM_RESULT_FILE:-}" ]]; then
    printf 'GAME_PLATFORM_RESULT=FAIL flow=%s reason=%s\n' "$1" "$2" > "$GAME_PLATFORM_RESULT_FILE"
  else
    printf 'GAME_PLATFORM_RESULT=FAIL flow=%s reason=%s\n' "$1" "$2"
  fi
  exit 1
}
game_platform_validate() {
  [[ "$(git -C "${PLATFORM_SOURCE:-}" rev-parse HEAD 2>/dev/null)" == "$GAME_PLATFORM_SHA" ]] \
    || game_platform_fail preflight wrong_platform_sha
  [[ -n "${GAME_PLATFORM_RESULT_FILE:-}" && "${NODE_BOOT_WORLD:-1}" == 1 \
    && "${NODE_BOOT_SPELLS:-0}" == 0 && "${NODE_BOOT_KEEP_STACK:-0}" == 0 \
    && "${NODE_BOOT_REUSE_STACK:-0}" == 0 ]] || game_platform_fail preflight unsupported_lifecycle_mode
}
game_platform_cleanup() {
  if [[ -n "$GP_HARNESS_PID" ]]; then
    kill -TERM -- "-$GP_HARNESS_PID" 2>/dev/null || true
    local deadline=$((SECONDS + 15))
    while kill -0 "$GP_HARNESS_PID" 2>/dev/null && (( SECONDS < deadline )); do sleep 0.2; done
    kill -KILL -- "-$GP_HARNESS_PID" 2>/dev/null || true
    wait "$GP_HARNESS_PID" 2>/dev/null || true
  fi
  # Also covers unexpected exits/timeouts before an explicit fail call.
  game_platform_retain_test_logs || true
  # Preserve private logs before node_boot removes its work directory.
  if [[ -n "${NODE_BOOT_FAILURE_LOG_DIR:-}" ]]; then
    [[ ! -f "$node_log" ]] || cp "$node_log" "$NODE_BOOT_FAILURE_LOG_DIR/node.log"
    [[ ! -f "$WORK/portal-harness.log" ]] || cp "$WORK/portal-harness.log" "$NODE_BOOT_FAILURE_LOG_DIR/portal-harness.log"
  fi
  [[ -z "$GP_PG_CONTAINER" ]] || docker rm --force --volumes "$GP_PG_CONTAINER" >/dev/null 2>&1 || true
}
game_platform_start() {
  local password port deadline ready=0
  password="$(openssl rand -hex 32)"
  port="${GAME_PLATFORM_PLATFORM_PG_PORT:-15436}"
  [[ "$port" =~ ^[1-9][0-9]{3,4}$ && "$port" -le 65535 && "$port" != "$PG_PORT" ]] \
    || game_platform_fail preflight invalid_platform_pg_port
  GP_PG_CONTAINER="$WP5_PROJECT-portal-postgres"
  docker run --detach --name "$GP_PG_CONTAINER" --publish "127.0.0.1:$port:5432" \
    --env POSTGRES_USER=game_platform_admin --env POSTGRES_PASSWORD="$password" \
    --env POSTGRES_DB=postgres "$PG_IMAGE" >/dev/null
  deadline=$((SECONDS + 60))
  until docker exec "$GP_PG_CONTAINER" pg_isready -h 127.0.0.1 -U game_platform_admin >/dev/null 2>&1; do
    (( SECONDS < deadline )) || game_platform_fail platform postgres_readiness_timeout
    sleep 0.2
  done
  [[ "$(docker exec "$GP_PG_CONTAINER" psql -U game_platform_admin -d postgres -qAt -c 'SHOW server_version_num')" == 180006 ]] \
    || game_platform_fail platform wrong_postgres_version
  export PORTAL_E2E_ADMIN_DATABASE_URL="postgresql://game_platform_admin:$password@127.0.0.1:$port/postgres"
  export PORTAL_E2E_OUT="$WORK/portal"
  # The harness intentionally forwards only test.env names. A private exec-only
  # launcher supplies identity/OAuth configuration absent from that whitelist;
  # every migrate/seed/serve/worker/gateway invocation is the real built binary.
  cargo build --locked --manifest-path "$PLATFORM_SOURCE/portal-rs/Cargo.toml" -p oteryn-portal > "$WORK/portal-build.log" 2>&1 \
    || game_platform_fail platform binary_build_failed
  cargo metadata --manifest-path "$PLATFORM_SOURCE/portal-rs/Cargo.toml" --no-deps --format-version 1 > "$WORK/portal-meta.json"
  local prepare_code=0
  python3 "$GAME_SOURCE/tools/qualification/game_platform/prepare.py" "$WORK" "$ACCOUNT_ID" "$SECOND_ACCOUNT_ID" \
    "$WORLD_ID" "$CHANNEL_ID" "$FRESH_PUBLIC_HEX" "$RECOVERY_PUBLIC_HEX" "$FRESH_KEY_ID" "$RECOVERY_KEY_ID" \
    || prepare_code=$?
  [[ "$prepare_code" != 3 ]] || game_platform_fail platform real_portal_binary_missing
  [[ "$prepare_code" == 0 ]] || game_platform_fail platform configuration_failed
  export PORTAL_E2E_BINARY="$WORK/portal-exec"
  export PORTAL_E2E_HTTP_PORT="${GAME_PLATFORM_HTTP_PORT:-18080}"
  export PORTAL_E2E_INTERNAL_PORT="${GAME_PLATFORM_INTERNAL_PORT:-18543}"
  export PORTAL_E2E_ADMISSION_PORT="${GAME_PLATFORM_ADMISSION_PORT:-18091}"
  export PORTAL_E2E_GATEWAY_PORT="${GAME_PLATFORM_GATEWAY_PORT:-18090}"
  export OTERYN_PUBLIC_ORIGIN="http://127.0.0.1:$PORTAL_E2E_HTTP_PORT"
  export PORTAL_E2E_NATIVE_SCOPES="[\"$WORLD_ID/$CHANNEL_ID\"]"
  setsid bash "$PLATFORM_SOURCE/portal-rs/e2e/start.sh" --seed-plan "$WORK/seed-plan.json" \
    > "$WORK/portal-report.json" 2> "$WORK/portal-harness.log" &
  GP_HARNESS_PID=$!
  deadline=$((SECONDS + 180))
  while kill -0 "$GP_HARNESS_PID" 2>/dev/null; do
    if python3 "$GAME_SOURCE/tools/qualification/game_platform/report.py" "$WORK/portal-report.json" "$WORK" >/dev/null 2>&1; then
      ready=1; break
    fi
    (( SECONDS < deadline )) || break
    sleep 0.2
  done
  [[ "$ready" == 1 ]] || game_platform_fail platform harness_not_ready
  python3 "$GAME_SOURCE/tools/qualification/game_platform/route_check.py" "$WORK" \
    || game_platform_fail platform route_seed_conflict_failed
  evidence "flow=route_seed result=pass revision_format=1 idempotent=1 conflict_exit=2"
  # node_boot's unassigned identity phase needs only NativeEvidence credentials.
  # Do not combine the harness's other fifteen purpose identities with this leaf.
  cp "$PORTAL_E2E_OUT/certs/ca.pem" "$WP5_PKI/server-ca.crt"
  cp "$PORTAL_E2E_OUT/certs/clients/native-evidence.pem" "$WP5_PKI/client.crt"
  cp "$PORTAL_E2E_OUT/certs/clients/native-evidence.key" "$WP5_PKI/client.key"
  WP5_PORT="$PORTAL_E2E_INTERNAL_PORT"
  evidence "platform_roles=migrate,serve,worker,gateway platform_pg=18.6 game_pg=18.6 separate_database_roles=1 clients=16"
}
game_platform_config() {
  python3 - "$1" <<'PY'
import pathlib, sys
p = pathlib.Path(sys.argv[1])
s = p.read_text().replace('peer_name = "source.test"', 'peer_name = "localhost"')
s = s.replace('source_authority = "platform"', 'source_authority = "oteryn-platform-e2e"')
p.write_text(s)
PY
}
game_platform_identity() {
  result=NODE_BOOT_IDENTITY_ONLY
  ACTIVE_STAGE=web_identity
  # Compile before the node's bounded assignment wait and journey budget begin.
  cargo test --locked -p oteryn-platform-client --test web_identity_pkce_isolation \
    --test native_ticket_and_gateway_fail_closed --test native_device_rotation_replay --no-run \
    > "$WORK/identity-build.log" 2>&1 || game_platform_fail web_identity driver_build_failed
  stage operator_setup -- operator_setup
  [[ ${STATUS[operator_setup]} == PASS ]] || game_platform_fail game operator_setup_failed
  start_node
  await_log awaiting_assignment 120
  evidence "game_platform_provenance game=$(git rev-parse HEAD) platform=$GAME_PLATFORM_SHA game_pg=18.6 platform_pg=18.6 bundle=$WORLD_DIGEST"
  GAME_PLATFORM_INPUT="$WORK/identity.json" GAME_PLATFORM_REPORT="$WORK/portal-report.json" \
    GAME_PLATFORM_GAME_DATABASE_URL="$ADMIN_URL" \
    timeout --kill-after=1s 60s cargo test --locked -p oteryn-platform-client --test web_identity_pkce_isolation \
      web_identity_pkce_isolation -- --exact --nocapture > "$WORK/identity.log" 2>&1 \
    || game_platform_fail web_identity journey_assertion_failed
  kill -0 "$NODE_PID" 2>/dev/null || game_platform_fail game node_exited_during_identity
  grep -Fx 'NODE_BOOT_EVIDENCE flow=web_identity result=pass account_isolation=1' "$WORK/identity.log" \
    || game_platform_fail web_identity evidence_missing
  local target budget
  for target in native_ticket_and_gateway_fail_closed native_device_rotation_replay; do
    ACTIVE_STAGE="$target"
    budget=60
    [[ "$target" != native_device_rotation_replay ]] || budget=120
    GAME_PLATFORM_INPUT="$WORK/identity.json" GAME_PLATFORM_REPORT="$WORK/portal-report.json" \
      GAME_PLATFORM_GAME_DATABASE_URL="$ADMIN_URL" LOCALAPPDATA="$WORK/native-client" \
      timeout --kill-after=1s "${budget}s" cargo test --locked -p oteryn-platform-client --test "$target" \
        "$target" -- --exact --nocapture > "$WORK/$target.log" 2>&1 \
      || game_platform_fail "$target" journey_assertion_failed
    kill -0 "$NODE_PID" 2>/dev/null || game_platform_fail game node_exited_during_native
    if [[ "$target" == native_ticket_and_gateway_fail_closed ]]; then
      grep -Fx 'NODE_BOOT_EVIDENCE flow=native_ticket_gateway result=fail_closed game_admissions=0' "$WORK/$target.log" \
        || game_platform_fail "$target" evidence_missing
    elif grep -q '^GAME_PLATFORM_EVIDENCE blocker=portal-rs/crates/gameauth/src/devices_http.rs:' "$WORK/$target.log"; then
      grep '^GAME_PLATFORM_EVIDENCE blocker=portal-rs/crates/gameauth/src/devices_http.rs:' "$WORK/$target.log"
      grep -Eq '^GAME_PLATFORM_RESULT=BLOCKED owner=Platform what=portal-rs/crates/gameauth/src/devices_http.rs:(224|199)$' "$GAME_PLATFORM_RESULT_FILE" \
        || game_platform_fail "$target" blocker_result_missing
    else
      grep -E '^NODE_BOOT_EVIDENCE flow=native_device_rotation result=pass rotations=[1-9][0-9]* replay_refused=1 signout_offline=1$' "$WORK/$target.log" \
        || game_platform_fail "$target" evidence_missing
    fi
  done
  evidence "blocker=portal-rs/crates/gameauth/src/admission/grant.rs:24 pending=encrypted_signing_custody"
  # Assignment requires control-owned scope grants. The default node_boot stage
  # inserts them directly; this lane must not invent those authority records.
  printf 'GAME_PLATFORM_EVIDENCE blocker=tools/qualification/node_boot/run.sh:999 pending=control_scope_grant_issuer\n'
  # Keep FAIL or an earlier Platform BLOCKED; BLOCKED supersedes PASS.
  if [[ ! -f "$GAME_PLATFORM_RESULT_FILE" ]] || grep -Fxq 'GAME_PLATFORM_RESULT=PASS' "$GAME_PLATFORM_RESULT_FILE"; then
    printf 'GAME_PLATFORM_RESULT=BLOCKED owner=Game what=tools/qualification/node_boot/run.sh:999\n' > "$GAME_PLATFORM_RESULT_FILE"
  fi
  exit 2
}
