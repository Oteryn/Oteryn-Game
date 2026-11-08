# Quest sideB merge resolution

All 76 requested working-tree files are marker-free. No staging, commits, pushes or full regeneration were performed. Git still reports unmerged index entries because staging was prohibited.

## Per-file choices

| File | Resolution |
| --- | --- |
| `apps/game-server/tests/integration_rest/world_reward_claims_binding.rs` | Main version retained (newer behavior/fields). |
| `content/creatures/prey-slot-values.json` | Main version retained (newer behavior/fields). |
| `content/items/definitions/spell-native-profiles.json` | Union of both production-key sets (5,934); main wins existing record collisions. |
| `content/items/definitions/srx1-item-updates.json` | Union of both production-key sets (55); main wins existing record collisions. |
| `content/quests/grizzly-boss-rooms.json` | Main version retained (newer behavior/fields). |
| `content/spells.manifest.json` | Main version retained (newer behavior/fields). |
| `content/world/pins/oteryn.json` | Kept main pin fields/digest; exact regeneration deferred as requested. |
| `content/world/placements/index.json` | Combined object/record entries from both sides; main scalar values take precedence. Regeneration deferred. |
| `crates/client-assets/src/world.rs` | Kept main placement digest; exact regeneration deferred as requested. |
| `crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs` | Combined main baseline/custody checks with member identities; fixed source cohort count at 71 and retained source-qualified zero-speed assertion for item 3212. |
| `crates/gameplay/dispatch/src/gameplay_transport/npc_command/reward_callbacks.rs` | Combined both sides, retaining main current fields and member additions. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/classic_quests.rs` | sideB version retained. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_callbacks.rs` | Combined rectangular and arbitrary-position fixtures, retaining all current Terrain fields. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks.rs` | Combined both sides, retaining main current fields and member additions. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs` | Kept main fixture construction and all member quest routes, binding checks and premium setup. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc8_mystery_geometry.rs` | Main version retained (newer behavior/fields). |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc8_secret_library.rs` | Main version retained (newer behavior/fields). |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc8_trader.rs` | Main version retained (newer behavior/fields). |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc8_treasure_island.rs` | Main version retained (newer behavior/fields). |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc8_venore.rs` | Combined both sides, retaining main current fields and member additions. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/soul_war.rs` | Main version retained (newer behavior/fields). |
| `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs` | Combined both sides, retaining main current fields and member additions. |
| `crates/gameplay/durability/src/durability/quest_state/writer.rs` | Main version retained (newer behavior/fields). |
| `crates/gameplay/durability/src/durability/quest_world_callback.rs` | Main version retained (newer behavior/fields). |
| `crates/gameplay/durability/src/durability/reward_claim_mint/fluid.rs` | Kept main root complete-plan proof and child fluid initialization with receipt provenance. |
| `crates/gameplay/durability/src/durability/reward_claim_mint/quest.rs` | Kept main receipt, condition and state-operation handling; retained member completion plan field. |
| `crates/gameplay/npc/src/npc_talk.rs` | Combined native NPC admission and member merchants; retained each dialogue/greeting document once. |
| `crates/gameplay/quests/src/quest/carried_step.rs` | Combined both sides, retaining main current fields and member additions. |
| `crates/gameplay/quests/src/quest/fire_feathered_native.rs` | Main version retained (newer behavior/fields). |
| `crates/gameplay/quests/src/quest/hive_access.rs` | Main version retained (newer behavior/fields). |
| `crates/gameplay/quests/src/quest/reward_callbacks.rs` | Combined current fields/state operations/load_for with member exchanges, bounds and marker handling; removed duplicate definitions. |
| `crates/gameplay/quests/src/quest/rotten_blood.rs` | Main version retained (newer behavior/fields). |
| `crates/gameplay/quests/src/quest/schrodinger_island.rs` | Main version retained (newer behavior/fields). |
| `crates/gameplay/quests/src/quest/shared_switches.rs` | Combined both sides, retaining main current fields and member additions. |
| `crates/gameplay/quests/src/quest/shared_world_progress.rs` | Combined both sides, retaining main current fields and member additions. |
| `crates/gameplay/quests/src/quest/world_callbacks.rs` | Combined main state operations with member dialogue registry, markers and selector partition checks. |
| `crates/gameplay/world/src/map/floor.rs` | Main version retained (newer behavior/fields). |
| `deploy/synology-game/preprod-topology.toml` | Kept main matching map revision and bundle digest; regeneration deferred. |
| `docs/agents/evidence/quest-claim-qualification.json` | Kept main: 162 pending claims; every member-qualified claim is already qualified by main. |
| `docs/balance/PREY-SLOT-VALUE-2026-11-01.md` | Main version retained (newer behavior/fields). |
| `docs/balance/PREY-TIME-SIM-2026-11-01.md` | Main version retained (newer behavior/fields). |
| `docs/reference/DROPPED-WRATH.md` | sideB version retained. |
| `docs/reference/colours-opening-validation.json` | sideB version retained. |
| `docs/reference/main-3209-validation.json` | sideB version retained. |
| `docs/reference/main-83ac-validation.json` | sideB version retained. |
| `docs/reference/main-98d6-validation.json` | sideB version retained. |
| `docs/reference/main-c77-validation.json` | sideB version retained. |
| `docs/reference/night-retro-validation.json` | sideB version retained. |
| `docs/reference/unit-completion-validation.json` | sideB version retained. |
| `docs/reference/wrath-crystals-validation.json` | sideB version retained. |
| `docs/reference/wrath-dream-validation.json` | sideB version retained. |
| `docs/reference/wrath-imperial-validation.json` | sideB version retained. |
| `docs/reference/wrath-source-dragon-validation.json` | sideB version retained. |
| `docs/repository/QUEST_CALLBACK_COVERAGE.json` | Kept main current coverage: 172 qualified, 162 pending, 2 excluded; all member claims remain represented. |
| `qlane-meta/questpc8/ENGINE-REQUESTS.md` | Retained sideB file in delete/modify conflict. |
| `qlane-meta/questpc8/FINAL.md` | Retained sideB file in delete/modify conflict. |
| `qlane-meta/questpc8/QUEST-STATUS.json` | Retained sideB file in delete/modify conflict. |
| `qlane-meta/questpc8/RESUME.md` | Retained sideB file in delete/modify conflict. |
| `tools/content-migration/qengine_equipment_audit.py` | sideB version retained. |
| `tools/content-migration/refresh_equipment_profile_pin.py` | Combined current custody baseline and registered-overlap validation with member frozen-union mode. |
| `tools/content-migration/regenerate_content.py` | Kept main regeneration order/proof refresh and added member authoring tools; consolidated duplicate invocations. |
| `tools/content-migration/test_quest_callback_profiles.py` | Both sides already equivalent; retained content. |
| `tools/content-migration/test_refresh_equipment_profile_pin.py` | Retained both sets of test cases. |
| `tools/content-migration/test_regenerate_content.py` | Kept main compiler/callback fixture and member exact snapshot comparisons. |
| `tools/content-schema/native-gameplay/prey-model-results.json` | Main version retained (newer behavior/fields). |
| `tools/content-schema/native-gameplay/questpc8-equipment-custody-census.json` | Combined object/record entries from both sides; main scalar values take precedence. Regeneration deferred. |
| `tools/content-schema/quest-authoring/check_quest_claim_qualification.py` | Main version retained (newer behavior/fields). |
| `tools/content-schema/quest-authoring/qengine_rewards.py` | Combined main outfit source paths and callbacks with member NPCs and L2 stories/shops. |
| `tools/content-schema/quest-authoring/quest_batch4.py` | Kept main at_least validation and member read-only increment refusal. |
| `tools/content-schema/quest-authoring/samples/binding_packets/index.json` | Main version retained (newer behavior/fields). |
| `tools/content-schema/quest-authoring/samples/binding_packets/npc/quest-npc-dialogue-links.json` | Main version retained (newer behavior/fields). |
| `tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json` | Main version retained (newer behavior/fields). |
| `tools/content-schema/quest-authoring/test_native_claim_source_readiness.py` | Combined main true-hold comparison with member historical category assertions. |
| `tools/content-schema/quest-authoring/test_quest_claim_qualification.py` | Combined both sides, retaining main current fields and member additions. |
| `tools/content-schema/reward-claim-authoring/container_native.py` | sideB version retained. |
| `tools/content-schema/world-authoring/samples/world-base-capture-v1.json` | Combined object/record entries from both sides; main scalar values take precedence. Regeneration deferred. |

