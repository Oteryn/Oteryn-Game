#!/usr/bin/env bash
set -euo pipefail
export CARGO_BUILD_JOBS=4 CARGO_NET_OFFLINE=true
quest_cargo=/home/mole/oteryn-cache/codex-bin/cargo
: > .questpc2/pick-exchange-verification.tsv
verify() {
    local name=$1
    shift
    if "$@" > ".questpc2/${name}.log" 2>&1; then
        printf '%s\tPASS\n' "$name" >> .questpc2/pick-exchange-verification.tsv
    else
        local result=$?
        printf '%s\tFAIL(%s)\n' "$name" "$result" >> .questpc2/pick-exchange-verification.tsv
        tail -25 ".questpc2/${name}.log"
        return "$result"
    fi
}
verify pick-exchange-pg-final "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib pick_exchange_atomic_confirmation_refusal_and_replay -- --nocapture --test-threads=1
verify pick-exchange-oriental-regression "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib oriental_trader_catalogue_ariki_sale_and_achievement_once -- --nocapture --test-threads=1
verify pick-exchange-clippy-final "$quest_cargo" clippy --locked -p oteryn-gameplay-content -p oteryn-gameplay-world -p oteryn-gameplay-npc -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch --all-targets --all-features --quiet -- -D warnings
verify pick-exchange-fmt-final "$quest_cargo" fmt --all --check
verify pick-exchange-mounted-fmt rustfmt --edition 2024 --check crates/gameplay/dispatch/src/gameplay_transport/npc_command/npc_item_exchanges.rs crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/pick_exchange.rs crates/gameplay/durability/src/durability/npc_trade/pick_exchange.rs crates/gameplay/durability/src/durability/npc_trade_audit/pick_exchange.rs
verify pick-exchange-whitespace git diff --check -- . ':(exclude).questpc2/**'
