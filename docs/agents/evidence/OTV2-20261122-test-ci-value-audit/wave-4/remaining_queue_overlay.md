# Exact wave-4 content-tools remaining-queue delta

Source: `../wave-2/content_tools_remainingfiles.tsv` (695 data rows), minus the exact 18 paths recorded in `../wave-3/remaining_queue_overlay.md` (677 remaining). Each of the ten paths below was checked in the wave-2 source as `STATIC_AST_ONLY` and fully read at exact `2749acb3295b536f0b88dc04c25fe0f6afcbd526` before subtraction:

- `tools/content-migration/test_boss_reward_item_profiles.py` — 4 literal methods; 4 PASS
- `tools/content-migration/test_cake_bridge_reference.py` — 3 literal methods; 3 PASS via unittest; direct rc0 executes zero
- `tools/content-migration/test_cake_creature_admission.py` — 3 literal methods; 3 PASS
- `tools/content-migration/test_cake_native_successor.py` — 5 literal methods; 4 PASS original; 1 ERROR missing zstandard; same 1 PASS pinned zstandard 0.25.0
- `tools/content-migration/test_classify_monster_population.py` — 5 literal methods; 5 PASS
- `tools/content-migration/test_compact_record_json.py` — 1 literal methods; 1 PASS
- `tools/content-migration/test_complete_captured_loot_alias_delta.py` — 6 literal methods; 5 PASS; 1 ERROR FileNotFoundError /workspace/.../field-patches.json
- `tools/content-migration/test_complete_creature_dependencies.py` — 10 literal methods; 10 PASS
- `tools/content-migration/test_complete_familiar_population.py` — 4 literal methods; 4 PASS
- `tools/content-migration/test_complete_loot_quality_fields.py` — 9 literal methods; 9 PASS

Before: 433 `STATIC_AST_ONLY` + 10 `PARTIALLY_REVIEWED` + 234 `INVENTORY_ONLY` = **677**.

After: 423 `STATIC_AST_ONLY` + 10 `PARTIALLY_REVIEWED` + 234 `INVENTORY_ONLY` = **667**.

Fully read does **not** mean passing: the historical 294-row `test_complete_captured_loot_alias_delta.py` assertion remains blocked by missing snapshot on this machine. This queue counts paths, not Rust/Python test invocations, unique semantic behavior, or completion percentage; the other audit lanes retain their independent remaining ledgers.