## Adjacent compilation repairs

These automatic-merge errors were also repaired: repeated constructor fields, a lost existing fluid field, missing current fields in new fixtures, duplicated tests, and a shifted document reference.

- `crates/gameplay/dispatch/src/gameplay_transport/arena_match.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/l6_reward_fluids.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qengine_access.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qengine_rewards.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questl2_djinn_world.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questl2_white_raven_world.rs`
- `crates/gameplay/durability/src/durability/reward_claim_mint.rs`
- `crates/gameplay/quests/src/quest/reward_callbacks/carried_exchange_tests.rs`
- `crates/gameplay/quests/src/quest/reward_callbacks/published_markers.rs`

## Validation

- The marker scan over all 76 requested paths returned no matches (rg exit 1).
- Every requested JSON file parses; duplicate JSON object keys are rejected in the validation read. Both item-profile production-key sets are retained without duplicate identities.
- Seven touched crates passed `cargo check --locked -p ...` and all-target/all-feature Clippy with `-D warnings`. The final repeat also passed all four commands (exit 0).
- `cargo fmt --all --check` and `git diff --check` passed.
- 22 focused Python checks passed: 10 equipment-pin tests, 8 qualification tests, 1 source-readiness test, and 3 pure regeneration tests. The regeneration tests call only the isolated fixture helpers; no full regeneration was run.
- 28 filtered durability unit tests and the focused NPC merchant test passed.
- Both focused PostgreSQL tests passed against actual fixtures: fluid root/children mint/replay (3.25 s), exact fluid output projection/refusals (2.46 s). The dispatch SQL test printed `POSTGRES_EXACT_FLUID_OUTPUTS_LEGACY_AND_REFUSALS_PASS`.

