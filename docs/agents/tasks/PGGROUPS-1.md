# PGGROUPS-1: dispatch PostgreSQL test compilation groups

Base: `e2152d050`; branch: `claude/happy-davinci-ijz3wy-pggroups`.

PostgreSQL cases remain library tests. Local developers must pass
`--features pg-tests-all` to include all PG cases, or select one of
`pg-tests-g1`, `pg-tests-g2`, `pg-tests-g3`, `pg-tests-g4`.
New PG test modules must be assigned to the smallest group by source lines.
The default features include no PG group.

## Validation evidence

The coordinator replaced the full-build inventory requirement with a source-level
inventory because the base lib-test exceeds this container's 16 GiB hard limit.
The single baseline attempt was stopped without retrying (compiler observed at
14,739,128 KiB RSS (about 14.1 GiB)). No baseline test count is inferred from that incomplete build.

Walk every module mounted under `postgres_tests`, emitting fully qualified test
function paths for native and macro test attributes. Compare the base revision
against the union of the four owning feature groups and require exactly one
owner for every case. Source inventory passed: 2,279 tests before and after; g1 636, g2 504, g3 557, g4 582; no-feature PG count 0. Every test has exactly one group. `diff /tmp/before.txt /tmp/after.txt` is empty. Both lists have SHA-256 `8c20341098ce01afd76d9d090772a65f4f1f4c49ee7194fc193342a14369a6ee`.

The parser at `/tmp/pg-inventory.py` walked 1,081 files from `git show e2152d05:<path>` and the changed tree, following explicit paths, ordinary/inline modules and generated mounts. It expanded `damage_cases!`, `quest_case!`, and `native_case!`: 2,209 direct tests plus 70 generated tests. It compared actual generated module cfgs and test cfgs, including dispatch’s generated copy of the shared fixture. Test body token hashes, excluding cfg attributes, show zero changes.

Required group builds run sequentially with:

```sh
/usr/bin/time -v cargo test -p oteryn-gameplay-dispatch --lib --features pg-tests-gN --no-run -j1
```

Repository policy validation passed. Governance validation and 135 governance
unit tests passed. Full merge-group mutation simulation passed all 61 mutations, four native
PowerShell failure positions, and the full fan-in cases. The official portable
PowerShell 7.4.6 runtime and GNU time 1.9-0.2 profiler were extracted under
`/tmp` for verification; project dependencies are unchanged. Repository policy also passed on the final CI tree.

All four first full code-generation builds passed sequentially at `-j1`:

| Group | Peak RSS (KiB) | Peak RSS (GiB) | Elapsed |
|---|---:|---:|---:|
| g1 | 13,279,684 | 12.66 | 18m40s |
| g2 | 13,222,208 | 12.61 | 16m25s |
| g3 | 12,819,008 | 12.23 | 16m52s |
| g4 | 12,486,916 | 11.91 | 18m19s |

These are the first full code-generation measurements, before the final unused-import
cfg cleanup. Final warning-free rechecks passed for g1 (3m51s), g2 (4m34s),
g3 (3m50s), g4 (3m46s), and no-feature (1m14s), with zero dispatch warnings.
All-targets/all-features Clippy with `-D warnings` passed before the final
import-only cfg cleanup; its final repeat is running. Existing vendored Tokio/SQLx
dependency warnings are outside this change. Each final compiled binary’s
`--list` PG names also matched its source inventory exactly (636/504/557/582).

The build script emits cfgs for auto-mounted declarations and for dispatch’s
mount of the shared revision fixture. The fixture’s six tests belong to g1.
W22’s shared table hook keeps its original `None` result when its cases are off:
cfg attributes leave an empty case enum and an empty match, without adding a
fallback expression. Existing test and helper bodies match the baseline token stream
when all PG groups are enabled (conditional import variants select the original
imports), with the catalogue loop’s mechanical `index`/`_index` alpha-rename normalized. The retained source guard uses `_evidence` in exact single-group g4 and its original name elsewhere; its type, value, lifetime and drop behavior are preserved. These two private helper naming adaptations avoid unused warnings while preserving behavior. The additional audit is `/tmp/pg-allfeatures-audit.py`.

## CI coverage

`pr-postgres-harness.yml` and `merge-group-gate.yml` compile dispatch in four
separate matrix jobs into `pg-lib-gN` archives with `--build-jobs 1`, then plan/run every archive.
Each producer has its own artifact and candidate-bound digest; consumers verify
all digests and heads before combining archives. Existing timeouts are unchanged. Other libraries remain
in `pg-lib`. Checksums and archive consumers include all four group archives.
`integration-regressions.yml` enables g2 for its retained `boss_activity` case.
`merge-authority-audit.yml` and the repository policy/simulation command guards
track the new commands; blob pins were regenerated with the repository's
`git_blob_sha` tooling. `merge-gate.yml` fast shards enable no PG feature and
therefore need no PG split.

Both archive planners passed synthetic coverage checks with duplicate non-PG
test IDs. Real nextest accepted the duplicate priority filters. No PG test ignore,
timeout or assertion was changed.

## Module assignment

Baseline line counts below are actual `wc -l` counts over each complete mounted subtree, including nonconventional and external mounts; nested modules inherit their owner’s group. Evidence: `/tmp/PGGROUPS-1-wc.txt` and `/tmp/pg-inventory-module-wc.json`. Shared helpers retain their consumer groups; tests have exactly one owner. The parent file adds 1,343 lines of shared helpers and inline cases assigned to g1. Closely coupled enum-case consumers stay together to preserve helper bodies.

| Group | Source lines |
|---|---:|
| pg-tests-g1 | 90,314 |
| pg-tests-g2 | 90,321 |
| pg-tests-g3 | 90,316 |
| pg-tests-g4 | 90,334 |

