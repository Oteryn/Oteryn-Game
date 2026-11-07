Corruptor vocation rewards now pass actual kill, framed Quick Loot, equipment, frozen-loot retry and corpse recovery/retirement cases. Mintwallin Key3620 has fresh exact-source USE/replay/refusal proof and its ledger promotion. These are bounded owner/source qualifications; full Newhaven/Mintwallin World admission and complete A..M storylines are not certified.

Retained Phase A:12 cohorts/56 reward sets/104 claims/106 placements. Added Mintwallin key:1 claim/1 placement. Global source inventory:127→128 qualified,207→206 unqualified,2 excluded.

Changed product files (14):

- `content/content.lock.json`
- `content/items/definitions/spell-native-profiles.json`
- `content/items/index.json`
- `content/manifest.json`
- `content/quests/grizzly-boss-rooms.json`
- `content/spells.manifest.json`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/newhaven_corruptor.rs`
- `docs/agents/evidence/quest-claim-qualification.json`
- `docs/repository/QUEST_CALLBACK_COVERAGE.json`
- `tools/content-migration/quest_finish_newhaven_items.py`
- `tools/content-migration/test_quest_finish_newhaven_items.py`
- `tools/content-schema/quest-authoring/check_quest_claim_qualification.py`
- `tools/content-schema/quest-authoring/quest_callback_coverage.py`
- `tools/content-schema/quest-authoring/test_quest_claim_qualification.py`

Checkpoint/evidence files: `.questpc/RESUME.md`, `.questpc/SHARED-FIXES.md`, `restart3-validation.json`, `restart3-review.md`, retained command logs/results and the two serial check runners.

Exact PostgreSQL command filters (all absolute queued Cargo, locked/offline,4 jobs,serial tests):

`-p oteryn-gameplay-dispatch --all-features --lib -- newhaven_corruptor_ viola_mission_yes viola_book_postgres_real_use_repeat_replay_refusal_and_fences --test-threads=1 --nocapture`

PASS9 selected/0fail in908.49s tests/909.696s wall:7actual PG cases,1guarantee unit,1internal child-process helper. The helper alone is not PG qualification. All11actual PG markers have positive durations.

`-p oteryn-gameplay-dispatch --all-features --lib quest_catalogue_chests_mintwallin_prison_key3620_use_replay_and_refusal -- --test-threads=1 --nocapture`

PASS1/0 before promotion, marker5.630022771s; PASS1/0 after promotion, marker5.542130326s/30.33s tests/55.148s wall.

Rust unit filters: `-p oteryn-gameplay-content --lib equipment_projection_tests` (1pass); `-p oteryn-gameplay-quests --lib tutorial_corruptor_rewards_use_source_items_and_base_vocations` (1pass).

Final checks: dispatch+content all-target/all-feature Clippy -Dwarnings PASS18.203s after promotion; workspacefmt PASS10.197s and explicit mounted-filefmt PASS; Newhaven authoring10/10, Grizzly1/1, qualification7/7 and governance129/129 PASS. Nativeproducer/check PASS; exact baseline artifact comparison proves only12selected profiles container/equipment groups changed. Coverage/ledger baseline comparison proves only Key3620 and derived totals changed. All original assertions retained; new strict boundary witness observedRED841!=840 thenGREEN. No new public RustAPI, enum,field,trait method,command,capability,migration or dependency.

Unfinished:

- Original aggregate scoped NPC case: JeanPierre command2603 hi Rejected!=Ok; authoritative calendar owner absent; no source guards relaxed
- Remaining complete A..M item/trade/outfit/story/World paths need the native owners documented in restart2-phase-b-c-blockers.md and QENGINE integration
- Coordinator staged Worldpin refresh, migration allocation, independent exact-candidate review and CI/publication; no Git mutation authorized

Local AUTHORING; no frozen candidate, no CI/mergeability claim. Exact source hashes, commands, exit statuses, durations and proof locators: `restart3-validation.json`.
