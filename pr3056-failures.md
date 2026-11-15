# PR #3056 CI failures (head ff0fb599754fa11ac9be0a7b246331ee9d7c9d07)

Source: run 43414198089 (PR PostgreSQL harness; 22 failed partitions, partitions 1 and 14 cancelled, Node boot failed) and run 43414197967 (Merge gate). Logs read via GitHub MCP get_job_logs. The tool caps tail at 5000 lines; partitions 5, 10, 11, 15, 17, 22 have longer logs (5324-19658 lines), so their nextest summary FAIL lists are complete but the failure blocks of the earliest-failing tests were outside the tail (class Z).

**Total failing tests: 117** across 22 partitions (all crate oteryn-gameplay-dispatch; all under gameplay_transport::monk_save::postgres_tests except quest_admission, which is gameplay_transport::quest_admission_postgres_tests). No TIMEOUT, stack overflow, compile error, missing-table or migration-checksum failure in any nextest summary.

Test paths below omit the prefix `gameplay_transport::monk_save::postgres_tests::`; source locations omit `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/`. Partition numbers in brackets.

## Class summary

| Class | Count |
|---|---|
| A. Quest source eligibility sees durable starting level 8 (fixture level not applied) | 9 |
| B. Boss/endgame entry refused with content refusal (ContentUnavailable/ContentChanged, wire [8,2,32,4/5]) instead of expected Accepted/Level(12) | 10 |
| C. Native death drain stays Deferred instead of Drained | 6 |
| D. Movement/STEP/push/teleport Rejected (expected Moved/Occupied) | 10 |
| E. Use/claim/command Rejected where Accepted/Committed/Cast/Ok expected (reward chests, claims, flowers, markers, outfit hand-ins) | 10 |
| F. Native predecessor made no progress at fixed clock (srx3 primal) | 4 |
| G. "no command result" (srx3 cruelty / soul-war source use) | 6 |
| H. Lifecycle hook must recover last disconnected actor without a native tick | 2 |
| I. Copied source events did not finish without advancing time (srx3_native_source_cast.rs:66) | 2 |
| J. Bakragore FIRST Think did not capture historical flavor (left [] right [1]) | 3 |
| K. Native USE: unexpected end of file | 2 |
| L. Expected durable/native effect not committed (count left 0 vs right >=1) | 10 |
| M. Wrong refusal kind (NothingToUse/StaleState/false vs expected) | 4 |
| N. "actual Sorrow Item owner" (srx3 hatred entry) | 2 |
| O. Other single-instance failures | 20 |
| Z. Message not retrievable (failure block outside the 5000-line log tail) | 17 |

## A. Quest source eligibility sees durable starting level 8 (fixture level not applied) (9)

Partitions: 4, 6, 11, 13, 16, 18, 19

Representative: `thread 'questpc7-grave' (8069) panicked at crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs:1776:13: assertion 'left == right' failed: source eligibility uses the actual durable starting level | left: 8 | right: 250`

- [4] `questpc7_grave::questpc7_grave_whole_native_journey_and_reward` -- quest_world_callbacks/scenario.rs:1776:13 assertion 'left == right' failed: source eligibility uses the actual durable starting level | left: 8 | right: 250
- [6] `questpc7_heart_story::questpc7_heart_whole_native_breaches_and_rewards` -- quest_world_callbacks/scenario.rs:1776:13 assertion 'left == right' failed: source eligibility uses the actual durable starting level | left: 8 | right: 150
- [18] `npc_program::catalogue_npc_scoped_a_m_starts_are_source_ordered_and_atomic` -- quest_world_callbacks/scenario.rs:1776:13 assertion 'left == right' failed: source eligibility uses the actual durable starting level | left: 8 | right: 35
- [13] `questpc7_feaster_thaian::questpc7_feaster_thaian_native_admission_preserves_source_facts` -- quest_world_callbacks/scenario.rs:1776:13 assertion 'left == right' failed: source eligibility uses the actual durable starting level | left: 8 | right: 250
- [13] `questpc7_outfit_items::questpc7_fiend_personal_tokens_require_actual_native_addons` -- quest_world_callbacks/scenario.rs:1776:13 assertion 'left == right' failed: source eligibility uses the actual durable starting level | left: 8 | right: 250
- [16] `questpc7_ferumbras::questpc7_ferumbras_full_native_seals_and_reward_bundle` -- quest_world_callbacks/scenario.rs:1776:13 assertion 'left == right' failed: source eligibility uses the actual durable starting level | left: 8 | right: 150
- [16] `questpc7_outfit_items::questpc7_inquisition_personal_tokens_require_actual_native_addons` -- quest_world_callbacks/scenario.rs:1776:13 assertion 'left == right' failed: source eligibility uses the actual durable starting level | left: 8 | right: 250
- [11] `npc_program::catalogue_npc_calendar_month_runs_in_real_dispatch` -- quest_world_callbacks/scenario.rs:1776:13 assertion 'left == right' failed: source eligibility uses the actual durable starting level | left: 8 | right: 35
- [19] `questpc7_eligibility::questpc7_reward_requirements_high_level_and_readonly_parent` -- quest_world_callbacks/scenario.rs:1776:13 assertion 'left == right' failed: source eligibility uses the actual durable starting level | left: 8 | right: 1000

