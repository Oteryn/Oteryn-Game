# Restart4 bounded reward-source qualification

PARTIAL: 44 additional A..M reward claims qualified; full quests remain separately unqualified.

Final3/3actual PostgreSQL cohorts:14reward sets,46claims,55native-source placements,allpositive-duration markers. Two Dreamer claims retain their older qualification locators. Global172qualified/162remaining/2excluded; canonicalA..M150qualified/0remaining/2Dawnport-excluded. No new whole-quest flag or World admission certificate.

Changed product files (6):

- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs`
- `docs/agents/evidence/quest-claim-qualification.json`
- `docs/repository/QUEST_CALLBACK_COVERAGE.json`
- `tools/content-schema/quest-authoring/check_quest_claim_qualification.py`
- `tools/content-schema/quest-authoring/quest_callback_coverage.py`
- `tools/content-schema/quest-authoring/test_quest_claim_qualification.py`

Scratch/checkpoint files: `.questpc/RESUME.md`, `restart4-*` scripts, manifests, logs, results, review and qualification evidence. No shared-fixture fix; `.questpc/SHARED-FIXES.md` unchanged.

Exact final cargo command (CARGO_BUILD_JOBS=4):

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib -- quest_catalogue_chests_botany_arena_beggar_children_cults_use_replay_and_refusal quest_catalogue_chests_dark_trails_dreamer_druid_ferumbras_knowledge_use_replay_and_refusal quest_catalogue_chests_hidden_threats_hunter_knight_koshei_use_replay_and_refusal --test-threads=1 --nocapture
```

Final wall duration: 388.003s. Log: `.questpc/restart4-story-reward-pg-4.log`; every placement duration: `.questpc/restart4-story-reward-pg-4.qualified.json`.

Qualified reward sets by named test (reward sources only):

- `quest_catalogue_chests_botany_arena_beggar_children_cults_use_replay_and_refusal`: an_interest_in_botany_quest, barbarian_arena_quest, beggar_outfits_quest, children_of_the_revolution_quest, cults_of_tibia_quest
- `quest_catalogue_chests_dark_trails_dreamer_druid_ferumbras_knowledge_use_replay_and_refusal`: dark_trails_quest, dreamer_s_challenge_quest, druid_outfits_quest, ferumbras_ascension_quest, forgotten_knowledge_quest
- `quest_catalogue_chests_hidden_threats_hunter_knight_koshei_use_replay_and_refusal`: hidden_threats_quest, hunter_outfits_quest, knight_outfits_quest, koshei_the_deathless_quest

Validation: dispatch all-target/all-feature Clippy -Dwarnings PASS; workspace and explicit mounted-file format checks PASS; qualification7/7 and governance129/129 PASS; both artifact validators PASS. Initial dispatch check passed before source edits. No new enum/field/trait/publicAPI, command/capability, migration, dependency or runtime change. Exact counts, timings and hashes: restart4-validation.json. Metadata structural comparison: restart4-promotion.json. Self-review and inherited review limits: restart4-review.md.

Unfinished:

- Inherited aggregate NPC case catalogue_npc_scoped_a_m_starts_are_source_ordered_and_atomic: command2603 hi to JeanPierre, Rejected!=Ok at classic_quests.rs168; authoritative calendar composition absent. Prior exact failed output retained; original test and guards unchanged; not rerun by this reward-only delta.
- Remaining complete NPC item/XP/outfit/story and real World access/admission paths; source claims do not certify these. See restart2-phase-b-c-blockers.md.
- QENGINE-owned shared movement/encounter/reset features and their quest composition; not implemented here.
- Coordinator staging/World-pin refresh, live migration allocation, independent exact-candidate review, CI and integration of the inherited authoring tree. No Git/index/publication mutation authorized or performed.

Local AUTHORING; no frozen SHA, current CI or mergeability claim.
