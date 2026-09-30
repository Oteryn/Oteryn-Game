# OTV2-20260930-item-sem-2b-1-item-stats-promotion

```yaml
task_id: OTV2-20260930-item-sem-2b-1-item-stats-promotion
title: ITEM-SEM-2b-1 Promote TibiaWiki Item stats (weapon, armor, imbuement slots, weight) into content
mode: IMPLEMENT
status: completed_pending_merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-sem-2   # owner 1a; #1329 on this branch was closed as superseded by #1325
issue: 162
parent: OTV2-20260930-item-sem-2-item-stats (archived; 2a and its fixup #1325 are on main)
owner: owner-directed Claude Code session
created_at: 2026-09-30
updated_at: 2026-09-30
owned_paths:
  - tools/content-schema/item-authoring/{lower_wiki_stats_packet.py,test_lower_wiki_stats_packet.py}
  - docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json
  - apps/game-server/src/content/{item_stats_promotion.rs,mod.rs}
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-census/g4_item_wave1_stage.py   # v2 values supersede frozen Wave 1 candidates
  - content/world/** and the content tree (regenerated)
  - docs/agents/tasks/archive/OTV2-20260930-item-sem-2b-1-item-stats-promotion.md
public_contracts:
  - DUR04-REFERENCE-ITEM-PROFILE-V1 (limits unchanged)
jira: null   # sync pending (coordinator batch)
```

## Outcome

- DONE: `lower_wiki_stats_packet.py` lowers the pinned snapshot `imports/tibiawiki/facts/items-stats.json` (records
  keyed by Item key, else by bare Tibia id, per #1325) into the pinned packet
  `OTV2-20260930-item-stats-promotion-v2.json`: 10,260 fields on 6,382 Items (weight 6,371; weapon type 872;
  defense 732; attack 643; imbuement slots 633; armor 470; range 218; defense modifier 172; elemental attacks 149).
  Reported, not promoted: 5 weight conflicts between pages, 7 malformed values (`0.5`, `?`, `12 +1`...).
- DONE: `item_stats_promotion.rs` applies it in the materializer right after the A12 key switch; rows are decoded
  strictly (closed enums, bounds, one row per field, existing Item only) and set the field, replacing earlier values.
  148 fields changed an earlier (Crystal v1 or Wave 1) value.
- FOUND: Wave 1 bound Item 50161 (falcon sai, a monk fist weapon) to a TibiaWiki BR axe page (3 slots, AXE). The
  English wiki (2 slots, fist) now supersedes both; the tree validator accepts a superseding v2 value over the Wave 1
  provenance record.
- DONE (owner decision, 2026-09-30): weight unit = hundredths of an ounce (41.00 oz = 4100); the tree validator no
  longer blocks `physical.weight`.
- PROVEN: DUR04 item profile limits hold (reference artifact tests pass unchanged).
- Superseded: #1329 (bare-id snapshot keys, pickupable/marketable) was closed; #1325 had already fixed `main`.
  Pickupable and marketable move to 2c.

## Next

- 2b-2: requirements (level, vocation, hands, slot) into `equipment.patterns`.
- 2b-3: modifiers (skills, leech, critical hit, resistances).
- 2c: forge classification, light, blocking, usable, pickupable, marketable (model and contract review first).
- Owner question pending: add `test_lower_wiki_stats_packet.py` to the item-authoring CI lane (workflow edit).
