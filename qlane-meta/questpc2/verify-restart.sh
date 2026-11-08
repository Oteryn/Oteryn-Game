#!/usr/bin/env bash
set -euo pipefail
export CARGO_BUILD_JOBS=4
export CARGO_NET_OFFLINE=true
quest_cargo=/home/mole/oteryn-cache/codex-bin/cargo
: > .questpc2/restart-verification.tsv
verify() {
    local name=$1
    shift
    if "$@" > ".questpc2/${name}.log" 2>&1; then
        printf '%s\tPASS\n' "$name" | tee -a .questpc2/restart-verification.tsv
    else
        local result=$?
        printf '%s\tFAIL(%s)\n' "$name" "$result" | tee -a .questpc2/restart-verification.tsv
        tail -20 ".questpc2/${name}.log"
        return "$result"
    fi
}
verify restart-n-regression "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_n -- --nocapture --test-threads=1
verify restart-committed-selectors "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_committed_map_selectors_match_source -- --nocapture
verify restart-selector-binding "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_source_selectors_bind_exactly -- --nocapture
verify restart-world-facts "$quest_cargo" test --locked -p oteryn-gameplay-world --lib map::facts -- --nocapture
verify restart-content-claims "$quest_cargo" test --locked -p oteryn-gameplay-content --lib world_reward_claims -- --nocapture
verify restart-server-content "$quest_cargo" test --locked -p oteryn-game-server --test integration_rest content_ --quiet
verify restart-server-world "$quest_cargo" test --locked -p oteryn-game-server --test content_world_project_repository --quiet
verify restart-pin-check "$quest_cargo" run --locked --release -p oteryn-world-bundle-compiler -- pin-check . .questpc2/pin-check-phase-a
verify restart-diff-check git diff --check -- . ':(exclude).questpc2/**'
