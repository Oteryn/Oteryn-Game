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
| exact migration 2080 receipt-key case: 10,000 unique occurrences, legacy identity, malformed refusal | 0.210577941 s |

The pure m42_locked_source_selects_all_fifteen_utc_recipes test passed in 0.187s. The selected nextest run executed four tests, all passed (140.585s); unselected workspace tests were filtered, not claimed as validation. PG cases cover water/wine versus milk, missing ingredients, mismatched dish/topic, actual repeat cooking, once-only Culinary Master, replay, exhausted bonus, full-backpack refusal rollback and stale authority. Prior courses/apprenticeship are explicit resumed fixture history. The resumed-history fixtures qualify the mechanic; they do not claim a complete fifteen-course fresh-player story.

Quest-authoring checks passed. Its unmodified unit runner passed 1131 tests in 783.469s with eight standard pre-existing configuration skips. Content-tree validation, exact cookbook canonical/custody tests, official native-only regeneration and the repository duplicate-migration check passed.

Source recipe facts are retained in content/quests/hot-cuisine-batch3.json and the existing source-qualified batch3 cookbook. Runtime behavior reuses crates/gameplay/quests/src/quest/batch3_a/hot_cuisine.rs rather than a second recipe implementation.

## Canonical completion follow-up

Reused the coordinator-merged ENGNPC M04 implementation (also inspected on pushed worker head `863b400172f2698f6c90627ce7889ca999839bcf`), including its existing migration 2230. No second completion table, migration or writer was added. The native cooking plan marks first menu completion only after all fifteen persisted course counters reach one. Permitted later cooking remains non-completing.

The new pure completion test failed RED with “the final course must complete the canonical quest receipt” (0.027s), then passed GREEN in both the held and production module paths (0.035s/0.033s). The extended PG menu case proves the canonical quest-state marker and exactly one completing receipt, followed by actual bonus cooking, Culinary Master and replay. On the first current-QMECH integration its real PG duration was 48.529944712s; the existing ENGNPC M04 regression passed in 45.874761123s. The full five-test focused set passed.

The same integration reran all 112 authoring unit modules: 1138 tests, no failed modules. Final snapshot validation and its exact test durations are recorded with the final code commit.

## Push-batch snapshot e8a4a94f8

Merged coordinator QMECH head `e8a4a94f8` before this batch's final validation; newer coordinator work belongs to the next batch. Focused log: five tests passed in 183.470s. The PostgreSQL 18.6 canonical menu/Culinary Master story passed in 45.515757676s; ENGNPC M04 regression in 44.748188165s; three plain cooking recipes/replay/rollback in 47.731593487s. The exact receipt-key migration case reran in 0.211139499s. Nine selected quest/M07/SRV7 unit cases passed. All 112 authoring modules (1138 tests) passed again, as did quest-authoring checks, content-tree validation and the two independent cooking canonical/custody tests.

Final compile gates: gameplay quests/durability/dispatch clippy with all features and `-D warnings` passed (4m39s); workspace/all-targets check passed (4m14s) after supplying the missing neutral `creature_use: None` in the incoming Oramond authority fixture. Clippy of that server test target also passed (3m40s). Cargo fmt and explicit formatting checks for dynamically mounted PG modules passed. World pins and the preproduction digest were regenerated with the repository compiler. No checks were weakened or disabled.

## Coordinator-requested 0326 integration

The requested coordinator 0326dc2e5 merge was committed before the owner stopped further merges. Both sides' source behavior is retained; generated native providers were regenerated, not hand-merged. The independent equipment cohort sum is 5840. No further qmech/main merge is performed.

Focused tests: 5/5 passed in 184.904s on real PostgreSQL 18.6. Canonical menu/Culinary Master: 46.248617036s; exact typed fluid recipe: 45.555903765s; three plain recipes/replay/rollback: 47.615932692s; reused ENGNPC M04 regression: 45.11538521s. The exact migration 2080 function case reran in 0.227930502s. All 114 authoring unit modules (1145 tests), authoring checks, content-tree validation and independent cooking canonical/custody tests passed.

M46 was inspected as the next OPEN item, but coordinator commit 8b9968d4c supplied its implementation before publication. That implementation is retained rather than duplicated. The unpublished starter is not part of this package. Other stale OPEN rows have named assignments or pushed worker implementations, so they are not taken by questpc7.
