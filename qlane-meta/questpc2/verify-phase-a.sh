#!/usr/bin/env bash
set -uo pipefail
export CARGO_BUILD_JOBS=4
export CARGO_NET_OFFLINE=true
quest_cargo=/home/mole/oteryn-cache/codex-bin/cargo
failed=0
: > .questpc2/phase-a-verification.tsv
verify() {
    local name=$1
    shift
    if "$@" > ".questpc2/${name}.log" 2>&1; then
        printf '%s\tPASS\n' "$name" | tee -a .questpc2/phase-a-verification.tsv
    else
        local result=$?
        printf '%s\tFAIL(%s)\n' "$name" "$result" | tee -a .questpc2/phase-a-verification.tsv
        tail -20 ".questpc2/${name}.log"
        failed=1
    fi
}
verify phase-a-fmt-check "$quest_cargo" fmt --all --check
verify phase-a-fixture-fmt-check rustfmt --edition 2024 --check crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_reward_claims.rs
verify phase-a-clippy "$quest_cargo" clippy --locked -p oteryn-gameplay-world -p oteryn-gameplay-dispatch --all-targets --all-features --quiet -- -D warnings
verify phase-a-committed-selectors "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_committed_map_selectors_match_source -- --nocapture
verify phase-a-selector-binding "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_source_selectors_bind_exactly -- --nocapture
verify phase-a-n-regression "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_n -- --nocapture --test-threads=1
verify phase-a-world-facts "$quest_cargo" test --locked -p oteryn-gameplay-world --lib map::facts -- --nocapture
verify phase-a-content-claims "$quest_cargo" test --locked -p oteryn-gameplay-content --lib world_reward_claims -- --nocapture
verify phase-a-server-content "$quest_cargo" test --locked -p oteryn-game-server --test integration_rest content_ --quiet
verify phase-a-server-world "$quest_cargo" test --locked -p oteryn-game-server --test content_world_project_repository --quiet
verify phase-a-pin-check "$quest_cargo" run --locked --release -p oteryn-world-bundle-compiler -- pin-check . .questpc2/pin-check-phase-a
verify phase-a-diff-check git diff --check -- . ':(exclude).questpc2/**'
exit "$failed"
