#!/usr/bin/env bash
set -euo pipefail
export CARGO_BUILD_JOBS=4
export CARGO_NET_OFFLINE=true
quest_cargo=/home/mole/oteryn-cache/codex-bin/cargo
: > .questpc2/panpipe-key-verification.tsv
verify() {
    local name=$1
    shift
    if "$@" > ".questpc2/${name}.log" 2>&1; then
        printf '%s\tPASS\n' "$name" | tee -a .questpc2/panpipe-key-verification.tsv
    else
        local result=$?
        printf '%s\tFAIL(%s)\n' "$name" "$result" | tee -a .questpc2/panpipe-key-verification.tsv
        tail -20 ".questpc2/${name}.log"
        return "$result"
    fi
}
verify panpipe-key-bag-regression "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_batch_one_panpipe_bag_exact_contents -- --nocapture --test-threads=1
verify panpipe-key-committed-selectors "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_committed_map_selectors_match_source -- --nocapture
verify panpipe-key-selector-binding "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_source_selectors_bind_exactly -- --nocapture
verify panpipe-key-world-facts "$quest_cargo" test --locked -p oteryn-gameplay-world --lib map::facts -- --nocapture
verify panpipe-key-content-claims "$quest_cargo" test --locked -p oteryn-gameplay-content --lib world_reward_claims -- --nocapture
verify panpipe-key-server-content "$quest_cargo" test --locked -p oteryn-game-server --test integration_rest content_ --quiet
verify panpipe-key-server-world "$quest_cargo" test --locked -p oteryn-game-server --test content_world_project_repository --quiet
verify panpipe-key-pin-check "$quest_cargo" run --locked --release -p oteryn-world-bundle-compiler -- pin-check . .questpc2/pin-check-panpipe-key
verify panpipe-key-fmt-check "$quest_cargo" fmt --all --check
verify panpipe-key-fixture-fmt rustfmt --edition 2024 --check crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_reward_claims.rs
verify panpipe-key-diff-check git diff --check -- . ':(exclude).questpc2/**'
