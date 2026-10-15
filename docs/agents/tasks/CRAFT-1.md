# CRAFT-1 — item conversion crafting (skeleton task record)

```yaml
task_id: CRAFT-1
title: "CRAFT-1: item conversion crafting"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row CRAFT-1"
briefs:
  - (none on main; CRAFT brief required)
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/craft-1
needs_migration: true
needs_registry: false
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Use-with conversions (e.g. tools on items) produce output items through a data-driven conversion table, atomically and deterministically.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

Single row; children allocated after the architect writes the CRAFT brief.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| #2039 (ITEM-USE-1a+1b), ITEM-USE-1c | MERGED (#2039, #2082) |
| ITEM-LOOP-1 (family entry gate) | BLOCKING (Wave 1, not merged) |
| CRAFT brief / decision (conversion table, rates, RNG determinism) | BLOCKING (no brief on main; architect decision needed) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `content/craft/** (sources only)`
- `apps/game-server/src/durability/item_craft.rs`
- `apps/game-server/src/durability/item_craft_tests.rs`
- `apps/game-server/src/gameplay_transport/item_craft.rs`
- `apps/game-server/tests/item_craft_postgres.rs`
- `docs/agents/evidence/CRAFT-1-*.md`
- `docs/agents/tasks/CRAFT-1.md` (this record: status and notes only)

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

- ITEM-USE-1 use-with path (#2039/#2082) as the entry point
- durability/mod.rs registration

## Inseparable bundles (never split across PRs)

- migration + conversion writer (consume inputs + mint outputs + audit) in one transaction

## Acceptance criteria

- [ ] A conversion consumes inputs and mints outputs atomically or does nothing.
- [ ] Random outcomes are deterministic per the brief's RNG rule and reproducible in tests.
- [ ] Conversion table comes from content; unknown pairs refused with typed code.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server item_craft
cargo test -p oteryn-game-server item_use
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- PRs #2039, #2082
- plan §4 row CRAFT-1
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
