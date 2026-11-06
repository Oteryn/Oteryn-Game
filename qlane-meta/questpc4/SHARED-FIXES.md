# questpc4 shared fixture repairs

- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_callbacks.rs`:
  `source_world_choices` delegates to `source_world_positions` so a source-shared
  claim can retain chests on different rows (Troll Cave UID6039/6040). Sectors
  group on both coordinates and use bundle order; single-row callers retain
  their former tiles. Proof: `questpc4_troll_cave_rewards` passed with its marker in
  `logs/restart-rewards.log`; the other owned reward cases cover the unchanged
  single-row path. No out-of-lane PostgreSQL filter was run.
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/source_quest_rewards.rs` (renamed from `questpc4_rewards.rs`):
  each stale-fence probe uses its own composed owner; the active movement cells
  come from the activated production room. STEP reconnects carry their returned
  continuity into USE, including command 3 for reconstructed-owner replay.
  The refusal and replay assertions remain strict. Proof: all `questpc4_rewards`
  cases; current-tree run retained in `logs/restart-rewards.log`.

No migration, lease, replay-cursor or admission production code changed.

## Generated provenance ordering (no producer patch)

`components248/boss.json.gz` differs from HEAD in one gzip header byte only
(OS3→255); its decoded source JSON is identical. Its SHA changed from
`a12aebb94627758e7e5ce7a7931dc9ba62f46e2fe2487e7f56d54528f91a4a29`
to `e76e2f0eba20360f5f758a5cfc1235edd9edfd1cb8f69946a0a034b084ee9bd5`.
The donor generator propagates this into `refinements/joins.json`
`input_sha256.boss_descriptor_packet`; definitions then pin that joins packet.
A donor regeneration must therefore precede final Quest tree/consumer refresh.
The full NPC packets depend on final Quest hashes, while refinement dialogue
correctly depends on the stable `source_data` projection: this is not a cycle.
No provenance checks or compiler fields were weakened to resolve it. Proof: the
whole-check runner caught exact stale packet SHA fields in90 donor references
and two index inputs. Final ordered regeneration/check logs are retained.

## Native container Item count
- `tools/content-migration/test_quest_callback_profiles.py`: complete native bag recipes now reference131 distinct Items including the Grizzly icon, previously130. Compared HEAD/current recipe sets: the only addition is `oteryn:item.tibia.i3083` (Triangle Tower Garlic Necklace), with no removal. The exact count is131 and an explicit necklace assertion was added; all per-Item custody, native identity, weight and stack assertions remain intact. RED: `QuestCallbackProfiles.test_grizzly_icon_and_every_complete_chest_child_have_native_profiles`, `AssertionError: 131 != 130` (continuation-native-unit.log; other12 tests passed). Isolated GREEN passed1 test in4.103s (continuation-native-count-green.log).

