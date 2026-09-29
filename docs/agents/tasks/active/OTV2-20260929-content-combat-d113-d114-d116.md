# OTV2-20260929-content-combat-d113-d114-d116

```yaml
task_id: OTV2-20260929-content-combat-d113-d114-d116
title: D113/D114 item semantics (rat corpse, backpack); D116 rat spawn scoped out as a schema gap
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/content-combat-d113-d114-d116
issue: 162
allocation_comment: "#162 comments 5879404970, 5884460826, 5880825291"
base_sha: b35bad9dd5aa4ad9c86623b4295c54ee1d7bf695
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: worker "Oteryn: impl content", #162 control plane
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260929-content-combat-d113-d114-d116.md
  - content/world/definitions/reference.json
  - content/world/manifest.json
  - content/world/project.json
  - content/world/content.lock.json
  - content/items/definitions/items-02500-02999.json
  - content/items/definitions/items-05500-05999.json
  - content/items/index.json
  - content/content.lock.json
  - content/abilities/definitions/index.json
  - content/abilities/effects/index.json
  - content/abilities/formulas/index.json
  - content/behaviors/index.json
  - content/creatures/definitions/index.json
  - content/loot/index.json
  - content/presentations/definitions/index.json
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Product delta

**D114** (backpack `oteryn:item.registry.i00002752`): `semantics.equipment` KNOWN with
one pattern (`pattern_id` 1, `primary_slot` KNOWN(`CONTAINER`), every other pattern field
UNKNOWN); `stack_class` -> `NonStackable`; `materializable` -> `true`. Container capacity
20 was already KNOWN and is unchanged.

**D113** (rat corpse `oteryn:item.registry.i00005801`): `materializable` -> `true`,
`stack_class` -> `NonStackable`, `semantics.container` KNOWN capacity 16 (D3 decision
#1198, `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX = 16`); no equipment pattern; every other
semantics field UNKNOWN. The 60 s corpse decay was checked against the schema: the
`semantics.temporal` group exists but its `decay_target` is a required transform target
(item-to-item), which does not model a plain timed despawn and no target item was
supplied by the decision, so it was left UNKNOWN rather than populated with an invented
target.

**D116** (2 rats in the starting room, outside the accepted Movement proof path): out of
scope, reported as a gap. The only spawn representation in the tree is
`NativeEntrySpawn` inside `apps/game-server/src/content/project/native_entry_room.json`
(`native_first_entry.spawn`) — a single, non-repeatable field. `native_entry.rs` validates
an exact, hardcoded cell set (`accepted::CELLS`/`DOOR_CELL`) for that same room, where the
only two `Walkable` cells (`entry-start`, `entry-east`) already form the Movement proof
path, `entry-north` is `Blocked` by design, and `entry-door` is gated behind the door. The
broader `content/world/worlds/world.json` (`OTERYN_WORLD_PROJECT_WORLDS/v2`) is still
empty (`worlds: []`, `placements: []`) — no general WorldPlacement/spawn population
exists elsewhere to attach a second spawn to. Adding a second rat or new walkable cells
would require inventing/extending both the JSON schema and its Rust validator, which is
out of the smallest-sufficient scope for this task. D113/D114 ship without it.

## Tooling used for the pinned digests

`tools/content-migration/world_project_v2_to_tree.py` regenerates the successor
`content/items/**` shard tree and family indices from `content/world/definitions/reference.json`.
No repository tool regenerates `content/world/manifest.json` / `project.json` /
`content.lock.json` from a live-edited `reference.json` (the seed example
`apps/game-server/examples/materialize_content_world_project_v2.rs` rebuilds the whole
WorldProject package from fixed, embedded, sha256-pinned evidence snapshots, not from the
tracked file, and was not touched by this task). Its digest formula was recovered from
`apps/game-server/src/content/production.rs` (`PackageManifestBinding::provenance_preimage`,
a big-endian-u32-length-prefixed concatenation of `package_key`, `package_revision`,
`semantic_schema_version`, `licensing_metadata`, `source_manifest_digest`, sha256'd) and
`apps/game-server/src/content/project.rs`/`project/v2.rs` (`source_manifest_digest` = raw
sha256 of `manifest.json`'s own bytes; `project.json`'s `manifest_sha256`/
`content_lock_sha256` are raw sha256 of `manifest.json`/`content.lock.json`'s own bytes).
The formula was verified by reproducing the pre-edit committed digests byte-for-byte
before use, then applied by a small local script to recompute the four pinned digests
after the `reference.json` edit. No digest was hand-typed.

## Known CI gap

`g4-canonical-worldproject-package-seed.yml` (`content/world/**`) materializes the
WorldProject package twice from `materialize_content_world_project_v2` and diffs it
against the tracked `content/world/` (minus the 10 successor-tree directory markers).
That materializer reconstructs the pre-edit `reference.json`/`manifest.json`/
`project.json`/`content.lock.json` byte-for-byte (verified locally: `diff` against a
freshly materialized package shows exactly those 4 files differing, and within
`reference.json` exactly the 2 edited item records differ) because it does not know
about this candidate's 2 field edits, which were authored directly rather than through a
new admission/evidence batch. This candidate's `g4-canonical-worldproject-package-seed`
run is therefore expected to fail its "Compare tracked package" step unless the control
plane accepts a direct edit here, or a follow-up teaches the materializer example about
it. `content-tree-migration.yml`'s two validators, `validate_materialized_game_tree.py`,
`cargo test --lib content`, and the three `content_world_project_*`/`content_world_cw2_b1_import`
integration tests were run locally and pass.

## Validation and custody

One writer on one exclusively allocated branch, direct hand-edit + digest recompute,
canonical compact-JSON serialization preserved and round-trip verified before editing.
`tools/content-migration/{test,validate}_world_project_v2_to_tree.py`,
`tools/content-schema/validate_materialized_game_tree.py`,
`tools/agents/validate_governance.py`, `git diff --check`, `cargo test --lib content`,
and the `content_world_project_repository`/`content_world_project_v2`/
`content_world_cw2_b1_import` integration tests all pass locally. No PR opened per the
control-plane task; branch pushed for the coordinator to pick up.
