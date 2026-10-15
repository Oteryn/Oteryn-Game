# ADMIT-CAP-2 — player field effects behind world-required capabilities (skeleton task record)

```yaml
task_id: ADMIT-CAP-2
title: "ADMIT-CAP-2: player field effects behind world-required capabilities"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row ADMIT-CAP-2; §7 'ADMIT-CAP-1, ADMIT-CAP-2' (ADMIT0 §6)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_ADMIT0_WORLD_REQUIRED_CAPABILITIES_DECISION_2026-10-01.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/admit-cap-2
needs_migration: false
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Player-created spatial field effects (FIELD-2) apply in a scope only when that scope's effective capability set contains WORLD_SPATIAL_FIELDS and WORLD_INTERACTION_V1. The effective set is fixed per scope generation, so admission and field application never disagree within one generation.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

Single row (ADMIT0 §6 guard on top of ADMIT-CAP-1 world-required capabilities). No further children.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ADMIT-CAP-1 (family entry gate) | BLOCKING (Wave 1, not merged) |
| FIELD-2 (LOOP-WORLD-1 child) | BLOCKING (`blocked` in docs/agents/tasks/active/LOOP-WORLD-1.md) |
| acceptance of ADMIT0 (brief CANDIDATE; plan §7 gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/gameplay_transport/world_required_field_guard.rs`
- `apps/game-server/src/gameplay_transport/world_required_field_guard_tests.rs`
- `apps/game-server/tests/admit_cap_field_guard.rs`
- `docs/agents/tasks/ADMIT-CAP-2.md` (this record: status and notes only)

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

- capabilities.rs: `GATED` table l.165 and offered sets l.76/104/132 (WORLD_SPATIAL_FIELDS, WORLD_INTERACTION_V1 rows)
- field application call site introduced by FIELD-2 (guard hook; path fixed by CP after FIELD-2 merges)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json and OTERYN_GAME_ERROR_CODE_REGISTRY.json (refusal code)

## Inseparable bundles (never split across PRs)

- guard + effective-set snapshot per generation + refusal code + tests

## Acceptance criteria

- [ ] A player field effect in a scope whose effective set lacks WORLD_SPATIAL_FIELDS or WORLD_INTERACTION_V1 is refused with a typed code and leaves no durable trace.
- [ ] The effective set is read once per scope generation; a mid-generation capability change takes effect only at the next generation.
- [ ] Monster/system fields are unaffected by the guard.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server admit_cap
cargo test -p oteryn-game-server field
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_ADMIT0_WORLD_REQUIRED_CAPABILITIES_DECISION_2026-10-01.md §6
- docs/agents/tasks/active/LOOP-WORLD-1.md (FIELD-2 state)
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
