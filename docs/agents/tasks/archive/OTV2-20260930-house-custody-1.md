# OTV2-20260930-house-custody-1

```yaml
task_id: OTV2-20260930-house-custody-1
title: "HOUSE-CUSTODY-1 HouseInterior location, reclaim provenance and item location exclusivity guard (migration 0025)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
issue: 162
branch: claude/house-custody-1
pr: "exact PR in the #162 FREEZE_SHA entry"
base_sha: a6a054e
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session_019zdL6wTxV5CR69TUCoSPgZ (oteryn-hard-worker)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0025_house_item_custody.sql
  - apps/game-server/tests/house_custody_postgres.rs
  - apps/game-server/tests/support/house_custody_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs (one include, PG-COVERAGE-1)
  - docs/agents/tasks/archive/OTV2-20260930-house-custody-1.md
public_contracts: []
depends_on: [HOUSE-CUSTODY-0]
blocks: [house interior runtime, MAP-OVERLAY-1 reset preflight, HOUSE-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Allocation: #162 5916023254 (lease `0025`, still free on `main` `a6a054e`). One migration, storage
only, per the HOUSE-CUSTODY-0 implementation brief:

- `game_item_house_interior_locations` (§3.1): `HouseId` is `(world_id, house_key)` with the
  revision-free House content key (`oteryn:content.house.<slug>`, catalogue contract §2.1); native
  position bytes; `stack_ordinal` unique per `(world, house, position)`; no `ChannelId`. Rows are
  never updated in place (a move is delete and insert, as for Ground).
- `game_item_house_reclaim_provenance` (§3.2): primary key `item_instance_id` (at most one per
  item); first-slice subject `reclaim_subject_character_id` with an FK to `game_character_roots`
  `ON DELETE RESTRICT` (EXP-HOUSES-01 §15); a deferred FK to the location row of the same house
  and the same placement transaction, and a deferred constraint trigger for the other direction
  (row inserted); a provenance is deleted only when its item leaves the house. An UPDATE bumps the revision by one and names a new
  placement transaction, which the FK binds to the replacing row (a same-house move).
- `game_item_location_exclusive` / `game_item_location_exclusivity_guard` (§3.3): one deferred
  guard on `game_item_instances` and the five location tables. A live item has exactly one row,
  a retired item none, a house item no child entries. SECURITY DEFINER (the runtime role must not
  see the house tables), takes the item row lock `FOR NO KEY UPDATE` so concurrent placements of
  one item serialize (every existing writer already holds that lock from its item UPDATE). The
  migration checks every existing item once and fails closed.
- §3.5: no grant to `oteryn_game_runtime`; `oteryn_game_control` gets SELECT, as on every item
  table.

Not built (brief): scope kind, writer, transfer shapes, reset code, the tile-of-this-house check
against the active bundle, the ordinal allocation under the house lock. They need the bundle and
the house runtime (house interior runtime child, MAP-OVERLAY-1).

`game_character_progression_consistency_guard` is not touched (no Character write here), so the
0030 build arms are unaffected and CHAR-REV-SEQ-1 does not apply.

Migration number: sqlx applies a missing lower version after higher ones (`0026`, `0028`, `0030`
already landed with `0025`, `0027`, `0029` absent), and the ledger inspection compares by version
set, so `0025` needs no renumber.

## Validation

- `house_custody_postgres` (4 cases) and the CI-run targets `character_authority_postgres`,
  `durability_postgres`, `runtime_scope_assignment_postgres`, `native_admission_source_postgres`
  on a local PostgreSQL 17.11 (17.6 is not in the PGDG index; the 170006 assertion was patched
  in the working tree only, never committed). CI runs 17.6.
- Mutation checks, each red: the placement transaction is not bound (FK and trigger); a revision
  bump without a new placement; the guard ignores the house table; the row-without-provenance
  trigger fires on UPDATE instead of INSERT; the provenance-delete trigger fires on UPDATE; a
  SELECT grant to the runtime role.
- `cargo fmt --check`, `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`,
  `postgres_target_aggregation`, `validate_governance.py`.

## Review

- Codex P2 4149092379 on `d88a8a1f` (provenance not bound to the placement transaction):
  accepted and repaired in the next candidate (FK, trigger, update guard, three cases).
- Codex P2 4149559640 on `1c6de1de` (a delete and reinsert of the provenance bypasses the update
  guard): accepted and repaired in the next candidate. A provenance delete now fails at commit
  while its item still has a house row; two cases (same and other subject) and one mutation.
- Codex P2 4149964145 on `79ebae5c` (a replacing row reuses the old placement transaction with an
  unchanged provenance): accepted and repaired in the next candidate. The provenance carries a
  trigger-stamped `written_xact_id`; a new location row needs a provenance written in its own
  physical transaction. One case and one mutation.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. This record was archived in the PR's final
  authoring commit (`docs/agents/tasks/archive/README.md`).

## Acceptance criteria

- [ ] Exact frozen head with passing CI.
- [ ] Independent persistence review (control plane).
- [ ] Protected Merge Queue integration.
