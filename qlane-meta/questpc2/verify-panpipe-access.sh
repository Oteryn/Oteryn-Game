#!/usr/bin/env bash
set -euo pipefail
export CARGO_BUILD_JOBS=4
export CARGO_NET_OFFLINE=true
quest_cargo=/home/mole/oteryn-cache/codex-bin/cargo
: > .questpc2/panpipe-access-verification.tsv
verify() {
    local name=$1
    shift
    if "$@" > ".questpc2/${name}.log" 2>&1; then
        printf '%s\tPASS\n' "$name" >> .questpc2/panpipe-access-verification.tsv
    else
        local result=$?
        printf '%s\tFAIL(%s)\n' "$name" "$result" >> .questpc2/panpipe-access-verification.tsv
        tail -25 ".questpc2/${name}.log"
        return "$result"
    fi
}
verify panpipe-access-clippy "$quest_cargo" clippy --locked -p oteryn-gameplay-content -p oteryn-gameplay-world -p oteryn-gameplay-npc -p oteryn-gameplay-dispatch --all-targets --all-features --quiet -- -D warnings
verify panpipe-access-bag-regression "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_batch_one_panpipe_bag_exact_contents -- --nocapture
verify panpipe-access-naginata-regression "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_naginata_pick_six_shared_hits_and_replay -- --nocapture
verify panpipe-access-selectors "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_committed_map_selectors_match_source -- --nocapture
verify panpipe-access-bindings "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims_source_selectors_bind_exactly -- --nocapture
verify panpipe-access-world-facts "$quest_cargo" test --locked -p oteryn-gameplay-world --lib map::facts -- --nocapture
verify panpipe-access-content-claims "$quest_cargo" test --locked -p oteryn-gameplay-content --lib world_reward_claims -- --nocapture
verify panpipe-access-npc-dialogue "$quest_cargo" test --locked -p oteryn-gameplay-npc --lib npc_talk -- --nocapture
verify panpipe-access-oriental-regression "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib oriental_trader_catalogue_ariki_sale_and_achievement_once -- --nocapture
verify panpipe-access-server-content "$quest_cargo" test --locked -p oteryn-game-server --test integration_rest content_ --quiet
verify panpipe-access-server-world "$quest_cargo" test --locked -p oteryn-game-server --test content_world_project_repository --quiet
verify panpipe-access-pin-check "$quest_cargo" run --locked --release -p oteryn-world-bundle-compiler -- pin-check . .questpc2/pin-check-panpipe-access
verify panpipe-access-final-fmt "$quest_cargo" fmt --all --check
verify panpipe-access-fixture-fmt rustfmt --edition 2024 --check crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_reward_claims.rs
verify panpipe-access-final-whitespace git diff --check -- . ':(exclude).questpc2/**'