| Module | Group | Lines including descendants |
|---|---:|---:|
| `a4_invisible` | 3 | 410 |
| `a4b_utility_runes` | 2 | 332 |
| `account_deletion` | 2 | 640 |
| `account_friends` | 3 | 280 |
| `achievement_granters` | 4 | 76 |
| `achievement_notices` | 1 | 201 |
| `activity_deadline_paging` | 2 | 498 |
| `activity_departed` | 2 | 842 |
| `activity_ground_loop` | 1 | 461 |
| `activity_idle` | 2 | 147 |
| `activity_logout` | 4 | 147 |
| `activity_loss` | 4 | 347 |
| `activity_player_death` | 4 | 1622 |
| `activity_runtime` | 2 | 527 |
| `activity_transfer_recovery` | 2 | 275 |
| `admit_cap_2` | 3 | 430 |
| `adventurer_blessing` | 4 | 318 |
| `all_charms_contracts` | 4 | 1056 |
| `all_charms_delayed` | 4 | 1199 |
| `all_charms_incoming` | 2 | 677 |
| `all_charms_incoming_order` | 3 | 876 |
| `all_charms_outgoing` | 3 | 1328 |
| `all_charms_secondary` | 1 | 376 |
| `ambient_b24` | 2 | 181 |
| `analyser` | 3 | 5 |
| `animus_mastery` | 3 | 414 |
| `annihilator_combat` | 1 | 328 |
| `annihilator_door` | 3 | 289 |
| `annihilator_finish` | 1 | 178 |
| `annihilator_route` | 1 | 783 |
| `appearance_census_departure` | 2 | 298 |
| `appearance_change_postgres` | 4 | 780 |
| `appearance_holder_retry` | 2 | 176 |
| `appearance_observation_busy` | 3 | 201 |
| `appearance_periodic_busy` | 1 | 559 |
| `appearance_respawn_pending` | 2 | 343 |
| `arena_match` | 4 | 1331 |
| `atelier` | 4 | 4857 |
| `authored_kill_bindings` | 2 | 76 |
| `authored_quest_bindings` | 1 | 174 |
| `authored_world_rewards` | 3 | 298 |
| `automatic_spell` | 3 | 89 |
| `autowalk` | 2 | 1187 |
| `background_operation` | 4 | 307 |
| `battle_instinct_status` | 2 | 107 |
| `bed_regen` | 3 | 361 |
| `bed_regen_sleep` | 3 | 80 |
| `bed_sleep_dispatch` | 4 | 223 |
| `bed_sleep_fixture` | 4 | 300 |
| `boss_activity` | 2 | 6149 |
| `boss_difficulty` | 3 | 407 |
| `boss_encounter` | 3 | 2139 |
| `boss_pity` | 2 | 209 |
| `boss_rewards` | 2 | 2055 |
| `bosstiary` | 2 | 198 |
| `bosstiary_slot_loot_kill` | 4 | 351 |
| `bosstiary_slots` | 4 | 325 |
| `browse_field` | 1 | 586 |
| `bundle_vertical_move` | 2 | 28 |
| `busy_holder` | 3 | 265 |
| `cake_contributions` | 2 | 340 |
| `cake_phases` | 4 | 306 |
| `cast_observer_elapsed_admission` | 1 | 130 |
| `challenge_retarget` | 2 | 232 |
| `character_achievements` | 2 | 229 |
| `character_home_town` | 2 | 739 |
| `character_info` | 4 | 446 |
| `character_profile` | 1 | 141 |
| `character_services_v2` | 4 | 520 |
| `character_title_completion` | 1 | 375 |
| `character_title_cosmetics` | 3 | 379 |
| `character_title_sources` | 4 | 370 |
| `charm_progression` | 3 | 839 |
| `chat_world_fixture` | 3 | 202 |
| `citizen_qualification` | 3 | 347 |
| `cl6_status` | 4 | 320 |
| `classic_quests` | 1 | 657 |
| `cleanup_saturation` | 2 | 112 |
| `client_e2e` | 2 | 911 |
| `clxa_hud` | 2 | 68 |
| `coin_exchange` | 1 | 1020 |
| `coin_market` | 4 | 630 |
| `condition_potions` | 4 | 529 |
| `condition_status` | 2 | 146 |
| `conditional_callback_failure` | 2 | 103 |
| `conditional_quest_greetings` | 1 | 180 |
| `container_sort` | 1 | 447 |
| `corpse_decay` | 3 | 223 |
| `cosmetic_tokens` | 1 | 368 |
| `creature_field_bundle` | 3 | 497 |
| `creature_illusion` | 4 | 121 |
| `creature_move` | 1 | 230 |
| `creature_push` | 2 | 692 |
| `crosshair_spells` | 4 | 428 |
| `crystal_consumables` | 3 | 6 |
| `cyclopedia` | 2 | 1795 |
| `cyclopedia_character` | 2 | 397 |
| `cyclopedia_combat` | 1 | 469 |
| `cyclopedia_items` | 4 | 100 |
| `cyclopedia_offence` | 1 | 410 |
| `cyclopedia_protection` | 4 | 115 |
| `cyclopedia_tracker` | 3 | 687 |
| `daily_boost` | 1 | 357 |
| `daily_resting` | 4 | 166 |
| `daily_reward` | 1 | 1146 |
| `daily_world_gates` | 4 | 577 |
| `dark_trails` | 1 | 319 |
| `dawnport_trial` | 1 | 329 |
| `death_protection` | 2 | 147 |
| `definition_ready_claims` | 4 | 173 |
| `delayed_strikes` | 3 | 965 |
| `depot_dispatch` | 3 | 4456 |
| `desert_earned_route` | 2 | 482 |
| `desert_party_rite` | 2 | 827 |
| `direct_serve_readiness` | 3 | 31 |
| `draconia_escape` | 4 | 238 |
| `drome_cube` | 3 | 1361 |
| `drome_cube_profiles` | 3 | 66 |
| `dwarven_ring` | 1 | 411 |
| `echo_warden` | 3 | 1245 |
| `elvenbane_combat` | 1 | 342 |
| `elvenbane_native_goals` | 3 | 446 |
| `elvenbane_route` | 1 | 217 |
| `elvenbane_treasure` | 1 | 393 |
| `endgame_entry` | 3 | 412 |
| `energy_ring` | 1 | 401 |
| `engclaim_m34` | 4 | 549 |
| `engclaim_m35` | 1 | 302 |
| `engclaim_native` | 3 | 335 |
| `engenc2_cults` | 3 | 580 |
| `engenc2_fugue` | 2 | 429 |
| `engenc2_ground_busy` | 3 | 140 |
| `engenc2_ground_read` | 1 | 140 |
| `engenc2_ground_support` | 4 | 103 |
| `engenc2_nomads_use` | 1 | 1026 |
| `engenc2_nomads_window` | 4 | 135 |
| `engenc_asura_dispatch` | 1 | 598 |
| `engenc_ordinary_hits` | 1 | 193 |
| `engenc_private_loot` | 2 | 12 |
| `engitem_seal_paint` | 2 | 630 |
| `engitem_seal_paint_lost_ack` | 3 | 160 |
| `engnpc_m04_completion` | 3 | 514 |
| `engnpc_paid_confirmation` | 3 | 96 |
| `engoutfit` | 4 | 526 |
| `engworldchg` | 3 | 164 |
| `engworldchg_fire` | 2 | 171 |
| `engworldchg_fury` | 3 | 282 |
| `engworldchg_guarded_sale` | 1 | 475 |
| `engworldchg_journals` | 3 | 975 |
| `engworldchg_kingsday` | 3 | 209 |
| `engworldchg_queued_deaths` | 2 | 12 |
| `entity_names` | 4 | 113 |
| `entity_names_unbound_creature` | 1 | 182 |
| `equipment_mods` | 1 | 677 |
| `equipment_scenarios` | 2 | 58 |
| `equipped_offence` | 1 | 384 |
| `event_containers` | 1 | 1178 |
| `event_object_uncertainty` | 2 | 157 |
| `exaltation` | 2 | 55 |
| `exercise_dummy_reservation` | 3 | 151 |
| `exiva_world_types` | 1 | 28 |
| `falcon_chests` | 1 | 448 |
| `familiar_logout_reconciliation` | 2 | 468 |
| `fathers_burden` | 4 | 146 |
| `fenced_home_town` | 3 | 118 |
| `field_runes` | 3 | 3378 |
| `first_entry_training_content` | 4 | 77 |
| `first_vocation_vitals` | 4 | 516 |
| `fixture` | 1 | 1768 |
| `floor_exhaust` | 2 | 174 |
| `forge_creature` | 1 | 195 |
| `forge_dispatch` | 2 | 267 |
| `forge_live` | 4 | 121 |
| `forge_preview` | 2 | 286 |
| `forge_resource_confirmation` | 2 | 104 |
| `fresh_admission_contended_holder` | 4 | 234 |
| `fresh_admission_holder_preference` | 3 | 90 |
| `fresh_admission_holder_priority` | 3 | 142 |
| `fresh_session` | 1 | 117 |
| `full_world_e2e` | 1 | 518 |
| `full_world_support` | 3 | 629 |
| `game_journal_outbox` | 1 | 133 |
| `gem_enchanting` | 1 | 593 |
| `gem_vessel_migration` | 4 | 110 |
| `grizzly_boss` | 3 | 1606 |
| `grizzly_durability` | 2 | 2360 |
| `grizzly_npc` | 3 | 2474 |
| `guild_bank` | 1 | 920 |
| `guild_chat` | 3 | 770 |
| `guild_war` | 1 | 362 |
| `guild_wire` | 3 | 1205 |
| `guildhall` | 2 | 793 |
| `hazard` | 2 | 466 |
| `henricus` | 1 | 421 |
| `highscores_world` | 1 | 162 |
| `hireling_lifecycle` | 4 | 4682 |
| `hireling_store` | 1 | 723 |
| `hotkey_equip` | 3 | 1013 |
| `hotkey_equip_coalescing` | 3 | 9 |
| `hotkey_swap` | 3 | 71 |
| `house_balloon` | 1 | 2675 |
| `house_base_decorations` | 2 | 356 |
| `house_move_out` | 2 | 1359 |
| `house_podium_dispatch` | 3 | 390 |
| `house_transfer` | 3 | 1906 |
| `house_transfer_guard` | 3 | 185 |
| `house_wire` | 4 | 881 |
| `house_wire_bundle` | 4 | 45 |
| `house_wrap` | 1 | 831 |
| `hunting_stamina` | 1 | 622 |
| `idle_kick` | 1 | 418 |
| `imbuement` | 3 | 1434 |
| `imbuement_details` | 2 | 121 |
| `imbuement_fees_migration` | 4 | 90 |
| `imbuement_inscription_gate` | 1 | 43 |
| `inspection` | 2 | 166 |
| `instance_equipment_content` | 2 | 153 |
| `item_cosmetics` | 2 | 1870 |
| `item_craft` | 3 | 367 |
| `item_craft_carried_fluid` | 1 | 758 |
| `item_forge` | 2 | 2172 |
| `item_forge_fusion` | 2 | 711 |
| `item_forge_integrity` | 2 | 502 |
| `item_look` | 2 | 588 |
| `item_seed_planting` | 1 | 487 |
| `item_spell_augments` | 1 | 108 |
| `journal_authority` | 2 | 75 |
| `journal_inventory` | 1 | 182 |
| `journal_logout` | 3 | 476 |
| `journal_queue` | 4 | 700 |
| `journal_watchdog` | 2 | 35 |
| `kill_reward_e2e` | 4 | 2449 |
| `l6_ancient_tombs` | 4 | 289 |
| `l6_carried_step` | 4 | 202 |
| `l6_diprath_room` | 1 | 328 |
| `l6_floor_assembly` | 4 | 976 |
| `l6_floor_native` | 1 | 124 |
| `l6_floor_profiles` | 3 | 130 |
| `l6_mead` | 4 | 669 |
| `l6_native_flames` | 1 | 307 |
| `l6_native_rooms` | 2 | 526 |
| `l6_native_signs` | 3 | 394 |
| `l6_paradox` | 2 | 251 |
| `l6_paradox_native` | 2 | 910 |
| `l6_paradox_story` | 2 | 543 |
| `l6_paradox_upper` | 4 | 267 |
| `l6_reusable_tools` | 4 | 170 |
| `l6_reward_fluids` | 2 | 250 |
| `l6_room_raise` | 4 | 231 |
| `l6_ruby_profile` | 4 | 73 |
| `l6_shared_gate_native` | 3 | 535 |
| `l6_shared_gates` | 3 | 289 |
| `l6_shared_switch_native` | 3 | 352 |
| `l6_shared_switch_timers` | 4 | 173 |
| `l6_shared_switch_treasure` | 4 | 162 |
| `l6_shared_switches` | 2 | 172 |
| `l6_spirit` | 4 | 502 |
| `l6_stag_item` | 1 | 60 |
| `l6_tomb_native` | 1 | 318 |
| `l6_tomb_other_native` | 3 | 131 |
| `l6_treasure_rewards` | 2 | 216 |
| `l6_world_admission` | 2 | 342 |
| `l6_world_pickups` | 1 | 773 |
| `l6_world_puzzles` | 3 | 437 |
| `logout_e2e` | 1 | 1102 |
| `loop_monster` | 1 | 1946 |
| `loop_training` | 3 | 1715 |
| `low_level_declaration` | 2 | 256 |
| `loyalty` | 2 | 382 |
| `loyalty_premium_days` | 4 | 136 |
| `m01b_explore` | 3 | 4885 |
| `m01c_collection` | 2 | 202 |
| `m01c_stampede_entry` | 3 | 203 |
| `m05_gnomus` | 3 | 189 |
| `m06_keys` | 2 | 433 |
| `m06q_quest_doors` | 2 | 1251 |
| `m07_dreamer_carrot` | 2 | 214 |
| `m07_dreamer_fire_bug` | 2 | 391 |
| `m07_dreamer_levers` | 3 | 103 |
| `m07_dreamer_ring_lost_ack` | 1 | 169 |
| `m07_dreamer_ring_offering` | 4 | 191 |
| `m07_dreamer_ring_reset` | 1 | 144 |
| `m07_lion_offerings` | 3 | 269 |
| `m07_machinery_burn` | 3 | 151 |
| `m07_machinery_catapults` | 4 | 31 |
| `m07_machinery_repairs` | 2 | 140 |
| `m07_machinery_wheels` | 4 | 29 |
| `m07_realm_trials` | 3 | 119 |
| `m07_too_hot_columns` | 2 | 84 |
| `m07_trials` | 1 | 113 |
| `m09_multi_rewards` | 3 | 182 |
| `m09_rookie_reward` | 4 | 140 |
| `m09_treasure_rewards` | 1 | 138 |
| `m16_personal_map_marker` | 2 | 293 |
| `m23_last_creep` | 2 | 895 |
| `m25_ahau_exit` | 4 | 530 |
| `m25_ahau_push` | 4 | 418 |
| `m25_corpse_chain` | 1 | 107 |
| `m25_forced_equipment_read` | 4 | 94 |
| `m25_galthen_native_death` | 4 | 206 |
| `m25_native_step_sql_contract` | 2 | 104 |
| `m25_step_exit_matrix` | 1 | 137 |
| `m26_bakragore_policy` | 4 | 128 |
| `m26_catalogue_aliases` | 1 | 96 |
| `m26_rotten_bakragore_population` | 4 | 189 |
| `m26_rotten_family_production` | 2 | 224 |
| `m26_rotten_pillar_production` | 3 | 121 |
| `m26_rotten_room_taints` | 3 | 52 |
| `m26_rotten_vemiath_production` | 4 | 182 |
| `m26_rotten_wave_production` | 4 | 548 |
| `m26_spite_chain` | 4 | 748 |
| `m26_spite_qualification` | 1 | 116 |
| `m26_static_instance` | 1 | 674 |
| `m35_claim_sets` | 1 | 1340 |
| `m38d_anna_targuna` | 3 | 112 |
| `m39_bone_journey` | 4 | 457 |
| `m39_tomes_journey` | 2 | 865 |
| `m40_native_dance` | 2 | 277 |
| `m42_hot_cuisine` | 3 | 526 |
| `m44_amphora_lifecycle` | 1 | 162 |
| `m44_children_lifecycle` | 3 | 173 |
| `m44_flower_lifecycle` | 4 | 255 |
| `m44_lions_lifecycle` | 1 | 341 |
| `m44_source_lifecycle` | 1 | 278 |
| `m49_elemental_arrival` | 3 | 1182 |
| `m56_cursed_crystal_wax` | 2 | 132 |
| `magic2_conjure` | 4 | 299 |
| `mainred_readiness` | 4 | 233 |
| `map_bundle_domain` | 3 | 140 |
| `map_harvest` | 1 | 904 |
| `map_pickup` | 1 | 640 |
| `map_seed_planting` | 4 | 904 |
| `market` | 3 | 1475 |
| `market_details` | 4 | 27 |
| `mining` | 2 | 740 |
| `monk_shrines` | 4 | 1017 |
| `monk_virtues` | 4 | 869 |
| `monster_attacks` | 4 | 1106 |
| `monster_reference_level` | 4 | 75 |
| `mount_taming` | 2 | 1270 |
| `native_objective_atomic` | 1 | 341 |
| `newhaven_corruptor` | 2 | 997 |
| `newhaven_starter` | 4 | 434 |
| `nologout` | 4 | 280 |
| `novelty_items` | 2 | 1271 |
| `npc_item_exchanges` | 4 | 585 |
| `npc_paid_travel` | 3 | 1241 |
| `npc_program` | 1 | 1247 |
| `npc_program_inventory` | 2 | 505 |
| `npc_shop` | 1 | 460 |
| `npcfix_m38_minzy` | 2 | 131 |
| `npcfix_m38_mistrock` | 1 | 193 |
| `npcfix_m38_rascalio` | 1 | 121 |
| `npcfix_m38_return_gate` | 1 | 125 |
| `npcfix_m38_travel_guards` | 1 | 103 |
| `npcfix_m38_travel_return` | 2 | 265 |
| `npcfix_m38_travel_ticket` | 2 | 103 |
| `nx2_arena_native` | 4 | 92 |
| `nx4_npc_presence` | 3 | 83 |
| `nx5_cosmetics` | 2 | 132 |
| `nx6_wayfarer` | 3 | 313 |
| `nx8_forge_projection` | 2 | 93 |
| `nx8_potion_durations` | 4 | 157 |
| `nx8_proficiency_catalogue` | 2 | 205 |
| `one_character` | 1 | 330 |
| `oriental_trader` | 4 | 361 |
| `paralyze_rune` | 1 | 225 |
| `parameter_spell` | 3 | 1087 |
| `party_buffs` | 4 | 691 |
| `party_chat` | 4 | 223 |
| `party_effects` | 4 | 253 |
| `party_list` | 4 | 218 |
| `party_member_effects` | 4 | 126 |
| `party_membership` | 4 | 528 |
| `party_skull` | 4 | 301 |
| `party_skull_observation` | 4 | 289 |
| `party_xp` | 1 | 3517 |
| `pick_exchange` | 3 | 455 |
| `planned_world_reset` | 1 | 340 |
| `planned_world_reset_retired_holder` | 2 | 193 |
| `planned_world_reset_source_map` | 2 | 156 |
| `platform_enforcement` | 2 | 1047 |
| `platform_streams` | 3 | 104 |
| `player_death_items` | 4 | 369 |
| `player_report` | 3 | 380 |
| `player_trade` | 1 | 1275 |
| `player_trade_holder` | 1 | 48 |
| `preadmission` | 1 | 218 |
| `premchan` | 1 | 262 |
| `premium_overview` | 2 | 197 |
| `pressure_plate_sewer` | 4 | 386 |
| `prey` | 2 | 632 |
| `prey_difficulty` | 1 | 511 |
| `prey_expansion` | 4 | 493 |
| `prey_loot_measurement` | 3 | 583 |
| `prey_store_retry` | 4 | 282 |
| `primal_menace_native` | 1 | 191 |
| `private_channel_admission_retry` | 1 | 119 |
| `private_channel_participants` | 1 | 243 |
| `private_channel_peers` | 2 | 69 |
| `private_channel_world` | 3 | 286 |
| `private_messages` | 3 | 190 |
| `proficiency_browse` | 2 | 218 |
| `proficiency_catalyst` | 2 | 297 |
| `proficiency_kill_credit` | 3 | 766 |
| `proficiency_modification` | 4 | 537 |
| `proficiency_select_perk` | 4 | 408 |
| `profile_b` | 4 | 382 |
| `promotion_npc` | 3 | 568 |
| `public_channels` | 2 | 411 |
| `pvp_death` | 1 | 128 |
| `pvp_death_most_damage` | 1 | 90 |
| `pvp_expiry` | 2 | 55 |
| `pvp_live` | 2 | 1378 |
| `qengine_access` | 2 | 1101 |
| `qengine_document_stories` | 2 | 408 |
| `qengine_endgame_stories` | 2 | 451 |
| `qengine_kill_stories` | 4 | 486 |
| `qengine_level_stories` | 2 | 448 |
| `qengine_m01a_imported_talk` | 1 | 222 |
| `qengine_m01a_production_talk` | 1 | 108 |
| `qengine_m01a_starter_routes` | 3 | 192 |
| `qengine_m01a_sven` | 4 | 147 |
| `qengine_m02_use` | 1 | 388 |
| `qengine_m03_pits` | 2 | 187 |
| `qengine_m03_use` | 1 | 398 |
| `qengine_outfit_stories` | 1 | 2343 |
| `qengine_rewards` | 4 | 2479 |
| `qengine_s06_construction` | 2 | 27 |
| `qengine_s06_items` | 2 | 46 |
| `qengine_voucher_stories` | 2 | 572 |
| `qengine_w15b_multi` | 3 | 134 |
| `qm03_world_admission` | 3 | 333 |
| `qm47_mead` | 1 | 535 |
| `qm47_mead_dispatch` | 1 | 438 |
| `qm48_mammoth` | 1 | 244 |
| `qm50_elevator` | 4 | 300 |
| `qm55_log_refresh` | 1 | 203 |
| `qmech_m21_nested_map` | 2 | 512 |
| `qn1_elemental_consume` | 2 | 1559 |
| `qn1_elemental_readset` | 3 | 36 |
| `qn1_elemental_world_pin` | 1 | 1372 |
| `qn1_environment_field` | 4 | 160 |
| `qn4_quest_crafting` | 2 | 294 |
| `qnpcfx_koshei` | 3 | 327 |
| `qnpcfx_m10` | 4 | 413 |
| `qready1_carried_recipes` | 4 | 163 |
| `qready1_crafting` | 4 | 173 |
| `qready1_reward_claims` | 3 | 898 |
| `qready1_workbenches` | 4 | 404 |
| `qready1_world_clocks` | 4 | 185 |
| `qready2_claims` | 4 | 570 |
| `qready2_purple_claims` | 1 | 295 |
| `quest_admission` | 3 | 397 |
| `quest_callbacks` | 4 | 945 |
| `quest_catalogue_chests` | 1 | 1390 |
| `quest_chest_postgres_tests` | 3 | 301 |
| `quest_conditional_reset` | 4 | 132 |
| `quest_container_chests` | 1 | 1070 |
| `quest_kill_objectives` | 4 | 104 |
| `quest_l7_asymmetric` | 2 | 116 |
| `quest_log_postgres_tests` | 2 | 188 |
| `quest_recovery` | 2 | 31 |
| `quest_reward_claims` | 1 | 1825 |
| `quest_runtime_fixture` | 1 | 34 |
| `quest_world_callbacks` | 4 | 7270 |
| `questl2_djinn` | 3 | 57 |
| `questl2_djinn_world` | 1 | 100 |
| `questl2_fixture` | 3 | 180 |
| `questl2_white_raven_world` | 4 | 80 |
| `questpc2_albinius` | 4 | 108 |
| `questpc2_albinius_portals` | 1 | 233 |
| `questpc2_gerimor` | 1 | 99 |
| `questpc2_gnomus` | 4 | 121 |
| `questpc2_henricus` | 4 | 63 |
| `questpc2_hero_rathleton` | 4 | 96 |
| `questpc2_oldrak` | 3 | 165 |
| `questpc2_rotten_source_items` | 1 | 57 |
| `questpc5_personal_map_marks` | 4 | 216 |
| `questpc6_access` | 1 | 525 |
| `questpc6_fibula_completion` | 3 | 358 |
| `questpc6_fibula_keys` | 4 | 12 |
| `questpc6_fibula_purchase` | 3 | 232 |
| `questpc6_fibula_route` | 2 | 123 |
| `questpc6_native_goals` | 4 | 337 |
| `questpc7_asymmetric_lever` | 4 | 26 |
| `questpc7_eligibility` | 1 | 202 |
| `questpc7_feaster` | 3 | 517 |
| `questpc7_feaster_thaian` | 3 | 41 |
| `questpc7_ferumbras` | 2 | 572 |
| `questpc7_forgotten_story` | 2 | 456 |
| `questpc7_formal` | 1 | 285 |
| `questpc7_friends` | 1 | 295 |
| `questpc7_goblin` | 4 | 454 |
| `questpc7_goblin_slug` | 1 | 37 |
| `questpc7_golden` | 2 | 117 |
| `questpc7_grave` | 3 | 371 |
| `questpc7_grimvale` | 4 | 83 |
| `questpc7_grimvale_story` | 2 | 385 |
| `questpc7_heart_story` | 2 | 378 |
| `questpc7_hero` | 2 | 315 |
| `questpc7_hoist` | 3 | 294 |
| `questpc7_hoist_story` | 3 | 387 |
| `questpc7_hot_corpses` | 3 | 134 |
| `questpc7_hydra` | 3 | 126 |
| `questpc7_native_deaths` | 4 | 253 |
| `questpc7_outfit_items` | 1 | 304 |
| `questpc7_reward_slices` | 1 | 79 |
| `questpc7_smith` | 3 | 171 |
| `questpc7_tokens` | 2 | 77 |
| `questpc7_world_outfit` | 4 | 223 |
| `questpc8_anniversary` | 4 | 412 |
| `questpc8_authored_programmes` | 4 | 295 |
| `questpc8_bridge_routes` | 4 | 428 |
| `questpc8_colours_opening` | 4 | 156 |
| `questpc8_fluid_output_proof` | 3 | 59 |
| `questpc8_kormarak` | 4 | 194 |
| `questpc8_life_ring` | 4 | 333 |
| `questpc8_life_ring_bridge` | 4 | 263 |
| `questpc8_magic_sword` | 2 | 506 |
| `questpc8_mastermind` | 1 | 156 |
| `questpc8_mystery` | 3 | 202 |
| `questpc8_mystery_assembly` | 4 | 286 |
| `questpc8_mystery_geometry` | 4 | 179 |
| `questpc8_native_cache` | 1 | 37 |
| `questpc8_native_scenes` | 4 | 242 |
| `questpc8_native_slice` | 1 | 147 |
| `questpc8_npc_program_regressions` | 4 | 631 |
| `questpc8_npc_teleport_story` | 4 | 118 |
| `questpc8_peer_predicate` | 2 | 239 |
| `questpc8_remaining_routes` | 2 | 28 |
| `questpc8_retrieval_routes` | 4 | 442 |
| `questpc8_reward_claims` | 1 | 69 |
| `questpc8_secret_library` | 4 | 621 |
| `questpc8_smith_cache` | 1 | 254 |
| `questpc8_state_clock` | 3 | 20 |
| `questpc8_static_constructors` | 3 | 93 |
| `questpc8_trader` | 4 | 874 |
| `questpc8_treasure_island` | 1 | 707 |
| `questpc8_venore` | 2 | 692 |
| `questpc8_venore_supplies` | 1 | 88 |
| `questpc8_voodoo` | 3 | 27 |
| `questpc8_winterlight` | 1 | 356 |
| `questpc8_wrath_crystals` | 3 | 311 |
| `questpc8_wrath_dream` | 3 | 101 |
| `questpc8_wrath_hunt` | 3 | 320 |
| `questpc8_wrath_hunt_npc` | 3 | 76 |
| `questpc8_wrath_opening` | 3 | 131 |
| `questpc8_wrath_source_dragon` | 4 | 128 |
| `quests3_primal_step` | 2 | 26 |
| `quick_loot` | 1 | 896 |
| `qworld` | 2 | 568 |
| `qworld_native_deaths` | 1 | 294 |
| `qworld_phase_steps` | 2 | 289 |
| `raid_live` | 3 | 1150 |
| `ranged_attack` | 3 | 717 |
| `ranged_training` | 4 | 468 |
| `reaudit_arena_lifecycle` | 3 | 1694 |
| `reaudit_starting_progression` | 2 | 529 |
| `reaudit_title_predicates` | 3 | 424 |
| `refusal_detail` | 4 | 201 |
| `released_death` | 4 | 378 |
| `rest_status` | 2 | 141 |
| `resume_holder_priority` | 3 | 147 |
| `review5_map_mark_runtime_lock` | 3 | 22 |
| `revocation_lineage` | 1 | 189 |
| `reward_bag` | 4 | 389 |
| `rotten_blood_progression` | 2 | 1145 |
| `rune_use_bridge` | 1 | 1019 |
| `rust_lottery` | 4 | 769 |
| `s10_bank` | 1 | 135 |
| `s10_blessings` | 4 | 296 |
| `s10_healing` | 1 | 234 |
| `s10_promotions` | 3 | 31 |
| `s10_rashid` | 4 | 131 |
| `s10_travel` | 3 | 305 |
| `s10_voices` | 1 | 16 |
| `s10_wandering` | 2 | 303 |
| `s15a_login_protection` | 1 | 79 |
| `s17_audit_busy` | 1 | 224 |
| `s17_client` | 3 | 463 |
| `s17_client_watch` | 4 | 421 |
| `s17_observer_busy` | 1 | 200 |
| `s17_persistent_bans` | 4 | 89 |
| `s17_viewer_projection` | 2 | 138 |
| `sanction_record` | 4 | 138 |
| `scenarios` | 2 | 2965 |
| `scheduled_event_rates` | 4 | 123 |
| `seam_control_loss` | 1 | 70 |
| `seam_control_loss_unknown` | 2 | 283 |
| `seam_fresh_commit_busy` | 3 | 142 |
| `seam_join` | 4 | 244 |
| `seam_resume` | 2 | 319 |
| `seam_resume_busy` | 2 | 112 |
| `seam_resume_clock_step` | 4 | 100 |
| `seam_terminal_retirement` | 3 | 225 |
| `seam_wheel_admission` | 3 | 197 |
| `season_first_dragon` | 2 | 673 |
| `security_revocation` | 1 | 436 |
| `server_modal_bed` | 4 | 329 |
| `server_save_countdown` | 1 | 76 |
| `session_ingress` | 1 | 282 |
| `skill_progress` | 2 | 311 |
| `soul_war` | 1 | 1742 |
| `soul_war_parity` | 4 | 64 |
| `soulpit` | 4 | 792 |
| `soulpit_boot` | 1 | 93 |
| `soulpit_lifecycle` | 1 | 855 |
| `soulpit_shared_entry` | 1 | 1543 |
| `soulseal_fights` | 1 | 367 |
| `source_quest_rewards` | 1 | 598 |
| `source_world_crop` | 1 | 213 |
| `special_potions` | 4 | 588 |
| `spell_aim_mode` | 3 | 437 |
| `spell_catalog` | 1 | 157 |
| `spite_step_instance_activation` | 1 | 364 |
| `srv6_hero_title` | 3 | 108 |
| `srv6_island_title` | 4 | 202 |
| `srv6_planegazer` | 1 | 75 |
| `srv6_title_task_board` | 2 | 202 |
| `srv7_m02_ground_stack` | 2 | 74 |
| `srv7_reliable_ram` | 2 | 353 |
| `srv9b_fluids` | 3 | 470 |
| `srv9b_marriage` | 2 | 621 |
| `srvenc_boss_publication` | 1 | 212 |
| `srvenc_source_boss` | 3 | 657 |
| `srvenc_source_deaths` | 1 | 572 |
| `srvgap2_primal_chain` | 4 | 730 |
| `srvgap2_wizard` | 1 | 5 |
| `srvgap_c4` | 2 | 84 |
| `srvgap_quest_disguise` | 3 | 548 |
| `srvgap_watch` | 1 | 84 |
| `srvwld_dryad_garden` | 2 | 125 |
| `srx1_writing` | 2 | 56 |
| `srx2_charm_consumers` | 1 | 1217 |
| `srx2_scavenge` | 3 | 1821 |
| `srx3_activity_source_fixture` | 4 | 397 |
| `srx3_boss_corpse` | 3 | 587 |
| `srx3_brain_heal` | 3 | 98 |
| `srx3_claustrophobic` | 4 | 214 |
| `srx3_cloak_blood` | 1 | 488 |
| `srx3_cruelty_callbacks` | 1 | 432 |
| `srx3_cruelty_uncredited_corpse` | 4 | 358 |
| `srx3_drome` | 3 | 127 |
| `srx3_drome_hazards` | 3 | 98 |
| `srx3_elder_terminal` | 4 | 76 |
| `srx3_gaz_minion_ground` | 3 | 341 |
| `srx3_greed_callbacks` | 3 | 251 |
| `srx3_greed_retention` | 2 | 594 |
| `srx3_hatred_entry` | 1 | 384 |
| `srx3_hatred_room_reuse` | 4 | 218 |
| `srx3_malice_callbacks` | 1 | 601 |
| `srx3_megalomania_callbacks` | 2 | 282 |
| `srx3_megalomania_step` | 3 | 801 |
| `srx3_native_retained_foam` | 1 | 453 |
| `srx3_native_smelly` | 3 | 175 |
| `srx3_native_source_cast` | 3 | 843 |
| `srx3_native_source_spawn` | 1 | 267 |
| `srx3_native_vortex` | 3 | 136 |
| `srx3_omrafir_beam` | 2 | 259 |
| `srx3_outburst_entry` | 3 | 155 |
| `srx3_outburst_phase` | 1 | 128 |
| `srx3_poor_soul_world_corpse` | 2 | 328 |
| `srx3_primal` | 1 | 691 |
| `srx3_primal_corpse` | 3 | 249 |
| `srx3_primal_death_cutoff` | 4 | 827 |
| `srx3_primal_prewarm` | 3 | 78 |
| `srx3_primal_ranged` | 1 | 217 |
| `srx3_primal_step` | 1 | 131 |
| `srx3_primal_terminal` | 3 | 254 |
| `srx3_rotten_advanced` | 3 | 863 |
| `srx3_rotten_area_think` | 1 | 150 |
| `srx3_rotten_bak_claim` | 1 | 23 |
| `srx3_rotten_callbacks` | 4 | 1661 |
| `srx3_rotten_death` | 3 | 582 |
| `srx3_rotten_due` | 2 | 234 |
| `srx3_rotten_essence` | 2 | 518 |
| `srx3_rotten_footprints` | 3 | 234 |
| `srx3_rotten_namespace` | 1 | 73 |
| `srx3_rotten_portal` | 2 | 120 |
| `srx3_rotten_portal_step` | 3 | 369 |
| `srx3_rotten_preparation` | 2 | 475 |
| `srx3_seven_teleports` | 2 | 137 |
| `srx3_soul_war_attacks` | 1 | 477 |
| `srx3_soul_war_auto` | 4 | 176 |
| `srx3_soul_war_namespace` | 2 | 40 |
| `srx3_soul_war_source_use` | 1 | 548 |
| `srx3_soul_war_world_entrance` | 4 | 126 |
| `srx3_soul_war_world_fixture` | 3 | 237 |
| `srx3_soulpit_entry` | 2 | 361 |
| `srx3_soulpit_native` | 4 | 1623 |
| `srx3_source_death_quest` | 1 | 270 |
| `srx3_source_death_timers` | 3 | 330 |
| `srx3_source_names` | 2 | 389 |
| `srx3_source_names_negative` | 3 | 150 |
| `srx3_spite_global` | 1 | 1661 |
| `srx3_taints` | 2 | 369 |
| `srx3_time_guardian` | 3 | 215 |
| `srx3_tyrn_heal` | 3 | 116 |
| `srx3_vemiath_return` | 1 | 173 |
| `stack_quantity` | 2 | 586 |
| `staff` | 2 | 3156 |
| `staff_release_holder_busy` | 3 | 74 |
| `stale_nested_locator` | 3 | 461 |
| `stance_reconciliation` | 2 | 403 |
| `standard_fishing` | 2 | 860 |
| `stash_browse` | 1 | 168 |
| `store` | 4 | 595 |
| `store_purchase_history` | 1 | 177 |
| `subarea` | 1 | 238 |
| `summer_corpse_capacity` | 4 | 141 |
| `summer_phosphorus` | 4 | 153 |
| `summer_population` | 1 | 505 |
| `support_intake` | 2 | 135 |
| `target_move` | 4 | 390 |
| `task_board` | 3 | 4126 |
| `team_finder_leader` | 2 | 373 |
| `team_finder_members` | 1 | 1745 |
| `temple_scroll` | 1 | 802 |
| `terrain_tool` | 2 | 812 |
| `threatened_dreams` | 3 | 242 |
| `timed_item` | 1 | 1012 |
| `triangle_tower` | 2 | 191 |
| `tutorial_corruptor_rearm` | 1 | 65 |
| `tutorial_vitals_fixture` | 2 | 83 |
| `underwater_drowning` | 2 | 1206 |
| `utility_runes` | 4 | 1193 |
| `vertical_move` | 4 | 464 |
| `viola` | 3 | 619 |
| `viola_book` | 4 | 472 |
| `vip` | 1 | 804 |
| `vip_presence` | 2 | 443 |
| `vocationless_vitals` | 3 | 111 |
| `w22a_loot` | 4 | 646 |
| `waterfall_access` | 2 | 92 |
| `weapon_proficiency_state` | 2 | 163 |
| `weather_wire` | 2 | 182 |
| `wheel_admission_deferred` | 1 | 132 |
| `wheel_allocation` | 4 | 1369 |
| `world_callback_clocks` | 2 | 238 |
| `world_event` | 4 | 1134 |
| `world_reset_activation` | 3 | 265 |
| `worldchg_benevola` | 3 | 459 |
| `worldchg_grimvale` | 2 | 1714 |
| `worldchg_hive_egress` | 3 | 297 |
| `worldchg_trader` | 4 | 368 |
| `worldchg_yeo` | 1 | 298 |
