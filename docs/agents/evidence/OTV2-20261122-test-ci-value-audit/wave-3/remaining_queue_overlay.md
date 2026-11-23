# Exact delta to the content-tools remaining queue

Input: `../wave-2/content_tools_remainingfiles.tsv` at pinned `2952e155ad6f371d1e4b3b351acd57d4757e386e`, parsed as 695 data rows (excluding header and trailing newline). All 18 paths below originally had `wave_status=STATIC_AST_ONLY`; no same path is counted twice.

## Read completely in preceding issue comment 7088506426 (10 files / 37 literal definitions)
- `tools/content-migration/test_amber_item_semantics.py`
- `tools/content-migration/test_apply_mitigation_estimates.py`
- `tools/content-migration/test_apply_monster_field_patches.py`
- `tools/content-migration/test_audit_house_decoration_sources.py`
- `tools/content-migration/test_boss_complete_order_metadata.py`
- `tools/content-migration/test_authored_npc_item_profiles.py`
- `tools/content-migration/test_authored_quest_custody_weights.py`
- `tools/content-migration/test_bestiary_inventory.py`
- `tools/content-migration/test_bind_monster_source_definitions.py`
- `tools/content-migration/test_coin_converter_profiles.py`

## Read completely in this wave (8 files / 25 literal definitions)
- `tools/content-migration/test_authored_quest_timed_custody.py` — 8; KEEP
- `tools/content-migration/test_authored_static_item_constructors.py` — 4; KEEP
- `tools/content-migration/test_boss2_craft_profile_join.py` — 2; KEEP
- `tools/content-migration/test_boss2_native_corpse_profile_coherence.py` — 3; KEEP
- `tools/content-migration/test_boss_conditional_bag_profile.py` — 1; KEEP
- `tools/content-migration/test_boss_corpse_terminal_reference.py` — 1; REWRITE
- `tools/content-migration/test_boss_movement_profiles.py` — 5; KEEP
- `tools/content-migration/test_boss_reward_binding_policy.py` — 1; KEEP

Before: 451 STATIC_AST_ONLY + 10 PARTIALLY_REVIEWED + 234 INVENTORY_ONLY = 695 entries.

After subtracting only these 18 exact paths: **433 STATIC_AST_ONLY + 10 PARTIALLY_REVIEWED + 234 INVENTORY_ONLY = 677 entries**. Retain the original source file as a historical index; this overlay is the current subtraction, not an assertion that other unvisited files were checked. Non-content lanes and the global 130-group queue are not included.

The `test_boss_corpse_terminal_reference.py` entry counts as **fully read but requires REWRITE**. It is not a passed test or a retired contract.
