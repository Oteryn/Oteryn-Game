# LOOP-GROUND-1 — ground loop: dropped item decay and cell limit (skeleton task record)

```yaml
task_id: LOOP-GROUND-1
title: "LOOP-GROUND-1: ground loop: dropped item decay and cell limit"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row LOOP-GROUND-1"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_DEATH_COMPLETE0_PLAYER_DEATH_MODULE_DECISION_2026-10-13.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/loop-ground-1
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Items on the ground (drops, corpses' spill) decay under one durable policy and each Ground cell enforces a fixed item limit, closing the ground loop after death batch B.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

Single row; split only after the decay-policy decision lands.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| DEATH-DUR-1 (death module batch B) | BLOCKING (not merged) |
| decision on dropped-item decay policy and Ground cell limit | BLOCKING (explicitly not decided: DEATH-COMPLETE-0 §6, remaining_unknowns; architect/owner decision needed) |
| acceptance of DEATH-COMPLETE-0 | BLOCKING (CANDIDATE) |
| MAP-OVERLAY-1a Ground rebuild | MERGED (#1800) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/item_decay_retire.rs`
- `apps/game-server/src/durability/item_decay_retire_audit.rs`
- `apps/game-server/src/durability/ground_cell_limit.rs`
- `apps/game-server/src/durability/ground_cell_limit_tests.rs`
- `apps/game-server/tests/ground_decay_postgres.rs`
- `docs/agents/evidence/LOOP-GROUND-1-*.md`
- `docs/agents/tasks/LOOP-GROUND-1.md` (this record: status and notes only)

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

- map/overlay.rs and map/overlay/pickup.rs (Ground overlay, MAP-OVERLAY-1a)
- durability/item_decay_retire.rs existing retire arms
- docs/contracts/RESOURCE_LIMITS_REGISTRY.json (cell limit)

## Inseparable bundles (never split across PRs)

- decay schedule + retire writer + audit
- cell limit (RESOURCE_LIMITS) + drop refusal + tests

## Acceptance criteria

- [ ] Dropped items decay on the decided schedule exactly once, including across restart.
- [ ] A drop beyond the cell limit is refused (or redirected per decision) with a typed code.
- [ ] Decay never removes items under custody or in a corpse still owned per DEATH-COMPLETE-0.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server ground
cargo test -p oteryn-game-server corpse_decay
cargo test -p oteryn-game-server item_decay
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_DEATH_COMPLETE0_PLAYER_DEATH_MODULE_DECISION_2026-10-13.md §6 (remaining_unknowns)
- PR #1800 (MAP-OVERLAY-1a)
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
