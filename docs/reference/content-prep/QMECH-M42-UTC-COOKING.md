# M42: fenced UTC cooking dispatch

The production NPC owner now calls the existing Hot Cuisine planner for all fifteen recipe confirmations. It reloads the clock, real level, live Premium view, quest tracks and exact carried Item/fluid custody under the Character fence. The plan may charge only the recipe selected by both its topic and persisted current dish. A permitted repeat has a distinct monotone cooking-occurrence reward key; command replay keeps its original receipt. Other NPC once-only reward identities are unchanged.

The same transaction consumes exact typed ingredients, grants the dish and progress, advances the final menu and delegates Culinary Master to the shared M41 achievement owner. No alternate achievement table or owner was added. Reused QMECH eligibility support remains authoritative. Migration 2080 checks occurrence identities and deferred typed input proof; earlier migrations are unchanged.

The native Item producer admits only the immutable cookbook operands and exact Bat/Chicken corpse stages (45 additional identities). Existing source physical facts, loot quantities and probabilities remain pinned. Canonical cookbook witnesses compare exact selected rows so unrelated changes in the same canonical shards do not repin the held batch3 packet. This qualifies ingredient custody, not additional food-use effects.

## Focused evidence

RED before dispatch: the retained PG recipe writer returned StageMismatch; the pure production dispatch test failed with missing plain-item resolution. On fresh QMECH integration all three PG cases also refused until fixtures supplied the actual new level/Premium preconditions. These gates were preserved, and the pure test asserts refusal for level34 and missing Premium.

GREEN, PostgreSQL 18.6 (Debian 18.6-1.pgdg13+2):

| test | real duration |
|---|---|
| m42_final_menu_and_culinary_master_use_real_typed_receipts | 47.242975205 s |
| m42_utc_cooking_refuses_wrong_fluid_and_commits_exact_typed_recipe | 46.250236506 s |
| npc_item_exchanges_consume_exact_recipes_with_one_receipt_and_replay | 46.782046502 s |
| exact migration2080 receipt-key case: 10,000 unique occurrences, legacy identity, malformed refusal | 0.210577941 s |

The pure m42_locked_source_selects_all_fifteen_utc_recipes test passed in 0.187s. The selected nextest run executed four tests, all passed (140.585s); unselected workspace tests were filtered, not claimed as validation. PG cases cover water/wine versus milk, missing ingredients, mismatched dish/topic, actual repeat cooking, once-only Culinary Master, replay, exhausted bonus, full-backpack refusal rollback and stale authority. Prior courses/apprenticeship are explicit resumed fixture history. These tests qualify the mechanic, not a complete fifteen-course fresh-player story or canonical terminal quest-state completion.

Quest-authoring checks passed. Its unmodified unit runner passed 1131 tests in 783.469s with eight standard pre-existing configuration skips. Content-tree validation, exact cookbook canonical/custody tests, official native-only regeneration and the repository duplicate-migration check passed.

Source recipe facts are retained in content/quests/hot-cuisine-batch3.json and the existing source-qualified batch3 cookbook. Runtime behavior reuses crates/gameplay/quests/src/quest/batch3_a/hot_cuisine.rs rather than a second recipe implementation.
