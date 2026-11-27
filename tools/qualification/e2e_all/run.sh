#!/usr/bin/env bash
# Reuse node_boot's pinned PostgreSQL, Platform, node and real-bundle lifecycle.
# This command is deliberately red while SCENARIO_MATRIX.md has coverage gaps.
set -uo pipefail
umask 077

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd -- "$ROOT" || exit 1
RUN="$(mktemp -d "$ROOT/.e2e-all.XXXXXX")" || exit 1
readonly SERVER_CAPTURES="$RUN/server-captures"
readonly CLIENT_CAPTURES="$RUN/client-captures"
mkdir "$SERVER_CAPTURES" "$CLIENT_CAPTURES" || exit 1
readonly CARGO="${CARGO:-cargo}"
node_status=0
client_status=0

# Keep the fixture pass that node_boot composes before its real-world pass.
# Neither database bootstrap nor node lifecycle is duplicated here.
(
    export E2E_ALL_CAPTURE_DIR="$SERVER_CAPTURES"
    NODE_BOOT_WORLD=1 NODE_BOOT_SPELLS=0 bash tools/qualification/node_boot/run.sh
) 2>&1 | tee "$RUN/node.log" || node_status=$?

replay_suite() {
    local test_list="$RUN/$2.tests" ignored="$RUN/$2.ignored" output="$RUN/$2.log"
    local -a tests
    "$CARGO" test --offline --locked "$@" --lib -- --list > "$test_list" || return 1
    "$CARGO" test --offline --locked "$@" --lib -- --list --ignored > "$ignored" || return 1

    # Preserve the suite's normal selection, including its default exclusion of ignored tests.
    local selection
    selection="$(python3 "$ROOT/tools/qualification/e2e_all/select_replays.py" \
        "$RUN/node.log" "$CLIENT_CAPTURES" "$ignored" "$test_list")" || return 1
    mapfile -t tests < <(printf '%s' "$selection")
    if (( ${#tests[@]} == 0 )); then
        printf 'Replay suite %s selected zero tests\n' "$2" >&2
        return 1
    fi

    "$CARGO" test --offline --locked "$@" --lib -- "${tests[@]}" --exact 2>&1 |
        tee "$output" || return 1
    # libtest can exit successfully for an unmatched filter. Require every selected
    # test to have executed and reported success.
    if [[ "$(awk '/^test .* \.\.\. ok$/ { ++passed } END { print passed + 0 }' "$output")" != "${#tests[@]}" ]]; then
        printf 'Replay suite %s did not pass all %s selected tests\n' "$2" "${#tests[@]}" >&2
        return 1
    fi
}

# Both sides use the existing capture-directory interface. Copy the completed
# capture set before replay so a reader cannot change the original evidence.
(
    cp -a -- "$SERVER_CAPTURES/." "$CLIENT_CAPTURES/" || exit 1
    export E2E_ALL_CAPTURE_DIR="$CLIENT_CAPTURES"
    status=0
    replay_suite -p oteryn-client-runtime || status=1
    replay_suite -p oteryn-client-domain || status=1
    replay_suite -p oteryn-client-ui || status=1
    replay_suite -p oteryn-client || status=1
    replay_suite -p oteryn-session || status=1
    replay_suite -p oteryn-dev-client || status=1
    python3 "$ROOT/tools/qualification/e2e_all/select_replays.py" --verify-passes \
        "$RUN"/oteryn-*.log || status=1
    exit "$status"
) 2>&1 | tee "$RUN/client.log" || client_status=$?

python3 tools/qualification/e2e_all/report.py \
    --matrix apps/game-server/src/qualification/SCENARIO_MATRIX.md \
    --log "$RUN/node.log" --node-status "$node_status" --client-status "$client_status"
result=$?
printf 'Evidence: %s\n' "$RUN"
exit "$result"
