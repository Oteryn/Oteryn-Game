# OTV2-20261003-timed-content-1

```yaml
task_id: OTV2-20261003-timed-content-1
title: TIMED-CONTENT-1 Timed-item facts (charges, duration, timed transforms) for rings, amulets, boots, torches and lamps
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: claude/canary-items-xml-pin
branch: claude/timed-content-1
issue: 1622   # allocation D384
pr: PENDING_PR
owned_paths:
  - tools/content-schema/item-authoring/{lower_timed_item_packet.py,test_lower_timed_item_packet.py,README.md}
  - docs/agents/evidence/OTV2-20261003-timed-item-facts-v1.json
  - apps/game-server/src/content/{item_timed_promotion.rs,mod.rs}
  - apps/game-server/examples/materialize_content_world_project_v2.rs   # call site: NOT applied, see Blocked
  - apps/game-server/tests/content_world_project_repository.rs
  - content/** (regenerated)   # NOT regenerated, see Blocked
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

## Blocked

- The materializer call site is not applied and the content tree is not regenerated. The digest of
  `apps/game-server/examples/materialize_content_world_project_v2.rs` is pinned by
  `tools/content-migration/samples/{navigation-seven-20261001,engine-family-navigation-265,official-rule-only-navigation-six}.json`,
  `imports/tibiawiki/facts/items-{family-alias26-20261001,bounded7-navigation-20261002}.json`,
  `content/items/taxonomy/items.json` and a world-object sample; editing the example fails
  `item_navigation_source_supplement.py` (`NAVIGATION_SOURCE_DIGEST`). Re-pinning those is outside the owned set.
- `show_count` has no Reference Item model field, and the model's temporal modes are two, not three.

## Validation

See the lead's report for the candidate; local checks run on this branch: `test_lower_timed_item_packet.py`
and `--check`, `cargo fmt --all --check`, clippy on `oteryn-game-server`, `cargo test` (content),
`git diff --check`, `python tools/agents/validate_governance.py`.

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
