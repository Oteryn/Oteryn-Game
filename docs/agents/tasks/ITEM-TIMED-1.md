# ITEM-TIMED-1 — timed items: charges, duration, repair (skeleton task record)

```yaml
task_id: ITEM-TIMED-1
title: "ITEM-TIMED-1: timed items: charges, duration, repair"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row ITEM-TIMED-1; §7 'ITEM-TIMED-1' (TIMED_ITEM0/0B child graph)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0_CHARGES_DURATION_AND_REPAIR_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0B_RUNTIME_CHARGES_AND_DURATION_DECISION_2026-10-03.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/item-timed-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Items with charges and duration behave end to end: runtime consumption on move/equip/use, house placement, condition effects, client wire, reference parity and NPC repair.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

TIMED-RT-1c (ITEM-MOVE-2a/2b) -> TIMED-HOUSE-1 (HOUSE-RUNTIME-1a) -> TIMED-FX-1 (COND-1) -> TIMED-WIRE-1 (Reg) -> TIMED-PARITY-1 -> TIMED-REPAIR-1 (NPC-TRADE-1 fee path). IMBUE/FORGE are Wave 4 and excluded.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING (Wave 1, not merged) |
| acceptance of TIMED_ITEM0 / TIMED_ITEM0B (plan §7 gate) | BLOCKING |
| TIMED-CONTENT-1, TIMED-RT-1a, TIMED-RT-1b | MERGED (#1699, #1681, archive timed-rt-1b) |
| ITEM-MOVE-2a / ITEM-MOVE-2b (TIMED-RT-1c) | BLOCKING (ITEM-LOOP-1 children; only the #2009 equip slice is on main) |
| ITEM-USE-1c | MERGED (#2082) |
| EQUIP-RT-1 | BLOCKING (no record on main) |
| COND-1 (TIMED-FX-1) | MERGED (MAGIC-COND-1 #2086, #2122) |
| NPC-TRADE-1 gold fee path (TIMED-REPAIR-1) | MERGED (#2040) |
| HOUSE-RUNTIME-1a house placement (TIMED-HOUSE-1) | BLOCKING (Wave 3, not merged) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/item_timed_state.rs`
- `apps/game-server/src/durability/item_timed_state_audit.rs`
- `apps/game-server/src/durability/item_timed_runtime.rs`
- `apps/game-server/src/durability/item_timed_runtime_tests.rs`
- `apps/game-server/src/durability/item_timed_repair.rs`
- `apps/game-server/src/gameplay_transport/item_timed.rs`
- `apps/game-server/src/gameplay_transport/item_timed_tests.rs`
- `crates/protocol-oteryn/src/item_timed.rs`
- `apps/game-server/tests/item_timed_postgres.rs`
- `docs/agents/evidence/ITEM-TIMED-1-*.md`
- `docs/agents/tasks/ITEM-TIMED-1.md` (this record: status and notes only)

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

- durability/item_timed_state.rs (existing, TIMED-RT-1a/1b) extension points
- connection.rs dispatch chain (NPC_TRADE_INTENT l.2766 for repair)
- capabilities.rs `GATED` l.165
- crates/protocol-oteryn/src/lib.rs `pub mod` l.2545-2568

## Inseparable bundles (never split across PRs)

- TIMED-RT-1c: charge/duration deltas inside the ITEM-MOVE-2 transaction
- TIMED-REPAIR-1: gold fee debit + item state restore + audit in one transaction
- TIMED-WIRE-1: wire + registry + capability + tests

## Acceptance criteria

- [ ] Charge/duration consumption is exactly-once across move, equip, use and restart.
- [ ] Expired items transition deterministically (tick-independent of client).
- [ ] Repair debits gold and restores state atomically; concurrent repair serializes.
- [ ] Placed house items keep their timed state.
- [ ] Parity evidence against reference values recorded.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server item_timed
cargo test -p oteryn-game-server timed
cargo test -p oteryn-game-server item_transfer
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0_CHARGES_DURATION_AND_REPAIR_DECISION_2026-10-01.md
- docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0B_RUNTIME_CHARGES_AND_DURATION_DECISION_2026-10-03.md
- PRs #1699, #1681, #2082, #2086, #2122, #2040
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
