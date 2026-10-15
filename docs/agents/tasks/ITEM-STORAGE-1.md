# ITEM-STORAGE-1 — item storage: depot, supply stash, writable items (skeleton task record)

```yaml
task_id: ITEM-STORAGE-1
title: "ITEM-STORAGE-1: item storage: depot, supply stash, writable items"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row ITEM-STORAGE-1; §7 'ITEM-STORAGE-1' (DEPOT0, SUPPLY_STASH0, WRITE0 child graphs)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_DEPOT0_CHARACTER_DEPOT_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_SUPPLY_STASH0_SUPPLY_STASH_DECISION_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_WRITE0_BOOKS_SCROLLS_AND_BLACKBOARDS_DECISION_2026-10-01.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/item-storage-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Characters store items durably in a per-town depot and a supply stash, and read/write books, scrolls and blackboards, all through the ITEM-MOVE-2 transaction family.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

DEPOT-CONTENT-1 -> DEPOT-1 -> DEPOT-WIRE-1; STASH-CONTENT-1 -> STASH-1 -> STASH-WIRE-1; WRITE-CONTENT-1 -> WRITE-1 -> WRITE-MAP-1 -> WRITE-MOD-1. WRITE-WIRE-1 is excluded from this completion set.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING (Wave 1, not merged) |
| acceptance of DEPOT0 / SUPPLY_STASH0 / WRITE0 (plan §7 gate) | BLOCKING |
| HOUSE-CUSTODY-1 exclusivity guard (DEPOT-1) | MERGED record (archive house-custody-1), but plan §7 says the guard is not yet built: CP verifies; BLOCKING until then |
| ITEM-MOVE-2a / 2b (DEPOT-1, STASH-1, WRITE-1) | BLOCKING (ITEM-LOOP-1 children) |
| ITEM-VIEW-1b (DEPOT-WIRE-1) | MERGED (archive OTV2-20261003-item-view-1b) |
| MAP-LOAD-1, MAP-WIRE-1 (MAP-WIRE-2) | MERGED (archive MAP-LOAD-1, #1810) |
| WO-1, BANK-RET-0 | MERGED (#1748) |
| MARKET-CONTENT-1 (STASH-CONTENT-1) | BLOCKING (MARKET-1, Wave 4) |
| MAIL-TEXT-1, BAGS-1 (WRITE-1) | BLOCKING (no record on main) |
| HOUSE-RUNTIME-1a/1b/1c (WRITE-1 house items) | BLOCKING (Wave 3, not merged) |
| LEVER-1 (WRITE-MAP-1) | BLOCKING (no record on main) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/character_depot.rs`
- `apps/game-server/src/durability/character_depot_tests.rs`
- `apps/game-server/src/durability/character_supply_stash.rs`
- `apps/game-server/src/durability/character_supply_stash_tests.rs`
- `apps/game-server/src/durability/item_text.rs`
- `apps/game-server/src/durability/item_text_tests.rs`
- `apps/game-server/src/gameplay_transport/depot.rs`
- `apps/game-server/src/gameplay_transport/supply_stash.rs`
- `crates/protocol-oteryn/src/depot.rs`
- `crates/protocol-oteryn/src/supply_stash.rs`
- `apps/game-server/tests/item_storage_postgres.rs`
- `docs/agents/evidence/ITEM-STORAGE-1-*.md`
- `docs/agents/tasks/ITEM-STORAGE-1.md` (this record: status and notes only)

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

- connection.rs dispatch chain (DEPOT/STASH intents near NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` l.165 and offered sets l.76/104/132
- crates/protocol-oteryn/src/lib.rs `pub mod` l.2545-2568
- durability/mod.rs registration; HOUSE-CUSTODY-1 exclusivity guard call

## Inseparable bundles (never split across PRs)

- DEPOT-1 / STASH-1: migration + container writer + move transaction + audit
- WRITE-1: text payload limit (RESOURCE_LIMITS) + writer + tests
- each *-WIRE-1: wire + registry + capability + tests

## Acceptance criteria

- [ ] Depot and stash contents survive restart and are per Character per town as the briefs require.
- [ ] Every store/withdraw is one ITEM-MOVE-2 transaction; no item duplication under concurrent moves.
- [ ] Depot access excluded while the same item is under house custody.
- [ ] Written text respects the RESOURCE_LIMITS cap and is sanitized.
- [ ] Older clients receive no depot/stash domain.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server depot
cargo test -p oteryn-game-server stash
cargo test -p oteryn-game-server item_text
cargo test -p oteryn-game-server item_transfer
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_DEPOT0_CHARACTER_DEPOT_DECISION_2026-09-30.md
- docs/architecture/reviews/OTERYN_GAME_SUPPLY_STASH0_SUPPLY_STASH_DECISION_2026-10-03.md
- docs/architecture/reviews/OTERYN_GAME_WRITE0_BOOKS_SCROLLS_AND_BLACKBOARDS_DECISION_2026-10-01.md
- PRs #1810, #1748
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
