#!/usr/bin/env bash
set -euo pipefail
export CARGO_BUILD_JOBS=4
export CARGO_NET_OFFLINE=true
quest_cargo=/home/mole/oteryn-cache/codex-bin/cargo
: > .questpc2/verification-results.tsv
verify() {
    local name=$1
    shift
    if "$@" > ".questpc2/${name}.log" 2>&1; then
        printf '%s\tPASS\n' "$name" | tee -a .questpc2/verification-results.tsv
    else
        local result=$?
        printf '%s\tFAIL(%s)\n' "$name" "$result" | tee -a .questpc2/verification-results.tsv
        tail -25 ".questpc2/${name}.log"
        return "$result"
    fi
}
verify qualified-fmt "$quest_cargo" fmt --all
verify qualified-fixture-fmt rustfmt --edition 2024 --check crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_reward_claims.rs
verify qualified-pin-refresh "$quest_cargo" run --locked --release -p oteryn-world-bundle-compiler -- pin-refresh .
verify qualified-clippy "$quest_cargo" clippy --locked -p oteryn-gameplay-world -p oteryn-gameplay-dispatch --all-targets --all-features --quiet -- -D warnings
verify qualified-questpc2 "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib quest_reward_claims -- --nocapture --test-threads=1
verify qualified-world-unit "$quest_cargo" test --locked -p oteryn-gameplay-world --lib map::facts -- --nocapture
verify qualified-content-unit "$quest_cargo" test --locked -p oteryn-gameplay-content --lib world_reward_claims -- --nocapture
verify qualified-server-content "$quest_cargo" test --locked -p oteryn-game-server --test integration_rest content_ --quiet
verify qualified-server-world "$quest_cargo" test --locked -p oteryn-game-server --test content_world_project_repository --quiet
verify qualified-pin-check "$quest_cargo" run --locked --release -p oteryn-world-bundle-compiler -- pin-check . .questpc2/pin-check-qualified
verify qualified-fmt-check "$quest_cargo" fmt --all --check
verify qualified-diff-check git diff --check