## B. Boss/endgame entry refused with content refusal (ContentUnavailable/ContentChanged, wire [8,2,32,4|5]) instead of expected Accepted/Level(12) (10)

Partitions: 5, 8, 12, 13, 15, 23

Representative: `thread '<unnamed>' (7687) panicked at crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/boss_activity.rs:1315:21: assertion 'left == right' failed: boss command 5, session GameSessionId([50, 2, 3, 4, 5, 6, 112, 8, 128, 10, 11, 12, 13, 14, 15, 50]), generation 1: Ok(Refused(ContentUnavai`

- [5] `boss_activity::l3_endgame_quest_entry_rotten_blood_requires_level` -- boss_activity.rs:1315:21 assertion 'left == right' failed: boss command 5, session GameSessionId([50, 2, 3, 4, 5, 6, 112, 8, 128, 10, 11, 12, 13, 14, 15, 50]), generation 1: Ok(Refused(ContentUnavailable)); raw payload [8, 2, 32, 4] | left: Reje
- [12] `boss_activity::l3_instance_endgame_rotten_blood_native_death_progresses_once` -- boss_activity.rs:1315:21 assertion 'left == right' failed: boss command 5, session GameSessionId([50, 2, 3, 4, 5, 6, 112, 8, 128, 10, 11, 12, 13, 14, 15, 50]), generation 1: Ok(Refused(ContentUnavailable)); raw payload [8, 2, 32, 4] | left: Reje
- [12] `m26_spite_chain::m26_spite_production_entry_fight_quest_reward_custody_and_return` -- boss_activity.rs:1315:21 assertion 'left == right' failed: boss command 1, session GameSessionId([50, 2, 3, 4, 5, 6, 112, 8, 128, 10, 11, 12, 13, 14, 15, 50]), generation 1: Ok(Refused(ContentUnavailable)); raw payload [8, 2, 32, 4] | left: Reje
- [12] `srx3_rotten_callbacks::srx3_murcion_published_whole_world_entry_native_children_postgres` -- boss_activity.rs:1315:21 assertion 'left == right' failed: boss command 1, session GameSessionId([50, 2, 3, 4, 5, 6, 112, 8, 128, 10, 11, 12, 13, 14, 15, 50]), generation 1: Ok(Refused(ContentChanged)); raw payload [8, 2, 32, 5] | left: Rejected
- [8] `boss_activity::l3_endgame_quest_entry_soul_war_child_requires_level` -- boss_activity.rs:1315:21 assertion 'left == right' failed: boss command 4, session GameSessionId([50, 2, 3, 4, 5, 6, 112, 8, 128, 10, 11, 12, 13, 14, 15, 50]), generation 1: Ok(Refused(ContentUnavailable)); raw payload [8, 2, 32, 4] | left: Reje
- [13] `endgame_entry::endgame_entry_postgres_real_dispatch_cannot_allocate_or_reset_without_w02` -- endgame_entry.rs:208:13 assertion 'left == right' failed | left: (Rejected, [8, 2, 32, 5]) | right: (Rejected, [8, 2, 32, 12])
- [15] `boss_activity::l3_instance_endgame_primal_native_death_unlocks_hazard_once` -- boss_activity/primal.rs:116:5 assertion 'left == right' failed | left: Refused(ContentChanged) | right: Refused(Level)
- [15] `m26_catalogue_aliases::m26_soul_war_postgres_all_six_source_aliases_reach_real_entry_gates` -- m26_catalogue_aliases.rs:62:13 assertion 'left == right' failed: oteryn:boss.goshnar_s_spite | left: (Rejected, [8, 2, 32, 4]) | right: (Rejected, [8, 2, 32, 12])
- [23] `boss_activity::l3_instance_endgame_soul_war_native_death_progresses_once` -- boss_activity.rs:1315:21 assertion 'left == right' failed: boss command 4, session GameSessionId([50, 2, 3, 4, 5, 6, 112, 8, 128, 10, 11, 12, 13, 14, 15, 50]), generation 1: Ok(Refused(ContentUnavailable)); raw payload [8, 2, 32, 4] | left: Reje
- [23] `m26_static_instance::m26_spite_postgres_static_instance_map_needs_no_channel_item_custody` -- boss_activity.rs:1315:21 assertion 'left == right' failed: boss command 1, session GameSessionId([50, 2, 3, 4, 5, 6, 112, 8, 128, 10, 11, 12, 13, 14, 15, 50]), generation 1: Ok(Refused(ContentUnavailable)); raw payload [8, 2, 32, 4] | left: Reje

## C. Native death drain stays Deferred instead of Drained (6)

Partitions: 2, 7, 9, 18, 21