All Cargo invocations use `/home/mole/oteryn-cache/codex-bin/cargo` and `CARGO_NET_OFFLINE=true`.

Exact Rust test commands and filters:

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-quests --lib --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-durability --lib reward_claim_mint --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --test reward_claim_mint_postgres --quiet -- filled_fluid_claims_commit_root_and_children_and_replay_without_refilling
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib questpc8_fluid_outputs_preserve_exact_subtype_and_legacy_plan_postgres --quiet -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-npc --lib conditional_merchants_have_greetings_without_reward_callbacks --quiet
```

Seven-crate check/Clippy package set:

```text
oteryn-game-server
oteryn-client-assets
oteryn-gameplay-dispatch
oteryn-gameplay-durability
oteryn-gameplay-npc
oteryn-gameplay-quests
oteryn-gameplay-world
```

## Unfinished

1. **Quest catalogue admission is blocked:** quest unit tests report **301 passed, 74 failed**. Their common root is `reward callback bounds`: the combined documents contain **257 callbacks**, exceeding the unchanged accepted **256** limit. The four document counts are shared 202, L2 ready stories 20, L6 stories 7, and chosen 28. No callback or distinct test case was removed, and the bound was not increased. Full quest dispatch qualification remains blocked by this admission failure.
2. **Full content regeneration is intentionally deferred.** Digests, manifests, world pins and derived census data still require the later authorized regeneration. These merge outputs are valid JSON and retain both sides' identities; they are not a regenerated content candidate.
3. The existing callback-profile Python suite reports **5 failures and 13 errors**. The common `QUESTS_2_EMPEROR_CALLBACK_NATIVE_PROFILE` mismatch exists on HEAD too: native profiles for items 3217 and 19084 contain the newer `always_on_top` and `always_on_top_order` attributes, while their frozen source packet omits them. Main's native attributes were preserved.
4. Git index conflicts remain recorded until the owner stages the resolved files; staging was prohibited.

## Final repeat

All commands returned exit 0 on the final source tree:

```sh
/home/mole/oteryn-cache/codex-bin/cargo fmt --all --check
/home/mole/oteryn-cache/codex-bin/cargo check --locked -p oteryn-game-server -p oteryn-client-assets -p oteryn-gameplay-dispatch -p oteryn-gameplay-durability -p oteryn-gameplay-npc -p oteryn-gameplay-quests -p oteryn-gameplay-world --quiet
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked -p oteryn-game-server -p oteryn-client-assets -p oteryn-gameplay-dispatch -p oteryn-gameplay-durability -p oteryn-gameplay-npc -p oteryn-gameplay-quests -p oteryn-gameplay-world --all-targets --all-features --quiet -- -D warnings
git diff --check
```