## Reward fixture teardown
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/source_quest_rewards.rs`: successful cases previously omitted Harness::cleanup. `Database` has no Drop implementation and `fixture::run` only runs the body, so the newly created database remained allocated after each placement. Release the authority/seal borrows and await the existing per-instance cleanup, exactly as quest_world_callbacks/scenario.rs already does. This affects only the database created by that current case; it performs no historical/shared database cleanup. No assertion or production path changed. Focused proof passed: `source_quest_rewards::troll_cave_rewards`, both source orientations, marker only after each cleanup returns,88.188581216s (continuation-troll-cleanup.log).

## Source reward fixture: quest-linked chests (recovery)
File: crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/source_quest_rewards.rs. The existing shared fixture never set quest_catalogue or called admit_quest_session, so Shattered Isles UID6194 was rejected by bundle_chest.rs before minting despite its existing native Item profile. Load the existing embedded catalogue only for claims with an authored quest_transition, admit the quest session on each reconstructed owner, and assert one consumed obligation/receipt across replay. No production guard or catalogue was changed. RED: recovery-four-rewards.log, the_shattered_isles_rewards, expected Committed but received Rejected. GREEN pending.
GREEN for the conditional catalogue/session fixture repair: recovery-four-rewards-green.log, the_shattered_isles_rewards,27.791745254s, completion marker;4/4 expansion cases pass. Batch clippy passes in recovery-first-clippy.log.

## Shared reward-choice approach geometry
File: crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_callbacks.rs. source_world_positions bounded its west approach only from the selected chest, so the rightmost Pits choice UID6212 left the western sibling UID6210 at x32804 without its x32803 approach tile. Derive min_x from every source choice x-2 (one-line fixture-only repair). No production movement or claim guard changed. New choice PG tests and focused Troll Cave prove the shared fixture; results pending.

## Mixed USE/STEP transport cursor
File: crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_callbacks.rs. wire_use_targets consumed AdmittedSession by value and discarded ConnectionEnd continuation, so a later STEP supplied the old item/stream cursor and setup::prepare disconnected before a command result. Keep its returned admitted continuation through a mutable reference, preserving the old wire_uses convenience interface. Update all callers (only the shared reward template and wrapper). RED: claims-choice-green.log, the_thieves_guild_rewards: Error: "no command result", serve_end at serve/mod.rs:217 with next_command_id=3 after successful USE advanced to5. GREEN pending in claims-choice-cursor-pg.log. No runtime admission guard, replay policy, deadline or test assertion changed.
Caller audit correction: three existing quest_container_chests callers use the by-value helper. Preserve that public test-helper signature as a thin wrapper, and add wire_use_targets_retained only for the two mixed USE/STEP callers. No other lane file is edited. claims-choice-cursor-pg.log stopped with three E0308 compile errors from the briefly changed signature, before test execution; the fresh corrected run is claims-choice-retained-pg.log. Full workspace rg now accounts for every caller.

Final GREEN for approach geometry and retained USE/STEP continuation: claims-choice-retained-pg.log,3 passed/0 failed, all source orientations for Pits349.127009281s, Thieves213.217412644s and Troll57.353195072s, followed by final all-target/all-feature clippy. Legacy helper callers preserve their original interface. No shared runtime guard changed.

## Complete native chest closure after Desert admission
File: tools/content-migration/test_quest_callback_profiles.py. Adding UID6158's already-canonical green bag introduces exactly three distinct complete-recipe Items:3077 Magic Light Wand,3084 Protection Amulet and3098 Ring of Healing. The native closure grows131->134; every Item's admission/physical/binding assertions remain intact and explicit member assertions were added. Native profile records are byte-for-byte unchanged. RED: desert-native-count-red.log, test_grizzly_icon_and_every_complete_chest_child_have_native_profiles,134!=131. GREEN: all13 tests in desert-native-python.log,44.905s. This is an exact generated-scope count update, not a runtime or fixture-guard change.

## Thawing source fixture and completion evidence
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/map_harvest.rs`: preserve the admitted session returned by the post-restart refusal before issuing subsequent harvest commands; the old fixture stopped there and did not need that cursor. Ten earned seeds, an eleventh seed and replay pass in `w19_ice_flower_harvest_wire_yield_replay_and_restart` (logs/thawing-ten-seeds-pg.log,27.2176211s). Move COMPLETED after cleanup and add duration; a route-less skip emits SKIPPED only. No assertion was weakened.
- `apps/game-server/src/node/serve/daily_world_changes.rs`: source-ground fixture now includes the fourteen Thawing positions, allowing boot tests to exercise the real map projection. Regression RED at missing base flower (logs/thawing-boot-red.log) and GREEN1/0 (logs/thawing-boot-green.log). This fixture extension changes no trader/fungus/Grimvale assertion. Production fix is mechanic-specific and uses the existing reset projection, not a new custody owner.

## Batch02 source callback/placement mechanisms
- `Callback.facing` and bounded `Callback.effects` use the existing movement proof/effect stream; all11 teleport helper callers are accounted for. Waterfall four real STEP callbacks, facing, two splash54 occurrences and replay pass116.573114609s.
- `quest_startup_bindings.py` losslessly restores seven pinned startup selectors into existing source B3 Items; conversion reapplies the same overlay. Full donor capture/check, exact native selectors, ambiguity refusal and idempotence pass; semantic comparison proves no other tile/item fact changed.
- `Passage.create_on_close` supports source-asymmetric wall removal/creation through the existing Channel map projection owner. Mixed inventory/teleport/XP/presentation effects are refused. Triangle UID30034 retains literal remove1270/create1025, source non-reopening/repeated creates and actual native basement STEP exit; PG34.271905343s.
- Both Vampire Shield source5102 doors had no native physics profiles. The existing source-derived key/level-door importer/admission set now includes5102/5103, generated from hash-verified pinned data. No existing profile changes. Both real native source actions1070 reject69/admit70, replay and stale test pass60.632619611s.
- Vampire's shared wire fixture uses current production progression policy, fresh initialization, real build/vocation/XP owners, bound lease and live-level refresh. The old initialized test-only progression fixture was intentionally retained for its existing callers; no production guard was relaxed.
See BATCH02-REVIEW.md and BATCH02-SEMANTIC-AUDIT.json. All remaining Game mechanisms stay actionable lane work.