Representative: `thread 'srvenc_m24_postgres_world_devourer_room' (12301) panicked at crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/srvenc_source_deaths.rs:521:9: assertion 'left == right' failed | left: Deferred | right: Drained`

- [2] `srvenc_source_deaths::srvenc_m24_postgres_world_devourer_room` -- srvenc_source_deaths.rs:521:9 assertion 'left == right' failed | left: Deferred | right: Drained
- [7] `questpc7_hero::questpc7_hero_full_native_investigation` -- questpc7_native_deaths.rs:195:5 assertion 'left == right' failed | left: Deferred | right: Drained
- [18] `boss_activity::l3_instance_endgame_soul_war_final_native_death_progresses_once` -- boss_activity/endgame.rs:175:5 assertion 'left == right' failed | left: Deferred | right: Drained
- [9] `questpc8_magic_sword::questpc8_magic_sword_two_and_four_native_demons_both_caches_once` -- grizzly_npc/questpc8_campaign.rs:435:5 assertion 'left == right' failed | left: Deferred | right: Drained
- [21] `questpc7_forgotten_story::questpc7_forgotten_whole_native_paths_and_learning` -- questpc7_native_deaths.rs:195:5 assertion 'left == right' failed | left: Deferred | right: Drained
- [21] `srvenc_source_deaths::srvenc_m24_postgres_anomaly_room` -- srvenc_source_deaths.rs:521:9 assertion 'left == right' failed | left: Deferred | right: Drained

## D. Movement/STEP/push/teleport Rejected (expected Moved/Occupied) (10)

Partitions: 3, 4, 6, 8, 11, 17, 18, 19, 21

Representative: `thread '<unnamed>' (7866) panicked at crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/annihilator_door.rs:49:5: assertion 'left == right' failed | left: Rejected | right: Moved`

- [4] `quest_container_chests::annihilator_reward_door_tracks_the_real_shared_claim` -- annihilator_door.rs:49:5 assertion 'left == right' failed | left: Rejected | right: Moved
- [4] `quest_reward_claims::quest_reward_claims_naginata_pick_six_shared_hits_and_replay` -- quest_reward_claims.rs:881:5 assertion 'left == right' failed | left: Rejected | right: Moved
- [6] `creature_push::reaudit_relocation_tiles_monster_push_wolf_capture_postgres` -- creature_push.rs:595:5 assertion 'left == right' failed | left: Rejected | right: Moved
- [8] `desert_earned_route::desert_dungeon_native_gate_rite_earned_rewards_and_exit` -- desert_earned_route.rs:273:5 assertion 'left == right' failed: literal source STEP must move | left: Rejected | right: Moved
- [17] `quest_reward_claims::quest_reward_claims_panpipe_locked_door_to_reward_and_replay` -- quest_reward_claims.rs:881:5 assertion 'left == right' failed | left: Rejected | right: Moved
- [18] `m07_machinery_repairs::m07_machinery_postgres_repair_consumes_exact_budget_and_keeps_walls_solid` -- m07_machinery_repairs.rs:61:9 assertion 'left == right' failed: never repair a wall onto an actor | left: Rejected | right: Occupied
- [3] `creature_push::reaudit_relocation_tiles_monster_push_trap_entry_exit_postgres` -- creature_push.rs:595:5 assertion 'left == right' failed | left: Rejected | right: Moved
- [11] `questpc6_access::questpc6_dreamers_challenge_exit_and_maze_passages_replay` -- questpc6_access.rs:287:32 assertion 'left == right' failed | left: Rejected | right: Moved
- [19] `srx3_rotten_advanced::srx3_vemiath_actual_source_teleport_exit_return_postgres` -- srx3_vemiath_return.rs:68:9 assertion 'left == right' failed | left: Rejected | right: Moved
- [21] `srx3_rotten_callbacks::srx3_mortal_essence_plain_corpse_enters_native_ground_custody_postgres` -- srx3_rotten_callbacks.rs:1091:21 assertion 'left == right' failed: actual native command9 pickup must normalize the source runtime coordinates under the current owner | left: Refused(Rejected) | right: Committed

## E. Use/claim/command Rejected where Accepted/Committed/Cast/Ok expected (reward chests, claims, flowers, markers, outfit hand-ins) (10)

Partitions: 4, 5, 9, 12, 13, 16, 18, 21, 24

Representative: `thread '<unnamed>' (4455) panicked at crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/pvp_live.rs:629:13: assertion 'left == right' failed | left: Rejected | right: Cast`

