# OTV2-20260929-d3-5-corpse-pickup

```yaml
task_id: OTV2-20260929-d3-5-corpse-pickup
title: D3-5 Combat corpse-container pickup request wired to the D3-4 TRANSFER
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/d3-5-corpse-pickup
issue: 162
allocation: "#162 control plane child D3-5 of the merged D3 decision (worker 'Oteryn: sol combat lead')"
base_sha: d4d5e2c4
head_sha: f8554c270458cdc65962f03597daf7ed288b9974
final_head_sha: f8554c270458cdc65962f03597daf7ed288b9974
final_head_frozen_at: null
owner: "Oteryn: sol combat lead (claude-code-session-01U1WRHgL9X8RbuiG1pwczrF)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/combat/pickup.rs
  - apps/game-server/src/durability/item_transfer.rs                        # shared: one read-only method and its return enum
  - apps/game-server/src/durability/mod.rs                                  # shared: one linkage line
  - apps/game-server/tests/combat_pickup_postgres.rs
  - apps/game-server/tests/support/combat_pickup_postgres_cases.rs
  - apps/game-server/tests/support/corpse_transfer_postgres_cases.rs        # shared: helper visibility, loot definition parameter
  - docs/agents/tasks/active/OTV2-20260929-d3-5-corpse-pickup.md
public_contracts:
  - DUR-03
  - GAME-ITEM-01
depends_on: [D3-4]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

`combat_pickup` (B3-2) gains the corpse-container request variant of decision
`reviews/OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md` §4.5 (D134):

- `CorpsePickupRequest` has the `GroundPickupRequest` shape, but its source is named as
  `corpse_item_instance_id` + the entry's `source_item_instance_id`. `settle_corpse_pickup` resolves the
  facts from Content exactly as `settle_ground_pickup` does and commits through the D3-4 TRANSFER.
- Neither request trusts the source family it names. Both first read where the item is now
  (`DurabilityRoot::read_item_source_location`: Ground, or an entry of one live corpse). A live item at a
  different source is refused with `GroundPickupError::SourceMismatch` before any freeze: a Ground request for
  a corpse entry, a corpse request for a Ground item, or an entry of another corpse. An item that is no
  longer a live source falls through to the TRANSFER. A replay then finds its receipt, and a stale command gets
  `SourceNotOnGround`.
- The pre-read is stable: Ground items are never re-parented into a corpse, and corpse entries leave only by
  TRANSFER or decay. So a location read here can only become "gone", never another source.
- The D133 window (`CorpseExclusiveWindow`) and the corpse's own exclusion (`CorpseNotPickupable`) are not
  re-implemented. They come unchanged from the D3-4 admission and its database guards.

## Acceptance criteria

- [x] `cargo fmt`, `cargo clippy -p oteryn-game-server --lib --tests -D warnings`, `cargo test --lib`
  (949 passed) pass locally.
- [x] Local PostgreSQL 17.11:
  - `combat_pickup_postgres`: 626 passed, including the new
    `corpse_pickup_takes_only_from_the_named_corpse_within_the_window_rules`.
  - `corpse_transfer_postgres`: 597 passed.
  - `character_authority_postgres`: 628 passed. The 10 failures there are the `PostgreSQL 17.6` version
    pin only (local server is 17.11).
- [x] Protected PostgreSQL lane green on the exact frozen head.

## Deviations and gaps

- Two pre-existing B3-2 test defects fixed in `combat_pickup_postgres_cases.rs`, because they blocked running
  it: a 62-character fixture digest (must be 64), and a hyphen in a test schema name. Neither
  `combat_pickup_postgres` nor its cases are in the protected PostgreSQL lane, so they had never run in CI.
- `combat_pickup_postgres` now also includes `corpse_transfer_postgres_cases` for its corpse MINT and loot
  forgery helpers, so those 5 D3-4 cases also run in this local-only target.
- No production caller yet, like B3-2: GAME-INTERACTION dispatch is a later admission stage.

## Closeout

- merge commit/result: `7acab4d` on protected `main` (#1226); every file the PR changed is byte-identical to `f8554c2`
- ownership release: all leases released at merge
