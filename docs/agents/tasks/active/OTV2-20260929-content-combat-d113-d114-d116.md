# OTV2-20260929-content-combat-d113-d114-d116

```yaml
task_id: OTV2-20260929-content-combat-d113-d114-d116
title: D113/D114 item semantics blocked on a missing owner-decision promotion mechanism; D116 rat spawn scoped out
mode: IMPLEMENT
status: blocked
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
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A first pass hand-edited `content/world/definitions/reference.json` directly and
recomputed its pinned digests. The control plane rejected that route: the canonical
package is materializer-owned and `g4-canonical-worldproject-package-seed` must stay
green. Per instruction, all 15 content files touched by that pass were reverted back to
byte-identical with the pre-edit tracked state (verified: `git diff HEAD~1` over
`content/` is empty after the revert) and this record now reports a **STOP**: no
existing mechanism accepts a plain owner-decided Item fact (`materializable`,
`stack_class`, `semantics.equipment`) for an ordinary opaque registry item.

## Mechanisms checked

- **Item semantic-promotion lowering v1** (#1155 `0d962e1`, `#1155`/`#1084`/`#1064`
  lineage; `apps/game-server/src/content/cw2_b1_import.rs`
  `protected_cw2_b1_item_semantic_promotion_lowering_v1_import` /
  `apply_item_semantic_promotion`, packet
  `docs/agents/evidence/OTV2-20260928-item-promotion-lowering-v1.json`). Confirmed by
  reading the packet: it already carries `container.capacity=20` and
  `presentation.name="backpack"` for `oteryn:item.registry.i00002752` (source_item_id
  2854) — this is where D114's "capacity 20 is already known" comes from — and **zero**
  rows for `oteryn:item.registry.i00005801`. It cannot carry D113/D114's remaining
  facts: (a) it is strictly Crystal-`items.xml`-sourced, with the whole packet validated
  against pinned source-repository/revision/artifact-digest lineage
  (`source_engine == "crystal"`, `zimbadev/crystalserver@ff7ede5…`) — an owner decision
  is not a Crystal fact and could not pass that lineage check; (b)
  `decode_item_semantic_promotion_value`/`apply_item_semantic_promotion` only decode 9
  field paths (`presentation.name`, `weapon.{attack,defense,extra_defense,hit_chance,range_cells}`,
  `protection.armor`, `charges.count`, `container.capacity`) — `equipment.patterns` is
  not one of them even though it is a listed destination in
  `CW2_B1_ITEM_FIELD_DISPOSITIONS`; (c) `apply_item_semantic_promotion`'s signature is
  `(&mut ReferenceItemSemantics, …)` — it cannot reach the enclosing record's
  `materializable`/`stack_class`, which are `ProjectReferenceRecord::Item` fields, not
  `semantics` fields.
- **TibiaWiki wave1 enrichment** (`apps/game-server/examples/materialize_content_world_project_v2.rs`
  `apply_wave1_fact`, evidence `ITEM_WAVE1_STAGED`): decodes exactly
  `weapon.weapon_type`, `imbuement.slot_count`, `stack.stackable` (only `false`),
  `trade_restrictions.marketable`. No `materializable`/`stack_class`/`container`/`equipment`
  case; same `&mut ReferenceItemSemantics`-only signature problem.
- **D11–D13 NPC offer-price decisions** (`#1159`/`#1124`, `populate_npcs`): a different
  family (`NPC`/`Service.Trade` offers), driven by wiki-majority price computation, not
  a route into `Item` `semantics`/`materializable`/`stack_class` at all.
- **`tools/content-schema/item-authoring/owner-item-family-decisions.json`**: a real
  owner ledger, but it records item-family *taxonomy* classification
  (`profile`/`reason`/wiki citation) consumed into `content/items/taxonomy/items.json`
  via `family_assignments`, not `Reference` semantics/`materializable`/`stack_class`.
- **Baseline allocation** (`protected_cw2_b1_full_item_family_import`,
  `cw2_b1_import.rs` ~L1643): every opaque `oteryn:item.registry.i*` record (38,093 of
  38,157) is permanently seeded `materializable: false, stack_class: Unknown`. Only the
  64-row `NATIVE_ITEM_BATCH` (hardcoded Rust constants, not a ledger) ever sets
  `materializable: true` with an explicit `stack_class` for a *named* item. No pass in
  `main()` (`materialize_content_world_project_v2.rs`) ever revisits an opaque item's
  `materializable`/`stack_class` after that baseline.

## Smallest extension point (not implemented)

**File:** `apps/game-server/src/content/cw2_b1_import.rs`
**Function:** a new function sibling to `apply_item_semantic_promotion` (defined next to
it, ~L2110), operating on `&mut ProjectReferenceRecord::Item { materializable,
stack_class, semantics, .. }` (the whole record, not just `semantics`) so it can also
flip `materializable`/`stack_class`, plus a new `equipment.patterns` case either there or
in `apply_item_semantic_promotion`. It would need its own pinned evidence packet
(distinct from the Crystal-sourced `ITEM_SEMANTIC_PROMOTION_LOWERING_V1_PACKET`),
keyed by `native_key` + `#162` comment id instead of Crystal `source_item_id`/lineage,
validated the same rigorous way (exact byte length + sha256, ordered/unique rows,
applied-partition counts) as `R7_P04_GOLD_COIN_EVIDENCE_PACKET`/the lowering packet, and
called from `main()` in `apps/game-server/examples/materialize_content_world_project_v2.rs`
after `populate_items`. This is reported only; it is not implemented.

## D116 (unchanged from the prior pass, still accepted as a gap)

2 rats outside the Movement proof path is scoped out. The only spawn representation is
`NativeEntrySpawn` in `apps/game-server/src/content/project/native_entry_room.json`
(`native_first_entry.spawn`) — a single, non-repeatable field whose room's cell set
(`native_entry.rs` `accepted::CELLS`/`DOOR_CELL`) is hardcoded and exactly validated; the
two `Walkable` cells (`entry-start`, `entry-east`) already are the proof path, and the
general `content/world/worlds/world.json` WorldPlacement tree is still empty
(`worlds: []`, `placements: []`).

## Validation

`git diff HEAD~1 -- content/` is empty (exact revert of the earlier hand-edit).
`tools/agents/validate_governance.py` and `git diff --check` pass. No content, tree, or
digest changes remain in this candidate; nothing else was run since there is no
substantive delta to validate this round.
