#!/usr/bin/env bash
set -euo pipefail
export CARGO_BUILD_JOBS=4 CARGO_NET_OFFLINE=true
quest_cargo=/home/mole/oteryn-cache/codex-bin/cargo
: > .questpc2/item-exchanges-verification.tsv
verify() {
    local name=$1
    shift
    if "$@" > ".questpc2/${name}.log" 2>&1; then
        printf '%s\tPASS\n' "$name" >> .questpc2/item-exchanges-verification.tsv
    else
        local result=$?
        printf '%s\tFAIL(%s)\n' "$name" "$result" >> .questpc2/item-exchanges-verification.tsv
        tail -25 ".questpc2/${name}.log"
        return "$result"
    fi
}
verify item-exchanges-durability-units "$quest_cargo" test --locked -p oteryn-gameplay-durability --lib durability::npc_trade::tests:: -- --nocapture --test-threads=1
verify item-exchanges-npc-units "$quest_cargo" test --locked -p oteryn-gameplay-npc --lib npc_talk -- --nocapture --test-threads=1
verify item-exchanges-pg-final "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib postgres_tests::pick_exchange:: -- --nocapture --test-threads=1
verify item-exchanges-oriental-regression "$quest_cargo" test --locked -p oteryn-gameplay-dispatch --lib oriental_trader_catalogue_ariki_sale_and_achievement_once -- --nocapture --test-threads=1
verify item-exchanges-server-check "$quest_cargo" check --locked -p oteryn-game-server --quiet
verify item-exchanges-clippy-final "$quest_cargo" clippy --locked -p oteryn-gameplay-content -p oteryn-gameplay-world -p oteryn-gameplay-npc -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch --all-targets --all-features --quiet -- -D warnings
verify item-exchanges-fmt-final "$quest_cargo" fmt --all --check
verify item-exchanges-mounted-fmt rustfmt --edition 2024 --check crates/gameplay/dispatch/src/gameplay_transport/npc_command/npc_item_exchanges.rs crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/pick_exchange.rs crates/gameplay/durability/src/durability/npc_trade/pick_exchange.rs crates/gameplay/durability/src/durability/npc_trade/obsidian_knife.rs crates/gameplay/durability/src/durability/npc_trade_audit/pick_exchange.rs crates/gameplay/durability/src/durability/npc_trade_audit/obsidian_knife.rs
verify item-exchanges-whitespace git diff --check -- . ':(exclude).questpc2/**'
