# questpc restart5 status reconciliation

All 178 pinned A..M titles now have a final local status: 22 QUALIFIED standalone source reward-only contracts and 156 BLOCKED, including one owner-excluded Dawnport title. This records implementation/qualification gaps; it does not claim that 156 quests were repaired or that all quests are playable. Per-title missing pieces, source file:line/JSONpointers and attempts:QUEST-STATUS.md/json.

No product changes were made in this continuation. All 174 inherited product paths are byte-for-byte unchanged. Current record/evidence files:restart5-changed-files.json. No shared-fixture change; SHARED-FIXES unchanged.

Fresh real PostgreSQL reward result: 9/9 PASS, 80 claims / 82 source placements, 44 reward sets; 643.87s test / 645.883s wall. Every placement emitted POSTGRES_CASE_COMPLETED,native_source=true and a positive duration. Only 22 standalone contracts qualify as per-title final successes; the remaining source reward proofs do not qualify full story/outfit/access paths.

Exact reward cargo command (CARGO_BUILD_JOBS=4):

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib -- quest_catalogue_chests_phase_a_01_use_replay_and_refusal quest_catalogue_chests_phase_a_02_use_replay_and_refusal quest_catalogue_chests_phase_a_03_use_replay_and_refusal quest_catalogue_chests_phase_a_04_use_replay_and_refusal quest_catalogue_chests_phase_a_05_use_replay_and_refusal quest_catalogue_chests_phase_a_06_use_replay_and_refusal quest_catalogue_chests_phase_a_08_use_replay_and_refusal quest_catalogue_chests_phase_a_09_use_replay_and_refusal quest_catalogue_chests_phase_a_11_use_replay_and_refusal --test-threads=1 --nocapture
```

Exact filters:

- `quest_catalogue_chests_phase_a_01_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_02_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_03_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_04_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_05_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_06_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_08_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_09_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_11_use_replay_and_refusal`

Other cargo tests:

```sh
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib catalogue_npc_scoped_a_m_starts_are_source_ordered_and_atomic -- --test-threads=1 --nocapture
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-quests --lib storage_owner_ -- --nocapture
```

The NPC test remains RED: command 2603 hi, Rejected != Ok at classic_quests.rs:168; 114.00s test, no completion marker. Traced CalendarMonth plus outside-August EndConversation 1000ms. The three pure storage-owner regression tests PASS; no guard/assertion changed.

Current checks: initial dispatch cargo check PASS; workspace fmt PASS 11.765s; explicit mounted-file fmt PASS 0.033s; dispatch+quests all-target/all-feature Clippy -D warnings PASS 1.924s; source qualification units 7/7 PASS; both coverage/qualification validators PASS. Commands/logs/results/hashes:restart5-validation.json.

Unfinished:

- 155 nonexcluded full quest paths remain blocked at the exact obligations listed per title; no new NPC exchange/calendar/outfit/story engine was implemented by this task-record continuation.
- Hot Cuisine aggregate PG failure remains as above; whole story/World qualification is not inferred from reward-source scenarios.
- Inherited staged World pin work, live migration allocation, independent exact-candidate review, CI and integration remain outside this local record-only result. No frozen candidate or mergeability claim.

AUTHORING; unchanged detached HEAD 1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7. No Git/index/publication/credential/homeCodex/Docker/dependency mutation.
