# OTV2-20261003-item-sem-2b2-equipment-requirements

```yaml
task_id: OTV2-20261003-item-sem-2b2-equipment-requirements
title: "ITEM-SEM-2b-2: slot, hands and requirements into content (narrowed to equipment.patterns)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-sem-2b2-20261003
issue: 162
coordination: 1622
pr: 1706
head_sha: "exact frozen head in the control plane FREEZE_SHA entry"
merge: "squash merge of #1706"
owner: oteryn-impl-worker (CP session_013KJX6mv8LQveCKKXYgAX94)
created_at: 2026-10-03
updated_at: 2026-10-03
packet: docs/architecture/reviews/OTERYN_GAME_ITEM_SEM2B2_EQUIPMENT_REQUIREMENTS_PACKET_2026-10-03.md
rebased_by: docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ITEM_EQUIP_PACKETS_2026-10-03.md (§1.4, §2.2; base main, depends_on [])
owned_paths:
  - tools/content-schema/item-authoring/{lower_wiki_stats_packet.py,test_lower_wiki_stats_packet.py,README.md}
  - docs/agents/evidence/OTV2-20261003-item-equipment-requirements-v1.json
  - apps/game-server/src/content/{item_stats_promotion.rs,mod.rs}
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/world/** and the content tree (regenerated)
  - docs/agents/tasks/archive/OTV2-20261003-item-sem-2b2-equipment-requirements.md
public_contracts: []
jira: null   # sync pending (coordinator batch)
```

## Narrowing (CP decisions, #1622)

- (a) The packet's target forms are absent from the runtime content model `ReferenceItemSemantics`: the compact `equipment` form, `requirements{min_magic_level, enforcement_mode: on_use}` and the `none` vocation. The slice lowers into the existing `equipment.patterns`. The `none` vocation (`without`) and the use-requirements group (runes, `mlrequired`, Extra-slot requirements) are held, reported, and re-packeted as ITEM-SEM-2b-3, a hard slice with contract review.
- (b) "Ammunition without a slot → Extra" is dropped. Item 3450 is pinned by the reward stack proof, which is outside the owned paths. Ammunition keeps no equipment block, as on main, and moves to ITEM-SEM-2b-3.
- The CP correction from the #1702 review: two-handed distance weapons take the `non_quiver_left_hand` group, not a shield reservation, and a bow plus a quiver stays legal (tested).

## Outcome

- DONE: 1,792 equipment pattern rows (1,791 in the tree census), recorded in `OTV2-20261003-item-equipment-requirements-v1.json`:
  - the shield slot is reserved by 275 two-handed weapons;
  - the group `oteryn:equipment-group.non_quiver_left_hand` is taken by 228 Items: two-handed distance weapons, shields and spellbooks;
  - 981 rows carry a level and 814 carry vocations;
  - the Extra slot carries no requirement;
  - level `0` writes nothing;
  - `None` means unrestricted.
- Reported, not written:
  - 7 `without` holds (`VOCATION_WITHOUT_HELD_FOR_ITEM_SEM_2B3`);
  - 6 `SLOT_HANDS_DISAGREE` holds;
  - 72 `USE_REQUIREMENT_HELD_FOR_ITEM_SEM_2B3` rows.
- DONE: `item_stats_promotion.rs` admits only that group, only on a hand slot and never together with a shield reservation. It refuses level 0, and it refuses a level or vocation on the Extra slot.
- Promoted vocations are matched by ITEM-MOVE-2a (arch batch §1.6). Content writes base keys only.
- The D303 guard (§2.7) did not apply: the patterns form on main already admits vocations and level.

## Validation

- `python3 tools/content-schema/item-authoring/test_lower_wiki_stats_packet.py` pass
- `python3 tools/content-schema/item-authoring/lower_wiki_stats_packet.py --check` pass
- `cargo test --locked -p oteryn-game-server --lib item_stats_promotion` pass (16)
- `python3 tools/content-migration/regenerate_content.py` pass (tree validators, item key references, authoring checks, `test_engine_items.py`, binding generator, `content_world_project_repository` 3 passed)
- `cargo fmt --all --check` pass
- `ruff check` / `ruff format --check` (changed files) pass
- `python tools/agents/validate_governance.py` pass
- `python -m unittest discover -s tools/agents/tests` OK

Review: content review on the final frozen head (CP routes).