- [4] `pvp_live::pvp_live_bundle_melee_spell_death_replay_and_reload` -- pvp_live.rs:629:13 assertion 'left == right' failed | left: Rejected | right: Cast
- [4] `definition_ready_claims::definition_ready_claim_bloodherb_use_and_replay_dispatch_postgres` -- quest_callbacks.rs:896:13 assertion 'left == right' failed | left: [Rejected, Rejected] | right: [Committed, NothingToUse]
- [5] `quest_catalogue_chests::quest_catalogue_chests_botany_arena_beggar_children_cults_use_replay_and_refusal` -- quest_container_chests.rs:822:21 assertion 'left == right' failed: oteryn:reward-claim.quest.u8_54.children_of_the_revolution.chest_tome_of_knowledge1 at (33247, 31161, -6) | left: [(Accepted, Committed), (DuplicateReplay, Committed), (Rejected, Rejecte
- [12] `questpc6_access::questpc6_source_narrative_caches_children_cults_dark_trails_dreamer_druid` -- quest_container_chests.rs:822:21 assertion 'left == right' failed: oteryn:reward-claim.quest.u8_54.children_of_the_revolution.chest_tome_of_knowledge1 at (33247, 31161, -6) | left: [(Accepted, Committed), (DuplicateReplay, Committed), (Rejected, Rejecte
- [18] `engoutfit::engoutfit_rock_steady_collection_commits_exact_handins_and_weapon` -- classic_quests.rs:238:5 assertion 'left == right' failed: 2912: collect | left: Rejected | right: Ok
- [9] `source_quest_rewards::tomes_of_knowledge_rewards` -- source_quest_rewards.rs:369:13 assertion 'left == right' failed: oteryn:reward-claim.quest.u8_54.tomes_of_knowledge.chest_tome_of_knowledge1 uid=6292 cold=false | left: [(Rejected, Rejected), (DuplicateReplay, Rejected), (Rejected, Rejected)] | right:
- [13] `m16_personal_map_marker::m16_sterling_commits_personal_marker_and_delivers_delta_and_reconnect_snapshot` -- m16_personal_map_marker.rs:250:21 assertion 'left == right' failed: yes | left: Rejected | right: Accepted
- [16] `m07_lion_offerings::m07_lion_flower_east_postgres_bloom_reset_and_repeatable_pickup` -- m07_lion_offerings.rs:255:9 assertion 'left == right' failed: flower command 0 | left: Rejected | right: Committed
- [24] `definition_ready_claims::definition_ready_claim_use_and_replay_dispatch_postgres` -- quest_callbacks.rs:896:13 assertion 'left == right' failed | left: [Rejected, Rejected] | right: [Committed, NothingToUse]
- [21] `m07_lion_offerings::m07_lion_flower_west_postgres_bloom_reset_and_repeatable_pickup` -- m07_lion_offerings.rs:255:9 assertion 'left == right' failed: flower command 0 | left: Rejected | right: Committed

## F. Native predecessor made no progress at fixed clock (srx3 primal) (4)

Partitions: 2, 4, 15, 24

Representative: `Error: "actual native predecessor made no progress at fixed clock 12891306; turn=1; original=None; profile=None; normal=None; deferred=[]; pods=[]; executions=[]; expiries=[]; native=NativeCallbacks { owner: Mutex { data: NativeCallbackState { spawns: NativeSpawns { pending: 0, .. }, time_guardian: `

- [4] `srx3_primal_death_cutoff::srx3_primal_actual_normal_map_sql_wait_releases_runtime_owner_postgres` --  Error: "actual native predecessor made no progress at fixed clock 12891306; turn=1; original=None; profile=None; normal=None; deferred=[]; pods=[]; executions=[]; expiries=[]; native=NativeCallbacks { owner: Mutex { data
- [2] `srx3_primal_death_cutoff::srx3_primal_actual_source_deferred_tasks_precede_physical_death_cutoff_postgres` --  Error: "actual native predecessor made no progress at fixed clock 18198799; turn=1; original=None; profile=None; normal=None; deferred=[]; pods=[]; executions=[]; expiries=[]; native=NativeCallbacks { owner: Mutex { data
- [15] `srx3_primal::srx3_primal_real_source_entry_wave_pods_and_terminal_postgres` --  Error: "actual native predecessor made no progress at fixed clock 127633571; turn=1; original=None; profile=None; normal=None; deferred=[]; pods=[]; executions=[]; expiries=[]; native=NativeCallbacks { owner: Mutex { dat
- [24] `srx3_primal_death_cutoff::srx3_primal_actual_queued_pod_expiry_precedes_core_without_sweeping_future_postgres` --  Error: "actual native predecessor made no progress at fixed clock 17067665; turn=1; original=None; profile=None; normal=None; deferred=[]; pods=[]; executions=[]; expiries=[]; native=NativeCallbacks { owner: Mutex { data

## G. "no command result" (srx3 cruelty / soul-war source use) (6)

Partitions: 3, 7, 9, 12, 18, 23

Representative: `Error: "no command result"`

- [12] `srx3_cruelty_callbacks::srx3_cruelty_actual_source_entry_and_native_defense_chain_postgres` --  Error: "no command result"
- [7] `srx3_soul_war_source_use::srx3_soul_war_sanity_prewrite_loss_retains_no_value_and_completes_ingress_postgres` --  Error: "no command result"
- [18] `srx3_cruelty_callbacks::srx3_cruelty_canceled_after_write_reconciles_source_effect_once_postgres` --  Error: "no command result"
- [9] `srx3_soul_war_source_use::srx3_soul_war_sanity_wrong_native_target_commits_cooldown_without_burn_postgres` --  Error: "no command result"
- [23] `srx3_cruelty_callbacks::srx3_cruelty_canceled_before_names_drains_original_source_use_postgres` --  Error: "no command result"
- [3] `srx3_cruelty_callbacks::srx3_cruelty_completed_handoff_retry_preserves_source_effect_once_postgres` --  Error: "no command result"

## H. Lifecycle hook must recover last disconnected actor without a native tick (2)

Partitions: 2, 6

Representative: `thread 'srx3-sanity-source-use' (12773) panicked at crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/srx3_soul_war_source_use.rs:370:9: the lifecycle hook must recover the last disconnected actor without a native tick`

- [2] `srx3_soul_war_source_use::srx3_soul_war_sanity_canceled_commit_reconciles_original_receipt_after_loss_postgres` -- srx3_soul_war_source_use.rs:370:9 the lifecycle hook must recover the last disconnected actor without a native tick
- [6] `srx3_soul_war_source_use::srx3_soul_war_sanity_completed_handoff_retry_completes_original_ingress_postgres` -- srx3_soul_war_source_use.rs:370:9 the lifecycle hook must recover the last disconnected actor without a native tick

## I. Copied source events did not finish without advancing time (srx3_native_source_cast.rs:66) (2)

Partitions: 2, 4

Representative: `thread 'srx3-cruelty-uncredited-corpse' (8505) panicked at crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/srx3_native_source_cast.rs:66:5: all copied source events must finish without advancing time`

- [4] `srx3_cruelty_uncredited_corpse::srx3_cruelty_native_eye_uncredited_poor_soul_corpse_is_pickupable_and_replay_bound_postgres` -- srx3_native_source_cast.rs:66:5 all copied source events must finish without advancing time
- [2] `srx3_rotten_callbacks::srx3_soul_war_actual_plain_child_corpse_materialization_postgres` -- srx3_native_source_cast.rs:66:5 all copied source events must finish without advancing time

## J. Bakragore FIRST Think did not capture historical flavor (left [] right [1]) (3)

Partitions: 9, 10, 17

Representative: `thread 'srx3-rotten-advanced' (11743) panicked at crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/srx3_rotten_advanced.rs:589:21: assertion 'left == right' failed: only actual FIRST Think captures the historical flavor | left: [] | right: [1]`

- [17] `srx3_rotten_death::srx3_bakragore_uncredited_death_retains_actual_first_think_essence_postgres` -- srx3_rotten_advanced.rs:589:21 assertion 'left == right' failed: only actual FIRST Think captures the historical flavor | left: [] | right: [1]
- [10] `srx3_rotten_death::srx3_bakragore_full_reward_queue_finishes_copied_death_without_mint_or_xp_postgres` -- srx3_rotten_advanced.rs:589:21 assertion 'left == right' failed: only actual FIRST Think captures the historical flavor | left: [] | right: [1]
- [9] `srx3_rotten_death::srx3_bakragore_credited_death_retains_actual_first_think_essence_postgres` -- srx3_rotten_advanced.rs:589:21 assertion 'left == right' failed: only actual FIRST Think captures the historical flavor | left: [] | right: [1]

## K. Native USE: unexpected end of file (2)

Partitions: 4, 16

Representative: `Error: "native USE 12204 command 1: unexpected end of file"`

- [4] `questpc8_colours_opening::questpc8_colours_native_envelope_and_self_marking_postgres` --  Error: "native USE 12204 command 1: unexpected end of file"
- [16] `authored_quest_bindings::authored_bindings_npc_delivery_and_carried_use_dispatch_postgres` --  Error: "native USE 3031 command 4: unexpected end of file"

## L. Expected durable/native effect not committed (count left 0 vs right >=1) (10)

Partitions: 2, 7, 8, 11, 17, 23

Representative: `thread 'engworldchg-guarded-sale' (7893) panicked at crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/engworldchg_guarded_sale.rs:377:13: assertion 'left == right' failed | left: 0 | right: 1`

- [2] `engworldchg_guarded_sale::engworldchg_guarded_wood_exchange_real_dispatch_postgres` -- engworldchg_guarded_sale.rs:377:13 assertion 'left == right' failed | left: 0 | right: 1
- [2] `questpc7_grimvale_story::questpc7_grimvale_whole_native_investigation_and_rewards` -- questpc7_grimvale_story.rs:244:9 assertion 'left == right' failed | left: 0 | right: 1
- [2] `worldchg_grimvale::qn5_feroxa_crowded_corpses_clear_in_bounded_recoverable_batches` -- worldchg_grimvale.rs:1022:17 assertion 'left == right' failed: first bounded batch must actually commit, not roll back forever | left: 0 | right: 1
- [8] `srx3_megalomania_callbacks::srx3_megalomania_actual_source_entry_and_initial_aspect_postgres` -- srx3_megalomania_callbacks.rs:111:17 assertion 'left == right' failed: actual committed source entry creates exact Purple and configured Aspect | left: 0 | right: 1
- [8] `srx3_source_death_quest::srx3_source_death_real_malice_lethal_runs_copied_goal_and_preserves_pre_taint_xp_postgres` -- srx3_source_death_quest.rs:228:13 assertion 'left == right' failed: copied SoulWarBossesDeath must award Malice before releasing the ordinary drain | left: 0 | right: 1
- [17] `srx3_rotten_advanced::srx3_vemiath_actual_source_portal_step_postgres` -- srx3_rotten_portal_step.rs:66:5 assertion 'left == right' failed: actual source90s callback physically created the portal | left: 0 | right: 1
- [7] `srx3_rotten_advanced::srx3_bakragore_actual_source_native_effects_postgres` -- srx3_rotten_bak_claim.rs:11:5 assertion 'left == right' failed: entry records only origin; the actual reserved FIRST think creates exactly one global phase decision receipt | left: 0 | right: 1
- [23] `engenc2_cults::engenc2_cults_callback_native_phases_and_settlement_postgres` -- engenc2_cults.rs:334:13 assertion 'left == right' failed: source Humans.Mission reaches two | left: 0 | right: 2
- [11] `questpc7_goblin::questpc7_goblin_five_native_stalkers_credit_exact_deaths` -- questpc7_goblin.rs:417:9 assertion 'left == right' failed: duplicate lethal notifications/drains credit once | left: 0 | right: 1
- [11] `srx3_claustrophobic::srx3_claustrophobic_actual_world_step_starts_source_raid_postgres` -- srx3_claustrophobic.rs:121:13 assertion 'left == right' failed: actual committed sand-timer STEP starts one source-qualified durable World raid | left: 0 | right: 1

## M. Wrong refusal kind (NothingToUse/StaleState/false vs expected) (4)

Partitions: 5, 13, 20

Representative: `thread '<unnamed>' (9627) panicked at crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/m07_lion_offerings.rs:148:5: assertion 'left == right' failed | left: NothingToUse | right: Rejected`

- [5] `m07_lion_offerings::m07_lion_tears_postgres_exact_consumption_counter_and_pagoda_reset` -- m07_lion_offerings.rs:148:5 assertion 'left == right' failed | left: NothingToUse | right: Rejected
- [13] `m07_lion_offerings::m07_lion_beauty_postgres_exact_consumption_counter_and_pagoda_reset` -- m07_lion_offerings.rs:148:5 assertion 'left == right' failed | left: NothingToUse | right: Rejected
- [13] `srx3_omrafir_beam::srx3_omrafir_actual_native_exact_stone_wall_heal_postgres` -- srx3_omrafir_beam.rs:127:17 assertion 'left == right' failed: only actual1487/1492/1500 stack qualifies; Rosha ordinary ground never substitutes fire | left: false | right: true
- [20] `m09_treasure_rewards::m09_treasure_telescope_once_and_replay` -- m09_treasure_rewards.rs:116:5 assertion 'left == right' failed | left: StaleState | right: Rejected

## N. "actual Sorrow Item owner" (srx3 hatred entry) (2)

Partitions: 16, 21

Representative: `Error: "actual Sorrow Item owner"`

- [16] `srx3_hatred_entry::srx3_hatred_instance_sorrow_survives_later_source_channel_activation_postgres` --  Error: "actual Sorrow Item owner"
- [21] `srx3_hatred_entry::srx3_hatred_real_source_pad_resets_torment_before_native_handoff_postgres` --  Error: "actual Sorrow Item owner"

## O. Other single-instance failures (20)

Partitions: 2, 3, 5, 6, 8, 9, 10, 11, 13, 15, 17, 19, 20, 21, 24

- [2] `creature_push::creature_push_real_dispatch_moves_and_refuses_on_postgres` -- creature_push.rs:454:5 assertion 'left == right' failed | left: 3 | right: 2
- [5] `srx3_soulpit_native::srx3_soulpit_native_use_scope_barrier_and_spawn_postgres` -- srx3_soulpit_native.rs:652:13 a retained real Source spawn cell is deterministically unoccupied
- [6] `engworldchg_journals::engworldchg_fire_goal_achievement_atomic_postgres` -- engworldchg_journals.rs:936:9 the native journal death settles
- [6] `srx3_rotten_callbacks::srx3_symbol_native_corpse_actual_step_fuses_remorse_and_ground_removal_postgres` --  Error: "Native(CapacityExceeded)"
- [8] `npc_paid_travel::npc_paid_travel_postgres_dispatch_atomic_payment_arrival_replay_and_refusals` --  Error: "post-travel walk handoff"
- [17] `quest_admission::quest_admission_replaced_generation_refuses_award` -- crates/gameplay/dispatch/src/gameplay_transport/quest_admission_postgres_tests.rs:331:21 assertion failed: owner.states.quest_sessions.lock().map_err(debug)?.get(&session).ok_or("entry")?.retry_at.is_none()
- [10] `srx3_taints::srx3_taints_postgres_census_over_one_work_page_keeps_every_current_player` --  Error: "genuine fresh player 46: genuine Fresh admission not committed:RejectedStaleAuthority"
- [10] `worldchg_grimvale::qn5_feroxa_oversized_tree_keeps_root_live_until_bounded_leaves_commit` -- worldchg_grimvale.rs:1046:17 bounded leaf batches must finish the tree
- [10] `srx3_soul_war_world_entrance::srx3_soul_war_actual_world_hunt_entry_rejects_missing_quest_access_postgres` -- srx3_soul_war_world_entrance.rs:105:13 assertion 'left == right' failed: actual hunt entrance returns an access-less current Player to the captured from-position | left: Some([33615, 31422, 10]) | right: Some([33615, 31421, 10])
- [9] `srx3_spite_global::srx3_weeping_actual_death_step_outfit_and_later_item_replay_postgres` --  Error: "no rows returned by a query that expected to return at least one row"
- [13] `srx3_greed_retention::srx3_greed_original_callbacks_survive_timer_and_transform_cancellation_postgres` -- srx3_greed_retention.rs:415:9 assertion failed: states.native_profile_pass.pending().is_some_and(|current| | Arc::ptr_eq(&pass, &current))
- [15] `annihilator_route::annihilator_source_route_native_combat_clearance` -- annihilator_combat.rs:322:5 real source AI must damage the participant
- [15] `qengine_m03_use::m03_time_machine_native_use_live_guards_replay_and_free_return_postgres` -- qengine_m03_use.rs:326:5 ordinary teleport still refuses the blocked script pad
- [3] `citizen_qualification::w23_all_citizen_uniques_and_svargrond_quest_boundary` --  Error: "citizen step"
- [3] `srx3_soul_war_attacks::srx3_soul_war_native_attack_suffix_survives_physical_cancellation_postgres` --  Error: "real native ordinary hit followed by source suffix"
- [20] `rotten_blood_progression::qengine_l13_decaying_defender_parent_and_accessories` -- rotten_blood_progression.rs:712:9 oteryn:creature.ichgahal: retain physical corpse qualification refusal
- [11] `srx3_rotten_callbacks::srx3_murcion_native_mushroom_fuse_removes_original_children_postgres` -- srx3_rotten_callbacks.rs:721:21 actual native Mushroom fuse must remove its original actor after executing source effects
- [19] `m09_rookie_reward::m09_rookie_guard_two_rewards_once_and_replay` -- m09_rookie_reward.rs:78:5 assertion 'left == right' failed | left: [] | right: [("oteryn:item.tibia.i12785", 1), ("oteryn:item.tibia.i3054", 1)]
- [24] `engworldchg::engworldchg_hive_binding_reset_and_native_use_postgres` -- engworldchg.rs:139:5 native activation must unlock the quest writer: Refused(NotSupported)
- [21] `srx3_soulpit_native::srx3_soulpit_native_early_exit_retires_private_encounter_postgres` -- srx3_soulpit_native/early_exit.rs:83:5 early exit must have real living opponents to retire

## Z. Message not retrievable (failure block outside the 5000-line log tail) (17)

Partitions: 5, 10, 11, 17, 22

- [5] `arena_match::arena_match_real_connection_status_and_full_match`
- [17] `srvgap2_primal_chain::srvgap2_primal_production_entry_cadence_death_settlement_and_return`
- [17] `arena_match::m51_postgres_native_recreated_pillars_complete_all_ten_pits_and_reward`
- [17] `engworldchg_journals::engworldchg_kingsday_native_npc_journal_postgres`
- [17] `grizzly_npc::questpc8_campaign::questpc8_hunting_campaign_native_deaths_portals_reports_and_rewards`
- [10] `arena_match::m51_postgres_native_portals_complete_all_ten_pits_and_reward`
- [22] `engnpc_paid_confirmation::engnpc_live_paid_quote_owns_yes_with_reward_callback`
- [22] `hireling_lifecycle::srx3_rotten_essence_named_house_native_blue_effect_postgres`
- [22] `m07_lion_offerings::m07_lion_strength_postgres_exact_consumption_counter_and_pagoda_reset`
- [22] `questpc6_access::questpc6_cults_orc_passages_preserve_progress_and_replay`
- [22] `questpc7_formal::questpc7_formal_exhibition_display_and_consumptive_accessories`
- [22] `questpc8_npc_program_regressions::catalogue_npc_scoped_a_m_starts_are_source_ordered_and_atomic`
- [22] `srx2_scavenge::srx2_scavenge_production_use_minted_corpse_atomic_replay_postgres`
- [22] `srx3_malice_callbacks::heal::srx3_heal_malice_native_defense_heals_named_boss_and_dispels_nearby_player_postgres`
- [22] `quest_catalogue_chests::quest_catalogue_chests_a_m_unqualified_source_placements_use_replay_and_refusal`
- [11] `source_quest_rewards::the_new_frontier_rewards`
- [11] `annihilator_route::annihilator_source_route_earned_reward_exit_and_reentry`

## Cross-cutting observations

- Failures are spread over every partition (2-9 per partition) with no single panic. The dominant pattern is content/state admission refusing what fixtures expect: boss entries refused as ContentUnavailable/ContentChanged (B), STEP/USE/claims Rejected (D, E), fixture characters at level 8 instead of the configured level (A), native deaths never drained (C) and native/source effects never committed (F, G, I, J, L). This matches the Node boot seam failures below (no reachable compiled monster spawn, Oressa absent, no walking route to a shipped citizen tile), which points at World bundle / content staging in this batch PR (3030 files changed) rather than PostgreSQL infrastructure.
- PostgreSQL server logs across the failing partitions: 18 "canceling statement due to lock timeout" (e.g. UPDATE game_character_quest_states while locking a tuple in relation "game_character_quest_states") and 2 "deadlock detected". These do not map one-to-one to the failing tests.
- Debug output left in the head: `[DBG-W11-DEATH]` (40 lines) and `PR3056_SOURCE_USE` (28 lines) show up in test stderr.
- Partitions 1 and 14 were cancelled (not failed) and were not analysed.

## PR heavy / Node boot against the real Platform (job 131847866325)

- Result: `NODE_BOOT_RESULT=NODE_BOOT_FAIL`. Only stage `seam_stages` failed (FAIL, exit 101). operator_setup, configuration_negatives, node_assigned_ready, control_socket_peer, character_bootstrap, graceful_shutdown, restart_without_supersession, superseding_replacement and signal_before_ready all PASS.
- Seam test: `test result: FAILED. 0 passed; 1 failed; ... finished in 3163.50s`; `NODE_BOOT_ROOT_CAUSES failed_rows=62 roots=56`.
- Failed scenarios (62):
  - S11: no-progress (categories).
  - W20a: world-clock deadline, sunrise not seen within the wall-time budget.
  - W20b, S13a and S13b.
  - S08: earned-levels; the hunt-walk, hunt and level acquisition steps failed.
  - P21: player-acquisition; hunt-kill had earned_xp=false.
  - P19 and P16: no-progress.
  - C20d.
  - S16a (oressa-reachable), which also blocks C20a.
  - C20b, C20c and P10.
  - C03: failed:baseline:no-reachable-compiled-monster-spawn.
  - C12: no-reachable-creature-spawn-has-content-health.
  - C09, C06, C02 and C01.
  - W22b: prey_query_returned_no_unlocked_slot, no_reachable_visible_rat.
  - C05.
  - C04: PZ cast Rejected.
  - W18.
  - C11: player-monk-choice observed Oressa absent.
  - W19a and W01b.
  - W23: no walking route to a reachable shipped citizen tile.
  - C17a, C17c, C16b, C16a, C18a, C17d, C13, C16c and C16d: required-actions-unproven.
  - E01, E03, E04, E05, E07 and S09: assertion.
  - S01 and S18: transport.
  - M01: transport, journey aborted; it blocks M02, M03, M04, M05 and M07.
  - I01, I04, I06, I07, I08, I09, I12, I13, I15 and I16: assertion, each at about 2.23 s.
- Also logged: `boot_failed code=E2004 name=BOOT_SOURCE_CUSTODY_FAILED` ("S2 custody failed at custody claim"; boot_step_failed step="custody claim" error=Unavailable). This happens inside negative/restart stages, which still report PASS.
- Reading: most roots share missing or unreachable World content (monster spawns, NPC Oressa, citizen tiles, rats). That is the same content-staging family as harness classes A/B/D/E.

## Merge gate / validate (run 43414197967, job 131873372184)

- The step "Require every applicable fast gate" failed with `Merge gate failed: - RUST_FAST=cancelled`. Every other input was success (SCOPE, LANES, GOVERNANCE, DEPENDENCY_REVIEW, ROUTING_CONTRACT, RUST_POLICY, RUST_SUPPLY_CHAIN, WORLD_BUNDLE). RUST_REQUIRED=true, so `cancelled` is not accepted. game-gate then failed on "Require canonical aggregate success" as a direct consequence.

## Merge gate / Rust changed-crate Clippy and unit tests (tests 3/3), cancel cause (job 131848714997)

- Downloading the log returns HTTP 404 (cancelled job, no log kept), so the cause below is inferred from the job metadata and the workflow.
- The job started at 11:07:15Z. Its step "Unit-test changed crates (nextest shard 3/3)" started at 11:13:30Z and was still in_progress when the job ended cancelled at 12:02:16Z.
- At the PR head, rust_fast in .github/workflows/merge-gate.yml has `timeout-minutes: 45`.
- Shards 1/3 and 2/3 spent 33 and 39 minutes in the test step alone and finished at 11:48Z and 11:51Z.
- Most likely cause: shard 3/3 hit the 45-minute job timeout. No other leg failed, fail-fast is false, and the run was not superseded. Cause class: timeout (the shard's runtime exceeds the job budget), not a test failure.
