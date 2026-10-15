# MARKET-1 — world market (skeleton task record)

```yaml
task_id: MARKET-1
title: "MARKET-1: world market"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 row MARKET-1 (row 122); §7 'TRADE-PLAYER-1, MAIL-1, MARKET-1' (MARKET-CONTENT-1, INBOX-1b, MARKET-1, MARKET-WIRE-1)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/market-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

World Market: operations, offers, escrow, book lock, place/accept/cancel/expiry and matching, market event and every §8 delta (MARKET-1); marketable facts (MARKET-CONTENT-1); MARKET_V1 wire and Inbox view (MARKET-WIRE-1).

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

MARKET-CONTENT-1 (content) -> MARKET-1 (hard, persistence/economy/security) -> MARKET-WIRE-1 (Reg, after DEPOT-WIRE-1). INBOX-1b after DEPOT-1. MARKET-TIER-1 belongs to ITEM-FORGE-1.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING |
| NPC-ECON-1 (family entry gate) | BLOCKING (Wave 2, not merged) |
| PREMIUM-1 / PREM-WIRE-1 | MERGED (#2110) |
| ECON-RET-0 / MARKET-RET-0 | MERGED (#1748) |
| BANK-1 | MERGED (#1766) |
| GOLD-FEE-2 | MERGED |
| INBOX-1a | MERGED (#1785) |
| MARKET-CONTENT-1 (own child) | BLOCKING (not started) |
| DEPOT-1, DEPOT-WIRE-1 (INBOX-1b, MARKET-WIRE-1) | BLOCKING |
| acceptance of MARKET0 (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/domain/market.rs`
- `apps/game-server/src/durability/market.rs`
- `apps/game-server/src/durability/market_audit.rs`
- `apps/game-server/src/durability/market_tests.rs`
- `apps/game-server/src/gameplay_transport/market.rs`
- `apps/game-server/src/gameplay_transport/market_tests.rs`
- `crates/protocol-oteryn/src/market.rs`
- `crates/protocol-oteryn/src/market_tests.rs`
- `apps/game-server/tests/market_postgres.rs`
- content authoring inputs for `trade.marketable` / `trade.market_category` (source side only)
- `docs/agents/tasks/MARKET-1.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- tools/content-migration/regenerate_content.py outputs
- content/project.json
- content/manifest.json, content/world/manifest.json
- content/content.lock.json, content/world/content.lock.json
- content/interactions/index.json
- content/world/pins/*
- Rust pins (pinned registries / fixtures regenerated from content)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- durability/bank.rs + bank_audit.rs (BANK-1) for escrow debit/credit; reuse, do not fork
- durability/character_inbox.rs for delivery
- apps/game-server/src/content/item_market_true_promotion.rs (existing market flag promotion)
- connection.rs dispatch chain; capabilities.rs `GATED` (`MARKET_V1`)
- GAME_EVENT_FOUNDATION_REGISTRY.json (market event), RESOURCE_LIMITS_REGISTRY.json

## Inseparable bundles (never split across PRs)

- content (MARKET-CONTENT-1) + regeneration + pins by one author
- migration + market writer + durability tests
- escrow = bank debit + offer row + ledger in one transaction (mint/burn + ledger)
- wire + registry + capability + handler + tests (MARKET-WIRE-1)

## Acceptance criteria

- [ ] Accept of an offer moves gold and item atomically; double accept loses deterministically under the book lock.
- [ ] Expiry returns escrow to Inbox/bank exactly once after restart.
- [ ] Market fee burn uses `FeeBurnCause::MarketFee` (D238) and balance hard ceiling holds.
- [ ] Every delta of MARKET0 §8 has a durability test.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server market
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
python3 tools/content-migration/regenerate_content.py   # slot commit only; runs --check tools
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md §3-§10, §13
- apps/game-server/src/durability/bank.rs
- archive OTV2-20261004-gold-fee-2.md
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
