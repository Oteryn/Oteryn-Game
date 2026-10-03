# OTV2-20261003-timed-content-1

```yaml
task_id: OTV2-20261003-timed-content-1
title: TIMED-CONTENT-1 Timed-item facts (charges, duration, timed transforms) for rings, amulets, boots, torches and lamps
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/timed-content-1
issue: 1622   # allocation D384
pr: PENDING_PR
owned_paths:
  - tools/content-schema/item-authoring/{lower_timed_item_packet.py,test_lower_timed_item_packet.py,README.md}
  - docs/agents/evidence/OTV2-20261003-timed-item-facts-v1.json
  - apps/game-server/src/content/{item_timed_promotion.rs,mod.rs}
  - apps/game-server/examples/materialize_content_world_project_v2.rs   # call site applied (D396)
  - apps/game-server/tests/content_world_project_repository.rs
  - content/** (regenerated)   # regenerated (D396)
  - docs/agents/tasks/archive/OTV2-20261003-timed-content-1.md
```

## Outcome

- DONE: `lower_timed_item_packet.py` lowers TibiaWiki `charges`/`duration` (first) and Canary `items.xml`
  (OTS_HYPOTHESIS_ONLY, fallback and the only transform source) into the pinned packet
  `OTV2-20261003-timed-item-facts-v1.json`: 276 rows on 111 Items (charges.count 24; temporal duration, mode
  and stop-duration 50 each; transform.decay 27, equip 37, unequip 38). Reported, not rows: 6 mode-undetermined
  (no equip pair and not a light source), 2 equip targets that are not timed, 1 wiki/Canary duration
  disagreement (wiki used), 22 `showcharges` values with no model field.
- DONE: `item_timed_promotion.rs` applies the packet strictly (closed enums, RL-01/RL-02 bounds, one row per
  field, existing Item and target only, never stackable, decay target never differs from
  `temporal.decay_target`, complete `temporal` group). `ON_EQUIP` maps to the authoritative active-time
  budget and `CONTINUOUS` to the durable absolute deadline; transforms fill the matching UseTransform kinds.
- Assumption: the inactive form of a pair carries only its `transform.equip` row, no `temporal` (TIMED-ITEM-0
  section 4); `stop_duration_while_unequipped` is true for `ON_EQUIP` forms.

## Re-pin receipt (D396)

- Merged `origin/main` (conflicts in `content/mod.rs` module list, `imports/canary/items-xml/.gitattributes`
  and the item-authoring README row only; resolved by keeping both module lists, `-text` per #1671 P2 and
  both README rows).
- Wired `apply_item_timed_promotion_v1(&mut draft)?` right after the stats promotion in
  `materialize_content_world_project_v2.rs`; regenerated with `tools/content-migration/regenerate_content.py`.
- Re-pins: none were needed. The old digest `82b086c4...` of the materializer example is not pinned by any
  file on current main (`rg` finds no occurrence); the navigation samples and TibiaWiki facts pin
  `item_identity.rs` (`dccd6090...`) and the sample files, and `materialize_content_world_project_v2.rs`
  appears there only as an `original_qualification_coordinate` path string. New example digest
  `fe0b0c21c3d78d2e9dc169b079ad557cd4d4d7ea72c1e0344b64f2676a7b5c8c`. `item_navigation_source_supplement.py`
  passes unchanged; no digest-only edit and no semantic edit to any pinned file. Receipt list: 0 files,
  old -> new: none.
- Regenerated (derived, by the tool): the Item definition shards, `content/items/taxonomy/items.json`, the
  index/lock files, `content/world/**` and the tree/Rust inventory pins in
  `content_world_project_repository.rs` (new `tree_sha256`
  `57784077f6b6bdada9ed49879aa24e728e26a6da61edf5633f7fa40bc83c067b`).
- Applier fix found on first run: an earlier stats promotion sets `temporal.duration` alone on the inactive
  form of a pair (for example i3049), so the completeness check now applies only to Items for which this packet
  has `temporal.*` rows. Test keys that dangled (`i1`, `i2`, `i9`) now use existing key `i3030`.
- Content now: 314 Items carry known `charges` or `temporal` (127 charges, 188 temporal, including earlier
  stats-sourced durations); the packet applies 276 rows on 111 Items.

## Blocked

- `show_count` has no Reference Item model field (stays in the packet until TIMED-WIRE-1), and the model's
  temporal modes are two, not three.

## Validation

`test_lower_timed_item_packet.py` and `lower_timed_item_packet.py --check` PASS; `validate_world_project_v2_to_tree.py`,
`test_world_project_v2_to_tree.py`, `validate_materialized_game_tree.py`, `item_key_references.py`,
`item_navigation_source_supplement.py`, `test_engine_items.py`, `item_weapon_proficiency.py --check` and
`g4_item_crystal_binding_generator.py --check` PASS; `cargo fmt --all --check` 0; clippy `-D warnings` 0;
`cargo test --locked -p oteryn-game-server` 17510 passed, 0 failed (exit 0). `validate_full_game_content_tree.py` fails
`SOURCE_ID_BOUNDARY_MISSING` on a tree-contract document this task does not touch.

## Rulings applied (D396, D397)

- D397: no reference-artifact codec change. Both `ON_EQUIP` and `CONTINUOUS` packet rows map to
  `AuthoritativeActiveTimeBudget` (TIMED-ITEM-0B §10.2: budget in a slot). A continuous form is the
  one with `stop_duration = false`; `show_count` stays in the packet until TIMED-WIRE-1.
- Lighting pairs (`transform {trigger: use}` unlit → lit) are not in Canary `items.xml` (Canary
  lights torches from scripts), so no use rows exist; ITEM-USE-0's lighting stays without content
  until a use pair is evidenced.
- D396: the materializer call site and the content-tree regeneration, with one mechanical re-pin of
  the navigation digests, wait for the content carrier (#1599/#1630) to merge; then merge main,
  re-pin, FREEZE.
- Carries #1671 P2 4174387005 (`-text` on the pinned Canary files).
