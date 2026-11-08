# qeng2 merge resolution

All 57 requested paths were inspected. All 657 current conflict blocks were resolved, including historical diff3 base sections. Listed JSON parses, and the merged cosmetic authoring and reward callback schemas validate. Git’s index remains untouched as instructed; unmerged index entries require the coordinator’s later staging.

No full regeneration, staging, commit, push, checkout, reset or stash was run. No protected CI paths or credentials were changed.

## Per-file resolution

| File | Resolution |
| --- | --- |
| `apps/game-server/migrations/1550_reward_claim_additional_items.sql` | Retained the existing incoming file; no markers were present (modify/delete or add conflict). Extended the existing claim-key proof for clock-guarded world occurrences while retaining main’s paid/NPC arms. |
| `apps/game-server/migrations/1551_complete_cosmetic_piece_recipes.sql` | Retained the existing incoming file; no markers were present (modify/delete or add conflict). |
| `content/cosmetics/outfits/authoring.json` | qeng2 superset; retains all main outfits/mounts and grant aliases. Generated digests were not refreshed. |
| `content/cosmetics/outfits/native-cosmetics.json` | combined stable-key registry entries; qeng2 scalar updates. Generated digests were not refreshed. |
| `content/cosmetics/taming-native.json` | Retained the existing incoming file; no markers were present (modify/delete or add conflict). |
| `content/dialogues/quests/shared-reward-greetings.json` | combined stable-key registry entries; qeng2 scalar updates. Generated digests were not refreshed. |
| `content/items/definitions/spell-native-profiles.json` | Union by production key; retained main’s newer overlapping records and all seven additional incoming items. Digests await regeneration. |
| `content/quests/shared-reward-callbacks.json` | combined stable-key registry entries; qeng2 scalar updates. Generated digests were not refreshed. |
| `content/quests/shared-reward-offers.json` | combined stable-key registry entries; qeng2 scalar updates. Generated digests were not refreshed. |
| `content/spells.manifest.json` | combined stable-key registry entries; qeng2 scalar updates. Generated digests were not refreshed. |
| `crates/gameplay/dispatch/src/gameplay_transport/bundle_chest.rs` | Retained main’s current reward API; omitted the obsolete additional_items local. Existing contents/backpack outputs remain. |
| `crates/gameplay/dispatch/src/gameplay_transport/grizzly_kills.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_cosmetics.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_cosmetics/positive.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/mount_taming/pairs.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qengine_access.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qengine_endgame_stories.rs` | Retained the existing incoming file; no markers were present (modify/delete or add conflict). |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks.rs` | Retained main’s world builder and its explicit appearance/ordinal parameters. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs` | Retained main’s newer implementation/tests; incoming additions outside the conflicting block remain. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/rotten_blood_progression.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/soul_war.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/effects.rs` | Retained main’s newer implementation/tests; incoming additions outside the conflicting block remain. |
| `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/durability/src/durability/character_revision_sequencer.rs` | Retained main’s newer implementation/tests; incoming additions outside the conflicting block remain. |
| `crates/gameplay/durability/src/durability/quest_state/writer.rs` | Combined catalogue-aware authored owners and kill settlement with main’s fenced world/NPC transactions; removed the obsolete duplicate mint path. |
| `crates/gameplay/durability/src/durability/quest_world_callback.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/durability/src/durability/reward_claim_mint/fathers_burden.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/durability/src/durability/reward_claim_mint/quest.rs` | Retained main’s newer implementation/tests; incoming additions outside the conflicting block remain. |
| `crates/gameplay/npc/src/npc_talk.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/quests/src/quest/carried_step.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/quests/src/quest/loader.rs` | Retained main’s newer implementation/tests; incoming additions outside the conflicting block remain. |
| `crates/gameplay/quests/src/quest/mod.rs` | Combined authored binding metadata and conditional kills with main’s fixture support; distinct effect encoding tags 11 and 12. |
| `crates/gameplay/quests/src/quest/reward_callbacks.rs` | Combined authored/carried/parent guards with main’s tests and payout fields; consolidated validation and bound nonempty parent guards into the value hash. |
| `crates/gameplay/quests/src/quest/reward_callbacks/state_ops.rs` | Retained main’s newer implementation/tests; incoming additions outside the conflicting block remain. |
| `crates/gameplay/quests/src/quest/rotten_blood.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/quests/src/quest/shared_switches.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/quests/src/quest/world_callbacks.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/quests/src/quest/world_callbacks/effects.rs` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `crates/gameplay/social/src/interaction/chest_use.rs` | Retained main’s current reward API; omitted the obsolete additional_items local. Existing contents/backpack outputs remain. |
| `crates/gameplay/world/src/map/facts/quest_callbacks.rs` | Retained main’s newer implementation/tests; incoming additions outside the conflicting block remain. |
| `qlane-meta/qengine2/CONTRACT-NOTES.md` | Retained the existing incoming file; no markers were present (modify/delete or add conflict). |
| `qlane-meta/qengine2/QUEST-STATUS.json` | Retained the existing incoming file; no markers were present (modify/delete or add conflict). |
| `qlane-meta/qengine2/RESUME.md` | Retained the existing incoming file; no markers were present (modify/delete or add conflict). |
| `qlane-meta/questpc5/ENGINE-QUEUE.json` | Retained the existing incoming file; no markers were present (modify/delete or add conflict). |
| `qlane-meta/questpc5/PG-NATIVE-ADMISSION-2026-11-08.json` | Retained the existing incoming file; no markers were present (modify/delete or add conflict). |
| `qlane-meta/questpc5/PG-READY-CLAIMS-2026-11-08.json` | Retained the existing incoming file; no markers were present (modify/delete or add conflict). |
| `qlane-meta/questpc5/PG-WORLD-CLOCK-2026-11-08.json` | Retained the existing incoming file; no markers were present (modify/delete or add conflict). |
| `qlane-meta/questpc5/PG-WORLD-DELIVERY-2026-11-08.json` | Retained the existing incoming file; no markers were present (modify/delete or add conflict). |
| `qlane-meta/questpc5/PUBLICATION-STATUS.md` | Retained the existing incoming file; no markers were present (modify/delete or add conflict). |
| `tools/content-migration/qengine_reward_item_profiles.py` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `tools/content-migration/regenerate_content.py` | Combined main’s custody census and equipment pin refresh with qeng2’s equipment audit and generator ordering. |
| `tools/content-schema/cosmetics-authoring/cosmetics.schema.json` | combined stable-key registry entries; qeng2 scalar updates. Generated digests were not refreshed. |
| `tools/content-schema/native-gameplay/qengine-reward-item-profiles.json` | combined stable-key registry entries; qeng2 scalar updates. Generated digests were not refreshed. |
| `tools/content-schema/quest-authoring/qengine_rewards.py` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |
| `tools/content-schema/quest-authoring/shared-reward-callbacks.md` | Retained main’s newer implementation/tests; incoming additions outside the conflicting block remain. |
| `tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json` | Combined main’s backpack outputs, experience and receipt predicates with incoming completed-parent requirements. |
| `tools/content-schema/quest-authoring/test_qengine_rewards.py` | Combined both sides where needed; retained main’s newer fields, contracts and tests and qeng2’s additions. |

## Necessary supporting edits

The following existing or restored member files provide the referenced catalogue, source generators, dispatch test consumers, or constructor/API compatibility. No new standalone PostgreSQL test target was added.

- `crates/gameplay/content/src/content/native_gameplay/mount_taming.rs`
- `crates/gameplay/items/src/item_cosmetics.rs`
- `crates/gameplay/quests/src/quest/authored_bindings/tests.rs`
- `crates/gameplay/quests/src/quest/reward_callbacks/carried_exchange_tests.rs`
- `crates/gameplay/quests/src/quest/world_callbacks/state_operations.rs`
- `crates/gameplay/durability/src/durability/reward_claim_mint/marlin_trophy.rs`
- `crates/gameplay/durability/src/durability/reward_claim_mint/npc_program.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/npc_command/seasonal.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/l6_reward_fluids.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/authored_world_rewards.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_cosmetics/glider.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_cosmetics/stock.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qengine_s06_construction.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qengine_s06_items.rs`
- `tools/content-schema/native-gameplay/item-cosmetic-actions-native.json`
- `tools/content-schema/quest-authoring/qengine_item_acquisitions.py`
- `tools/content-schema/quest-authoring/qengine_item_construction.py`
- `tools/content-schema/quest-authoring/qengine_mounts.py`

- `crates/gameplay/items/src/item_cosmetics/tests.rs`: retain the immutable capture assertions (48 rules / 205 grants), and also assert the incoming native catalogue (49 rules / 202 grants).

- `crates/gameplay/items/src/item_cosmetics/tests/additional.rs`: preserve all pool/duplicate checks and cover the source-qualified addonless Wedding/Jungle Explorer outcomes.

## Validation

- Final `cargo check --locked --offline` passed for all eight touched Rust crates: `oteryn-gameplay-content`, `oteryn-gameplay-items`, `oteryn-gameplay-dispatch`, `oteryn-gameplay-durability`, `oteryn-gameplay-npc`, `oteryn-gameplay-quests`, `oteryn-gameplay-social`, and `oteryn-gameplay-world`.
- `cargo clippy --locked --offline` with those eight packages, `--all-targets -- -D warnings`, passed. After the final item-validator/test repair, `oteryn-gameplay-items --all-targets -- -D warnings` passed again. Existing vendor warnings were emitted; there were no project Clippy errors.
- Targeted rustfmt `--check` and `git diff --check` passed.
- Exact `rg -n '^(<<<<<<<|=======|>>>>>>>)'` over the 57 requested files returned no matches (exit 1).
- All listed JSON parses. The merged cosmetic authoring and reward callbacks validate against their schemas.

Every cargo command below used `/home/mole/oteryn-cache/codex-bin/cargo`; no dependencies were downloaded. PostgreSQL was reached through that wrapper, with no URL printed and no early-return qualification.

| Exact test command after the absolute cargo path | Result |
| --- | --- |
| `test --locked --offline -p oteryn-gameplay-quests --lib quest::world_callbacks::` | 46 passed |
| `test --locked --offline -p oteryn-gameplay-quests --lib completed_parent_requirements_are_bounded_unique_and_read_only` | 1 passed |
| `test --locked --offline -p oteryn-gameplay-content --lib mount_taming::` | 10 passed |
| `test --locked --offline -p oteryn-gameplay-items --lib item_cosmetics::` | 15 passed after restoring the member’s bounded recipe admission and exact native catalogue assertions |
| `test --locked --offline -p oteryn-gameplay-quests --lib quest::` | 364 passed, 2 failed; see Unfinished |
| `test --locked --offline -p oteryn-gameplay-dispatch --lib world_callback_clock_ -- --nocapture` | Both PG cases started, then failed on native gameplay input digest mismatch; see Unfinished |

`python3 -m unittest discover -s tools/content-schema/quest-authoring -p test_qengine_rewards.py`: 31 passed and one generated-output reproduction test failed, as detailed below. This builds expected outputs in memory; it does not regenerate files.

## Unfinished

1. Full content regeneration, manifests and pins are deliberately deferred by the task. The PG filter `world_callback_clock_` reaches the harness but both cases fail with `RevisionMismatch("native gameplay input digest")`. The cases are `world_callback_clock_mint_cooldown_and_replay_dispatch_postgres` and `world_callback_clock_full_bag_rolls_back_timestamp_dispatch_postgres`. They are not qualified. Other production-artifact PG stories were not rerun against known stale digests.
2. `quest::loader::tests::owner_scope_excludes_dawnport_from_runtime_state_and_callbacks` fails: its unchanged expected inventory omits 48 retained L6/code-built callbacks. No assertions were loosened or removed.
3. `quest::reward_callbacks::tests::published_ungated_quest_hash_baseline` fails at `oteryn:quest.outfit_and_addon_quests`: current `f37a54a3aa987e9539e416932db32476440c6d05d2a220ccada66760a4497b77`, expected `87f6476714ed0ad9ea3a2eb7c54e0a2d34f263f536be9d6c143153629d2f328d`. The published baseline assertion remains unchanged. Compatibility investigation remains with integration; the baseline was not replaced merely to pass.
4. Python `test_qengine_rewards_outputs_reproduce_and_validate` fails at `content/spells.manifest.json` due to the deferred generated digest refresh.
5. Git’s index still reports unmerged entries because `git add` was explicitly prohibited. Content markers are resolved; staging and concluding the merge remain with the coordinator.

After regeneration, the following additional narrow dispatch filters cover the merged member’s real paths: `authored_world_rewards::`, `authored_quest_bindings::`, `authored_kill_bindings::`, `definition_ready_claims::`, `qengine_l13_`, `s06_glider_`, and `s06_stock_`. Invoke each with `test --locked --offline -p oteryn-gameplay-dispatch --lib <filter>` through the absolute wrapper. They are listed as follow-up coverage, not claimed as passing in this run.

## Self-review

Re-read the handwritten resolution diff against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. Reviewed session fences, locking and receipt composition, retained premium/value guards, bounded parent lookups, test preservation and source custody. Searched constructors and effect matches across `apps/`, `crates/` and `tools/`; updated the exposed fixture constructors and used all-target compilation to verify them. The new state-clock encoding tag is 12; conditional-kill encoding keeps 11, with an assertion proving they differ. No protected CI files changed.
