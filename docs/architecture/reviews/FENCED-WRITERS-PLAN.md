# Fenced runtime writers — #2046, slice 1

Baseline: origin/main `00c010668` before migration 0952. Inventory below is derived
from migration GRANT/REVOKE statements in numeric order, including column grants, role lists and literal FOREACH grant loops.
It deliberately includes every runtime direct-DML table, including supporting,
account, world and outbox aggregates, so Character state is not lost by filtering
only `game_character_*` names. SELECT-only and definer-only tables are excluded.
Grant source names identify the migrations to inspect before each later slice.
INSERT/UPDATE/DELETE here describe grants, not whether immutable triggers permit them.

## Fence and rollout

**S** means resolve the affected Character binding listed in the row, then verify
against `game_durability_reconnect_sessions`: `character_id`, `game_session_id`,
`current_generation` against the supplied connection generation,
`character_lease_generation`, `scope_ownership_generation`, and live
`session_state IN (1,2)`. Lock that session row FOR SHARE through commit to
serialize replacement. Retained receipt columns are expected evidence, never
independent proof of current authority. Existing Rust admission, runtime scope,
node, recovery, interpretation and revision checks remain required.

For a shared/account/world aggregate, resolve all affected Characters through its
owner before choosing a writer; background jobs need their existing scope/job
fence instead of fabricating a player session. The inventory is conservative:
those rows are migration-planning dependencies, not a proposal to force S onto
non-player jobs. Multi-character operations must verify each participant in stable
key order. Receipt/outbox writes must compose with their owning transaction.

Each later slice should select one owning transaction, preserve its revision and
receipt guards, add a bounded definer with a fixed trusted schema search path
(pg_temp last), revoke PUBLIC EXECUTE, grant runtime EXECUTE, revoke direct DML,
and move every production writer together. Verify current/stale generations,
identity mismatch, terminal session, direct DML denial and atomic rollback using
real PostgreSQL. Resolve specialized scope/job fences before migrating shared rows.

The owning citizenship contract is `docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md`
(home town, DUR-02 section 5): initialization and citizenship must prove the
current gameplay fence in the same transaction.

0952 migrates only `game_character_home_towns`, whose single Rust writer is
`DurabilityRoot::character_home_town`. Its function verifies S using the Character,
session, connection, lease and scope generations supplied by that already-fenced
transaction. It preserves existing citizenship receipts and deferred guards.
Runtime loses INSERT/UPDATE/DELETE; SELECT is retained. The prior home-town
`SELECT FOR UPDATE` becomes an ordinary SELECT because the existing gameplay
fence already locks the Character root through commit; PostgreSQL requires an
UPDATE privilege for a locking SELECT too. This slice does not claim
to fence the remaining tables or replace their existing authority checks.

## Direct-DML inventory before 0952

| Table | Runtime DML | Fence columns / binding to verify | Grant migration(s) |
| --- | --- | --- | --- |
| `game_account_achievement_grant_requests` | INSERT | S via `character_id` | `0021_account_achievements.sql` |
| `game_account_achievements` | INSERT | S via resolve Character through owning aggregate/receipt | `0021_account_achievements.sql` |
| `game_account_appearance_unlock_receipts` | INSERT | S via `granting_character_id` | `0093_character_appearance.sql` |
| `game_account_appearance_unlocks` | INSERT | S via resolve Character through owning aggregate/receipt | `0093_character_appearance.sql` |
| `game_account_badges` | INSERT | S via resolve Character through owning aggregate/receipt | `0573_account_badges.sql` |
| `game_account_bank_audit_outbox` | INSERT | S via resolve Character through owning aggregate/receipt | `0071_account_bank.sql` |
| `game_account_bank_balances` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0071_account_bank.sql` |
| `game_account_bank_coin_lines` | INSERT | S via resolve Character through owning aggregate/receipt | `0071_account_bank.sql` |
| `game_account_bank_entries` | INSERT | S via `acting_character_id`, `recipient_character_id` | `0071_account_bank.sql` |
| `game_account_bank_operations` | INSERT | S via `acting_character_id`, `recipient_character_id` | `0071_account_bank.sql` |
| `game_account_daily_reward_joker_entries` | INSERT | S via resolve Character through owning aggregate/receipt | `0159_daily_reward_delivery.sql` |
| `game_account_daily_reward_jokers` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0159_daily_reward_delivery.sql` |
| `game_account_guild_leadership` | INSERT, UPDATE, DELETE | S via `character_id` | `0132_guild_core.sql` |
| `game_account_prey_wildcard_entries` | INSERT | S via resolve Character through owning aggregate/receipt | `0214_daily_reward_effects.sql` |
| `game_account_prey_wildcards` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0214_daily_reward_effects.sql` |
| `game_activity_handoff_groups` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0628_activity_handoff_runtime.sql` |
| `game_activity_handoff_participants` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0628_activity_handoff_runtime.sql` |
| `game_activity_runtime_entries` | INSERT | S via `initiating_character_id`; retained evidence: `game_session_id` | `0628_activity_handoff_runtime.sql` |
| `game_activity_runtime_placements` | INSERT | S via `character_id` | `0628_activity_handoff_runtime.sql` |
| `game_activity_runtime_returns` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0628_activity_handoff_runtime.sql` |
| `game_activity_transfer_recoveries` | INSERT | S via resolve Character through owning aggregate/receipt; retained evidence: `game_session_id`, `connection_generation` | `0628_activity_handoff_runtime.sql` |
| `game_atelier_fee_coin_records` | INSERT | S via `character_id` | `0745_character_atelier_carried_coins.sql` |
| `game_atelier_item_records` | INSERT | S via `character_id` | `0314_character_gem_atelier_paid.sql` |
| `game_boss_difficulty_contexts` | INSERT, UPDATE | S via `character_id`; retained evidence: `game_session_id`, `connection_generation`, `character_lease_generation`, `scope_ownership_generation` | `0631_boss_difficulty_selection.sql` |
| `game_boss_difficulty_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id`, `connection_generation` | `0631_boss_difficulty_selection.sql` |
| `game_boss_reward_collection_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id`, `connection_generation` | `0630_boss_reward_collection.sql` |
| `game_boss_reward_contributors` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0629_boss_reward_settlement.sql` |
| `game_boss_reward_deaths` | INSERT | S via resolve Character through owning aggregate/receipt | `0629_boss_reward_settlement.sql` |
| `game_boss_reward_item_collections` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0630_boss_reward_collection.sql` |
| `game_boss_reward_items` | INSERT, UPDATE | S via `character_id` | `0629_boss_reward_settlement.sql` |
| `game_boss_room_activities` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0628_activity_handoff_runtime.sql` |
| `game_boss_room_participants` | INSERT, UPDATE | S via `character_id` | `0628_activity_handoff_runtime.sql` |
| `game_boss_spawn_clocks` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0246_boss_encounter.sql` |
| `game_character_account_guards` | INSERT, UPDATE | Bootstrap authority: `account_id` and accepted intent; no player session exists yet | `0006_node_boot_control_plane.sql` |
| `game_character_account_projection_outbox` | DELETE | Projection worker: `account_id`, `projection_epoch`, `projection_revision`; no player write authority | `0024_account_characters_projection.sql` |
| `game_character_achievement_progress` | INSERT, UPDATE | S via `character_id` | `0574_character_achievement_progress.sql` |
| `game_character_animus_masteries` | INSERT | S via `character_id` | `0408_character_animus_mastery.sql` |
| `game_character_appearance` | INSERT, UPDATE | S via `character_id` | `0093_character_appearance.sql` |
| `game_character_appearance_receipts` | INSERT | S via `character_id` | `0093_character_appearance.sql` |
| `game_character_atelier_receipts` | INSERT | S via `character_id` | `0313_character_gem_atelier.sql` |
| `game_character_atelier_state` | INSERT, UPDATE | S via `character_id` | `0313_character_gem_atelier.sql` |
| `game_character_audit_outbox` | INSERT | S via `character_id` | `0006_node_boot_control_plane.sql` |
| `game_character_bestiary_kill_receipts` | INSERT | S via `character_id` | `0019_character_bestiary_progress.sql` |
| `game_character_bestiary_progress` | INSERT, UPDATE | S via `character_id` | `0019_character_bestiary_progress.sql` |
| `game_character_blessings` | DELETE, INSERT | S via `character_id` | `0016_character_death_receipts.sql`, `0133_character_promotion_purchases.sql` |
| `game_character_bootstrap_intent_floors` | INSERT, UPDATE | Bootstrap authority: `issuer_scope`, monotonic `source_revision`, `issuer_decision_id`, `intent_binding`; no player session yet | `0006_node_boot_control_plane.sql` |
| `game_character_boss_cooldowns` | INSERT, UPDATE, DELETE | S via `character_id` | `0246_boss_encounter.sql` |
| `game_character_boss_eligibility` | INSERT, UPDATE | S via `character_id` | `0246_boss_encounter.sql` |
| `game_character_boss_open_world_eligibility` | INSERT, UPDATE | S via `character_id` | `0246_boss_encounter.sql` |
| `game_character_boss_reward_chests` | INSERT, UPDATE | S via `character_id` | `0435_boss_reward_chest.sql`, `0630_boss_reward_collection.sql` |
| `game_character_boss_slot_operations` | INSERT | S via `character_id` | `0786_character_boss_slots.sql` |
| `game_character_boss_slots` | INSERT, UPDATE | S via `character_id` | `0786_character_boss_slots.sql` |
| `game_character_bosstiary_kill_receipts` | INSERT | S via `character_id` | `0532_bosstiary_kill_receipts.sql` |
| `game_character_bosstiary_progress` | INSERT, UPDATE | S via `character_id` | `0445_character_bosstiary_progress.sql` |
| `game_character_bounty_reroll_token_entries` | INSERT | S via `character_id` | `0547_daily_reward_v2.sql` |
| `game_character_bounty_reroll_tokens` | INSERT, UPDATE | S via `character_id` | `0547_daily_reward_v2.sql` |
| `game_character_build_receipts` | INSERT | S via `character_id` | `0030_character_build_state.sql` |
| `game_character_build_state` | INSERT, UPDATE | S via `character_id` | `0180_loop_training.sql` |
| `game_character_charm_assignments` | INSERT, DELETE | S via `character_id` | `0020_character_charm_state.sql`, `0195_character_charm_unassign.sql` |
| `game_character_charm_expansions` | INSERT | S via `character_id` | `0568_character_charm_expansion.sql` |
| `game_character_charm_receipts` | INSERT | S via `character_id` | `0020_character_charm_state.sql` |
| `game_character_charm_unlocks` | INSERT, UPDATE, DELETE | S via `character_id` | `0785_character_charm_reset.sql` |
| `game_character_command_receipts` | INSERT | S via `character_id` | `0087_character_commands.sql` |
| `game_character_concoction_effects` | INSERT | S via `character_id` | `0582_wealth_duplex.sql` |
| `game_character_condition_snapshots` | INSERT, UPDATE, DELETE | S via `character_id`; retained evidence: `character_lease_generation` | `0821_character_condition_snapshots.sql` |
| `game_character_cosmetic_token_entries` | INSERT | S via `character_id` | `0547_daily_reward_v2.sql` |
| `game_character_cosmetic_tokens` | INSERT, UPDATE | S via `character_id` | `0547_daily_reward_v2.sql` |
| `game_character_cyclopedia` | INSERT, UPDATE | S via `character_id` | `0176_character_cyclopedia.sql` |
| `game_character_cyclopedia_combat` | INSERT, UPDATE | S via `character_id`; retained evidence: `game_session_id`, `connection_generation`, `character_lease_generation` | `0706_cyclopedia_combat_stats.sql` |
| `game_character_cyclopedia_receipts` | INSERT | S via `character_id` | `0176_character_cyclopedia.sql` |
| `game_character_cyclopedia_tracker` | INSERT, DELETE | S via `character_id` | `0586_cyclopedia_tracker.sql` |
| `game_character_daily_reward_claims` | INSERT | S via `character_id` | `0140_daily_reward.sql` |
| `game_character_daily_reward_lines` | INSERT | S via `claim_id` → `game_character_daily_reward_claims.transaction_id` → `character_id` | `0159_daily_reward_delivery.sql` |
| `game_character_daily_rewards` | INSERT, UPDATE | S via `character_id` | `0140_daily_reward.sql` |
| `game_character_death_receipts` | INSERT | S via `character_id` | `0016_character_death_receipts.sql` |
| `game_character_echo_warden_kill_receipts` | INSERT | S via `character_id` | `0654_echo_warden_charm_rewards.sql` |
| `game_character_echo_warden_rewards` | INSERT | S via `character_id` | `0654_echo_warden_charm_rewards.sql` |
| `game_character_equipment_audit_outbox` | INSERT | S via `event_id` / `transaction_id` → `game_character_equipment_receipts.character_id` | `0035_character_equipment.sql` |
| `game_character_equipment_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id`, `connection_generation` | `0035_character_equipment.sql` |
| `game_character_equipment_slots` | INSERT, DELETE | S via `character_id` | `0035_character_equipment.sql` |
| `game_character_equipment_state` | INSERT, UPDATE | S via `character_id` | `0035_character_equipment.sql` |
| `game_character_exercise_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0180_loop_training.sql` |
| `game_character_familiar_receipts` | INSERT | S via `character_id` | `0033_character_familiar_state.sql` |
| `game_character_familiar_state` | INSERT, UPDATE | S via `character_id` | `0033_character_familiar_state.sql` |
| `game_character_forge_dust` | INSERT, UPDATE | S via `character_id` | `0059_character_forge_dust.sql` |
| `game_character_forge_dust_entries` | INSERT | S via `character_id` | `0059_character_forge_dust.sql` |
| `game_character_grizzly_boss_entries` | INSERT, UPDATE | S via `character_id` | `0761_grizzly_boss_visits.sql` |
| `game_character_grizzly_refusals` | INSERT | S via `character_id` | `0760_grizzly_quest_rewards.sql` |
| `game_character_grizzly_reward_entitlements` | INSERT, UPDATE | S via `character_id` | `0760_grizzly_quest_rewards.sql` |
| `game_character_hazard` | INSERT, UPDATE | S via `character_id` | `0332_character_hazard.sql` |
| `game_character_hazard_receipts` | INSERT | S via `character_id`; retained evidence: `connection_generation` | `0332_character_hazard.sql` |
| `game_character_home_town_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id`, `connection_generation`, `character_lease_generation`, `scope_ownership_generation` | `0534_character_home_town.sql` |
| `game_character_home_towns` | INSERT, UPDATE | S via `character_id` | `0534_character_home_town.sql` |
| `game_character_known_spells` | INSERT | S via `character_id` | `0304_character_known_spells.sql` |
| `game_character_last_positions` | INSERT, UPDATE | S via `character_id`; retained evidence: `character_lease_generation` | `0080_character_last_position.sql` |
| `game_character_monk_reward_entitlements` | INSERT, UPDATE | S via `character_id` | `0548_monk_pilgrimage_rewards.sql` |
| `game_character_monk_state_receipts` | INSERT | S via `character_id` | `0026_character_monk_harmony.sql` |
| `game_character_npc_purchase_receipts` | INSERT | S via `character_id` | `0133_character_promotion_purchases.sql` |
| `game_character_operation_receipts` | INSERT, UPDATE | S via `character_id` | `0006_node_boot_control_plane.sql` |
| `game_character_pending_arrivals` | INSERT, DELETE, UPDATE | S via `character_id` | `0136_character_tutorial.sql`, `0144_pending_arrival_row_lock.sql` |
| `game_character_pending_respawns` | INSERT, DELETE | S via `character_id` | `0016_character_death_receipts.sql` |
| `game_character_prey_expansions` | INSERT | S via `character_id` | `0755_prey_list_expansion.sql` |
| `game_character_prey_history` | INSERT | S via `character_id` | `0754_prey_measurements.sql` |
| `game_character_prey_offered` | INSERT, UPDATE | S via `character_id` | `0752_prey_difficulty.sql` |
| `game_character_prey_receipts` | INSERT | S via `character_id` | `0337_prey_owner.sql` |
| `game_character_prey_slots` | INSERT, UPDATE | S via `character_id` | `0337_prey_owner.sql` |
| `game_character_primal_hazard_receipts` | INSERT | S via `character_id`; retained evidence: `connection_generation`, `character_lease_generation`, `scope_ownership_generation` | `0708_primal_hazard_unlock.sql` |
| `game_character_proficiency` | INSERT, UPDATE | S via `character_id` | `0032_character_proficiency.sql` |
| `game_character_proficiency_modification_lines` | INSERT | S via `character_id` | `0055_character_proficiency_modification.sql` |
| `game_character_proficiency_modification_terminals` | INSERT | S via `character_id` | `0055_character_proficiency_modification.sql` |
| `game_character_proficiency_modifications` | INSERT, UPDATE | S via `character_id` | `0055_character_proficiency_modification.sql` |
| `game_character_proficiency_receipt_lines` | INSERT | S via `character_id` | `0032_character_proficiency.sql` |
| `game_character_proficiency_receipts` | INSERT | S via `character_id` | `0032_character_proficiency.sql` |
| `game_character_progression_state` | INSERT, UPDATE | S via `character_id` | `0009_character_progression.sql` |
| `game_character_promotions` | INSERT | S via `character_id` | `0133_character_promotion_purchases.sql` |
| `game_character_pvp_aggression` | INSERT, UPDATE, DELETE | S for `attacker` and current target authority for `target`; `scope_generation`, `attacker_placement`, `target_placement`; expiry worker needs scope/time fence | `0164_live_pvp_harm.sql` |
| `game_character_pvp_harm` | INSERT | S for `attacker` and current victim authority for `victim`; `game_session_id`, `command_id`, `ordinal`, `binding` | `0164_live_pvp_harm.sql` |
| `game_character_pvp_ledger` | INSERT, UPDATE, DELETE | S via `victim_character_id`, `contributor_character_id` | `0081_pvp_state.sql` |
| `game_character_pvp_state` | INSERT, UPDATE | S via `character_id` | `0081_pvp_state.sql`, `0142_pvp_death_consequences.sql` |
| `game_character_quest_obligations` | INSERT, UPDATE | S via `character_id` | `0056_character_quest_state.sql` |
| `game_character_quest_receipts` | INSERT | S via `character_id` | `0056_character_quest_state.sql` |
| `game_character_quest_states` | INSERT, UPDATE | S via `character_id` | `0056_character_quest_state.sql` |
| `game_character_quest_tracks` | INSERT, UPDATE | S via `character_id` | `0056_character_quest_state.sql` |
| `game_character_quest_xp_obligations` | INSERT, DELETE | S via `character_id` | `0069_character_quest_xp_obligations.sql` |
| `game_character_quick_loot_categories` | INSERT, UPDATE, DELETE | S via `character_id` | `0227_quick_loot_settings.sql` |
| `game_character_quick_loot_filters` | INSERT, DELETE | S via `character_id` | `0227_quick_loot_settings.sql` |
| `game_character_quick_loot_settings` | INSERT, UPDATE, DELETE | S via `character_id` | `0227_quick_loot_settings.sql` |
| `game_character_recovery_admissions` | UPDATE | Recovery owner: `authority_scope_id`, `recovery_generation`, `recovery_event_id`, predecessor digest/generation; no player session required | `0006_node_boot_control_plane.sql` |
| `game_character_revenge_marks` | DELETE, INSERT, UPDATE | S via `victim_character_id`, `killer_character_id` | `0081_pvp_state.sql`, `0142_pvp_death_consequences.sql`, `0838_world_pass_role_privileges.sql` |
| `game_character_roots` | INSERT, UPDATE | Existing Character: S via `character_id` plus expected `character_revision`; bootstrap INSERT: accepted intent/account/recovery authority before a session exists | `0006_node_boot_control_plane.sql` |
| `game_character_rotten_blood` | INSERT, UPDATE | S via `character_id` | `0710_rotten_blood_quest.sql` |
| `game_character_rotten_blood_receipts` | INSERT | S via `character_id` | `0710_rotten_blood_quest.sql` |
| `game_character_sanctions` | INSERT, UPDATE | S via `character_id` | `0170_staff_actions.sql` |
| `game_character_social_blocks` | INSERT, UPDATE, DELETE | S via `character_id`, `blocked_character_id` | `0046_world_parties.sql` |
| `game_character_social_settings` | INSERT, UPDATE | S via `character_id` | `0163_vip.sql` |
| `game_character_soul_war` | INSERT, UPDATE | S via `character_id` | `0709_character_soul_war.sql` |
| `game_character_soul_war_receipts` | INSERT | S via `character_id` | `0709_character_soul_war.sql` |
| `game_character_soul_war_reward_entitlements` | INSERT, UPDATE | S via `character_id`; retained evidence: `game_session_id` | `0709_character_soul_war.sql` |
| `game_character_source_group` | INSERT | S via `character_id` | `0043_spell_familiar_group.sql` |
| `game_character_source_group_receipts` | INSERT | S via `character_id` | `0043_spell_familiar_group.sql` |
| `game_character_spell_privacy` | INSERT, UPDATE | S via `character_id` | `0036_character_spell_privacy.sql` |
| `game_character_spell_privacy_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0036_character_spell_privacy.sql` |
| `game_character_stance` | INSERT, UPDATE | S via `character_id` | `0017_character_stance.sql` |
| `game_character_stance_receipts` | INSERT | S via `character_id` | `0017_character_stance.sql` |
| `game_character_starter_grants` | INSERT | S via `character_id` | `0128_item_loop_move.sql` |
| `game_character_stash_audit_outbox` | INSERT | S via `transaction_id` → `game_character_stash_operations.character_id`; same-transaction `event_id` audit binding | `0189_supply_stash.sql` |
| `game_character_stash_balances` | INSERT, UPDATE | S via `character_id` | `0189_supply_stash.sql` |
| `game_character_stash_entries` | INSERT | S via `character_id`; `transaction_id`, `previous_entry_id`, `quantity_before` balance chain | `0189_supply_stash.sql` |
| `game_character_stash_item_lines` | INSERT | S via `transaction_id` → `game_character_stash_operations.character_id`; `item_instance_id`, `parent_item_instance_id`, `placement_ordinal` custody fence | `0189_supply_stash.sql` |
| `game_character_stash_operations` | INSERT | S via `character_id`; `game_session_id`, `command_id`, `transaction_id` receipt binding | `0189_supply_stash.sql` |
| `game_character_store_xp_boosts` | INSERT | S via `character_id` | `0913_daily_reward_missing_parts.sql` |
| `game_character_task_boards` | INSERT, UPDATE | S via `character_id` | `0620_task_board_weekly.sql` |
| `game_character_title_selection` | INSERT, UPDATE | S via `character_id` | `0572_character_titles_catalogue.sql` |
| `game_character_titles` | INSERT, UPDATE, DELETE | S via `character_id` | `0572_character_titles_catalogue.sql` |
| `game_character_training_markers` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0180_loop_training.sql` |
| `game_character_tutorial` | INSERT, UPDATE | S via `character_id` | `0136_character_tutorial.sql` |
| `game_character_unjustified_points` | DELETE, INSERT, UPDATE | S via `character_id`, `victim_character_id` | `0081_pvp_state.sql`, `0142_pvp_death_consequences.sql`, `0838_world_pass_role_privileges.sql` |
| `game_character_wheel_gift_receipts` | INSERT | S via `character_id` | `0625_character_wheel_gift.sql` |
| `game_character_wheel_gift_state` | INSERT, UPDATE | S via `character_id` | `0625_character_wheel_gift.sql` |
| `game_character_wheel_receipts` | INSERT | S via `character_id` | `0070_character_wheel_allocation.sql` |
| `game_character_wheel_slots` | INSERT, UPDATE, DELETE | S via `character_id` | `0070_character_wheel_allocation.sql` |
| `game_character_wheel_state` | INSERT, UPDATE | S via `character_id` | `0070_character_wheel_allocation.sql` |
| `game_character_wheel_vessels` | INSERT, UPDATE | S via `character_id` | `0314_character_gem_atelier_paid.sql` |
| `game_character_xp_receipts` | INSERT | S via `character_id` | `0009_character_progression.sql` |
| `game_charm_expansion_purchases` | INSERT, DELETE | S via `character_id` | `0912_charm_expansion_store_purchase.sql` |
| `game_chat_help_mutes` | INSERT, UPDATE | S via `character_id`, `moderator_character_id` | `0717_chat_public_roles_help.sql` |
| `game_chat_help_receipts` | INSERT | S via resolve Character through owning aggregate/receipt; retained evidence: `game_session_id`, `connection_generation` | `0717_chat_public_roles_help.sql` |
| `game_chat_relay_leases` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0122_chat_relay_leases.sql` |
| `game_coin_exchange_receipts` | INSERT, UPDATE | S via `character_id`; retained evidence: `game_session_id` | `0411_gold_converters.sql` |
| `game_daily_reward_commands` | INSERT | S via resolve Character through owning aggregate/receipt; retained evidence: `game_session_id` | `0159_daily_reward_delivery.sql` |
| `game_depot_search_retrieve_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0585_depot_search_retrieve.sql` |
| `game_drome_cube_uses` | INSERT | S via resolve Character through owning aggregate/receipt | `0637_drome_cube.sql` |
| `game_durability_admission_account_guards` | INSERT, UPDATE, DELETE | S via `presence_character_id`; retained evidence: `game_session_id` | `0006_node_boot_control_plane.sql` |
| `game_durability_admission_character_guards` | INSERT, UPDATE, DELETE | S via `character_id`; retained evidence: `game_session_id` | `0006_node_boot_control_plane.sql` |
| `game_durability_admission_guard_history` | INSERT, UPDATE, DELETE | S via resolve Character through owning aggregate/receipt | `0006_node_boot_control_plane.sql` |
| `game_durability_admission_lifecycle_receipts` | INSERT, UPDATE, DELETE | S via resolve Character through owning aggregate/receipt | `0006_node_boot_control_plane.sql` |
| `game_durability_admission_runtime_guards` | INSERT, UPDATE, DELETE | S via resolve Character through owning aggregate/receipt | `0006_node_boot_control_plane.sql` |
| `game_durability_admission_signing_trust_guards` | INSERT, UPDATE, DELETE | S via resolve Character through owning aggregate/receipt | `0006_node_boot_control_plane.sql` |
| `game_durability_control_loss_continuity` | INSERT, UPDATE, DELETE | S via `character_id` | `0006_node_boot_control_plane.sql` |
| `game_durability_executor_custody` | INSERT, UPDATE, DELETE | S via resolve Character through owning aggregate/receipt | `0006_node_boot_control_plane.sql` |
| `game_durability_fresh_admission_receipts` | INSERT, UPDATE, DELETE | S via `character_id`; retained evidence: `game_session_id`, `connection_generation`, `character_lease_generation`, `scope_ownership_generation` | `0006_node_boot_control_plane.sql` |
| `game_durability_native_source_descriptor_history` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0006_node_boot_control_plane.sql` |
| `game_durability_native_source_floors` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0006_node_boot_control_plane.sql` |
| `game_durability_native_source_observation_history` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0006_node_boot_control_plane.sql` |
| `game_durability_native_source_publication_slots` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0006_node_boot_control_plane.sql` |
| `game_durability_native_source_registration` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0006_node_boot_control_plane.sql` |
| `game_durability_reconnect_attempts` | INSERT, UPDATE, DELETE | S via `character_id`; retained evidence: `game_session_id` | `0006_node_boot_control_plane.sql` |
| `game_durability_reconnect_pending_commands` | INSERT, UPDATE, DELETE | S via resolve Character through owning aggregate/receipt; retained evidence: `game_session_id` | `0006_node_boot_control_plane.sql` |
| `game_durability_reconnect_sessions` | INSERT, UPDATE, DELETE | S via `character_id`; retained evidence: `game_session_id`, `character_lease_generation`, `scope_ownership_generation` | `0006_node_boot_control_plane.sql` |
| `game_durability_recovery_grant_consumptions` | INSERT, UPDATE, DELETE | S via resolve Character through owning aggregate/receipt; retained evidence: `game_session_id` | `0006_node_boot_control_plane.sql` |
| `game_durability_session_replacements` | INSERT, UPDATE, DELETE | S via `character_id` | `0006_node_boot_control_plane.sql` |
| `game_durability_session_use_ledgers` | INSERT, UPDATE, DELETE | S via `character_id` | `0006_node_boot_control_plane.sql` |
| `game_durability_session_use_memberships` | INSERT, UPDATE, DELETE | S via `character_id`; retained evidence: `game_session_id` | `0006_node_boot_control_plane.sql` |
| `game_durability_transport_ref_reservations` | INSERT, UPDATE, DELETE | S via resolve Character through owning aggregate/receipt; retained evidence: `game_session_id` | `0006_node_boot_control_plane.sql` |
| `game_endgame_terminal_outcomes` | INSERT | S via resolve Character through owning aggregate/receipt; retained evidence: `scope_ownership_generation` | `0711_endgame_terminal_outcomes.sql` |
| `game_equipment_hand_displacements` | INSERT | S via resolve Character through owning aggregate/receipt | `0662_hotkey_hand_displacement.sql` |
| `game_equipment_nested_removals` | INSERT | S via `character_id` | `0546_hotkey_nested_equipment.sql` |
| `game_exercise_dummy_reservations` | INSERT, UPDATE, DELETE | S via `character_id`; retained evidence: `game_session_id` | `0917_exercise_dummy_reservations.sql` |
| `game_field_removals` | INSERT | S via `character_id`; retained evidence: `game_session_id`, `connection_generation`, `character_lease_generation` | `0220_field_removal.sql` |
| `game_field_restart_retirements` | INSERT | S via resolve Character through owning aggregate/receipt | `0220_field_removal.sql` |
| `game_forge_cooldowns` | INSERT, UPDATE | S via `character_id` | `0723_native_forge_receipts.sql` |
| `game_forge_dust_limit_history` | INSERT, DELETE | S via `character_id` | `0320_item_forge_live.sql` |
| `game_forge_dust_limit_receipts` | INSERT | S via `character_id` | `0320_item_forge_live.sql` |
| `game_forge_dust_limit_reservations` | INSERT, UPDATE | S via `character_id` | `0320_item_forge_live.sql` |
| `game_forge_history` | INSERT, DELETE | S via `character_id` | `0723_native_forge_receipts.sql` |
| `game_forge_operations` | INSERT, UPDATE | S via `character_id`; retained evidence: `game_session_id` | `0723_native_forge_receipts.sql` |
| `game_forge_reservations` | INSERT, UPDATE | S via `character_id` | `0723_native_forge_receipts.sql` |
| `game_guild_bank_balances` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0174_guild_bank.sql` |
| `game_guild_bank_entries` | INSERT | S via resolve Character through owning aggregate/receipt | `0174_guild_bank.sql` |
| `game_guild_bank_event_outbox` | INSERT | S via resolve Character through owning aggregate/receipt | `0174_guild_bank.sql` |
| `game_guild_bank_operations` | INSERT | S via `actor`; retained evidence: `game_session_id` | `0174_guild_bank.sql` |
| `game_guild_disband_claims` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0174_guild_bank.sql` |
| `game_guild_disband_step_events` | INSERT | S via resolve Character through owning aggregate/receipt | `0132_guild_core.sql` |
| `game_guild_disband_steps` | INSERT | S via resolve Character through owning aggregate/receipt | `0132_guild_core.sql` |
| `game_guild_event_outbox` | INSERT | S via resolve Character through owning aggregate/receipt | `0132_guild_core.sql` |
| `game_guild_invitations` | INSERT, UPDATE, DELETE | S via `character_id` | `0132_guild_core.sql` |
| `game_guild_job_checkpoints` | INSERT | S via resolve Character through owning aggregate/receipt | `0132_guild_core.sql` |
| `game_guild_job_event_outbox` | INSERT | S via resolve Character through owning aggregate/receipt | `0132_guild_core.sql` |
| `game_guild_members` | INSERT, UPDATE, DELETE | S via `character_id` | `0132_guild_core.sql` |
| `game_guild_operations` | INSERT | S via `actor`; retained evidence: `game_session_id` | `0132_guild_core.sql` |
| `game_guild_ranks` | INSERT, UPDATE, DELETE | S via resolve Character through owning aggregate/receipt | `0132_guild_core.sql` |
| `game_guild_war_commands` | INSERT | S via resolve Character through owning aggregate/receipt; retained evidence: `game_session_id` | `0232_guild_wars.sql` |
| `game_guild_war_frags` | INSERT | S via `killer_character_id`, `victim_character_id` | `0232_guild_wars.sql` |
| `game_guild_wars` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0232_guild_wars.sql` |
| `game_guilds` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0132_guild_core.sql` |
| `game_hireling_entitlements` | INSERT | S via `character_id` | `0719_hireling_identity.sql` |
| `game_hireling_food_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0721_hireling_services.sql` |
| `game_hireling_gourmet_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0721_hireling_services.sql` |
| `game_hireling_gourmet_state` | INSERT, UPDATE | S via `character_id` | `0721_hireling_services.sql` |
| `game_hireling_inbox_scope_receipts` | INSERT | S via resolve Character through owning aggregate/receipt; retained evidence: `scope_ownership_generation` | `0722_hireling_lamp.sql` |
| `game_hireling_lamp_transitions` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0722_hireling_lamp.sql` |
| `game_hireling_package_lines` | INSERT | S via resolve Character through owning aggregate/receipt | `0721_hireling_services.sql` |
| `game_hireling_package_receipts` | INSERT | S via resolve Character through owning aggregate/receipt | `0721_hireling_services.sql` |
| `game_hireling_store_reservations` | INSERT, UPDATE | S via `character_id` | `0720_store_hireling_delivery.sql` |
| `game_house_acl` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0038_house_spell_acl.sql` |
| `game_house_acl_entries` | INSERT, DELETE | S via `character_id` | `0168_house_management.sql` |
| `game_house_acl_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0038_house_spell_acl.sql` |
| `game_house_audit_outbox` | DELETE, UPDATE | S via resolve Character through owning aggregate/receipt | `0143_house_ownership.sql` |
| `game_house_disposition_audit_outbox` | DELETE, UPDATE | S via resolve Character through owning aggregate/receipt | `0178_house_disposition.sql` |
| `game_house_editors` | INSERT, UPDATE | S via `character_id`; retained evidence: `game_session_id` | `0038_house_spell_acl.sql` |
| `game_house_furniture_kit_origins` | INSERT | S via resolve Character through owning aggregate/receipt | `0721_hireling_services.sql` |
| `game_house_hireling_receipts` | INSERT | S via resolve Character through owning aggregate/receipt; retained evidence: `game_session_id` | `0405_hireling.sql` |
| `game_house_hirelings` | INSERT, UPDATE | S via `owner_character_id` | `0405_hireling.sql` |
| `game_house_management_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0168_house_management.sql` |
| `game_house_move_out_notices` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0168_house_management.sql` |
| `game_house_rent_state` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0914_house_rent.sql` |
| `game_house_scope_handoffs` | INSERT, UPDATE, DELETE | S via `character_id`; retained evidence: `game_session_id` | `0074_house_scope_handoff.sql` |
| `game_house_transfer_receipts` | INSERT, UPDATE | S via `character_id`; retained evidence: `game_session_id` | `0215_house_transfer.sql` |
| `game_item_audit_outbox` | INSERT | S via resolve Character through owning aggregate/receipt | `0010_item_mint_ground.sql` |
| `game_item_bindings` | INSERT | S via `character_id` | `0159_daily_reward_delivery.sql` |
| `game_item_character_depot_locations` | INSERT, DELETE | S via `character_id` | `0157_depot_transfers.sql` |
| `game_item_condition_potion_receipts` | INSERT | S via resolve Character through owning aggregate/receipt | `0707_condition_potion_receipts.sql` |
| `game_item_consume_transform_records` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0589_item_consume_transform.sql` |
| `game_item_container_entries` | INSERT, DELETE, UPDATE | S via `character_id` | `0011_item_transfer_backpack.sql`, `0023_character_gold_fee_burn.sql`, `0299_container_sort.sql` |
| `game_item_container_slots` | INSERT | S via `character_id` | `0011_item_transfer_backpack.sql` |
| `game_item_container_sort_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id`, `connection_generation` | `0299_container_sort.sql` |
| `game_item_corpse_container_entries` | INSERT, DELETE | S via resolve Character through owning aggregate/receipt | `0013_corpse_container_mint.sql`, `0014_corpse_container_transfer.sql` |
| `game_item_craft_records` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0165_item_craft.sql` |
| `game_item_decay_retire_receipts` | INSERT | S via resolve Character through owning aggregate/receipt | `0015_corpse_decay_retire.sql` |
| `game_item_decay_retire_reservations` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0015_corpse_decay_retire.sql` |
| `game_item_fee_burn_lines` | INSERT | S via resolve Character through owning aggregate/receipt | `0023_character_gold_fee_burn.sql` |
| `game_item_fee_burns` | INSERT | S via `character_id` | `0023_character_gold_fee_burn.sql` |
| `game_item_fluid_loot_mints` | INSERT | S via resolve Character through owning aggregate/receipt | `0597_loot_fluid_states.sql` |
| `game_item_fluid_splashes` | INSERT, DELETE | S via resolve Character through owning aggregate/receipt | `0910_item_fluid_splashes.sql` |
| `game_item_fluid_states` | INSERT, DELETE, UPDATE | S via resolve Character through owning aggregate/receipt | `0560_item_fluid_states.sql` |
| `game_item_fluid_transitions` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0560_item_fluid_states.sql` |
| `game_item_ground_locations` | INSERT, DELETE | S via resolve Character through owning aggregate/receipt | `0010_item_mint_ground.sql`, `0011_item_transfer_backpack.sql` |
| `game_item_imbuement_checkpoints` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0250_imbuement_elapsed.sql` |
| `game_item_imbuement_item_records` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0229_imbuement_item_lifecycle.sql` |
| `game_item_imbuement_records` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0196_item_imbuement_apply_clear.sql` |
| `game_item_instances` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0010_item_mint_ground.sql`, `0058_item_timed_expiry.sql` |
| `game_item_keys` | INSERT | S via resolve Character through owning aggregate/receipt | `0086_item_keys.sql` |
| `game_item_loop_drop_channels` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0128_item_loop_move.sql` |
| `game_item_loop_move_lines` | INSERT | S via resolve Character through owning aggregate/receipt | `0128_item_loop_move.sql` |
| `game_item_loop_move_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id`, `connection_generation` | `0157_depot_transfers.sql` |
| `game_item_mint_receipts` | INSERT | S via resolve Character through owning aggregate/receipt | `0010_item_mint_ground.sql` |
| `game_item_mint_reservations` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0010_item_mint_ground.sql` |
| `game_item_nested_entries` | INSERT, DELETE, UPDATE | S via resolve Character through owning aggregate/receipt | `0128_item_loop_move.sql`, `0299_container_sort.sql` |
| `game_item_text_revisions` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0101_mail.sql` |
| `game_item_text_write_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0101_mail.sql` |
| `game_item_texts` | INSERT, UPDATE, DELETE | S via `writer_character_id` | `0101_mail.sql` |
| `game_item_tiers` | INSERT, UPDATE, DELETE | S via resolve Character through owning aggregate/receipt | `0724_native_forge_atomic_proofs.sql` |
| `game_item_timed_commands` | INSERT | S via `character_id` | `0179_item_timed_runtime.sql` |
| `game_item_timed_state_writes` | INSERT | S via `holder_character_id` | `0054_item_timed_states.sql` |
| `game_item_timed_states` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0054_item_timed_states.sql` |
| `game_item_transfer_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0011_item_transfer_backpack.sql` |
| `game_item_transfer_reservations` | INSERT, UPDATE | S via `character_id`; retained evidence: `game_session_id` | `0011_item_transfer_backpack.sql` |
| `game_item_use_records` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0083_item_use.sql` |
| `game_mail_operations` | INSERT | S via `sender_character_id`, `recipient_character_id`; retained evidence: `game_session_id` | `0271_mail_posting.sql` |
| `game_mail_outbox` | INSERT | S via resolve Character through owning aggregate/receipt | `0271_mail_posting.sql` |
| `game_map_item_mint_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0077_map_item_materialization.sql` |
| `game_map_item_mint_reservations` | INSERT, UPDATE | S via `character_id`; retained evidence: `game_session_id` | `0077_map_item_materialization.sql` |
| `game_mount_taming_attempts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0224_mount_taming.sql` |
| `game_native_map_item_audit` | INSERT | S via resolve Character through owning aggregate/receipt | `0049_native_map_item_initialization.sql` |
| `game_native_map_item_receipts` | INSERT | S via resolve Character through owning aggregate/receipt | `0049_native_map_item_initialization.sql` |
| `game_native_map_scope_adoptions` | INSERT | S via resolve Character through owning aggregate/receipt | `0049_native_map_item_initialization.sql` |
| `game_newhaven_travel_receipts` | INSERT, UPDATE | S via `character_id`; retained evidence: `game_session_id` | `0136_character_tutorial.sql` |
| `game_npc_bank_confirmations` | INSERT, UPDATE | S via `character_id`; retained evidence: `game_session_id` | `0114_npc_bank_confirmation.sql` |
| `game_npc_trade_lines` | INSERT | S via resolve Character through owning aggregate/receipt | `0082_npc_trade.sql` |
| `game_npc_trades` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0082_npc_trade.sql` |
| `game_parties` | INSERT, UPDATE, DELETE | S via `leader_character_id` | `0046_world_parties.sql` |
| `game_party_invitations` | INSERT, UPDATE, DELETE | S via resolve Character through owning aggregate/receipt | `0046_world_parties.sql` |
| `game_party_kill_xp_members` | INSERT | S via `character_id` | `0141_party_xp_hunting.sql` |
| `game_party_kill_xp_receipts` | INSERT | S via resolve Character through owning aggregate/receipt | `0141_party_xp_hunting.sql` |
| `game_party_members` | INSERT, UPDATE, DELETE | S via `character_id` | `0046_world_parties.sql` |
| `game_pick_refinements` | INSERT | S via resolve Character through owning aggregate/receipt | `0167_pick_refining.sql` |
| `game_player_death_item_records` | INSERT | S via `character_id` | `0531_player_death_item_primitives.sql` |
| `game_player_field_charge_writes` | INSERT | S via resolve Character through owning aggregate/receipt; retained evidence: `game_session_id` | `0220_field_removal.sql` |
| `game_player_field_cycles` | INSERT | S via resolve Character through owning aggregate/receipt; retained evidence: `game_session_id` | `0220_field_removal.sql` |
| `game_player_trade_participants` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0147_player_trade.sql` |
| `game_player_trade_receipts` | INSERT | S via `source_character_id`, `destination_character_id` | `0147_player_trade.sql` |
| `game_player_trades` | INSERT | S via resolve Character through owning aggregate/receipt | `0147_player_trade.sql` |
| `game_premium_account_conflict` | INSERT | S via resolve Character through owning aggregate/receipt | `0057_premium_semantic_conflict.sql` |
| `game_premium_account_fence` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0029_premium_entitlement_fence.sql` |
| `game_premium_activation_latch` | INSERT | S via resolve Character through owning aggregate/receipt | `0085_premium_activation_latch.sql` |
| `game_premium_entitlement_fence` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0029_premium_entitlement_fence.sql` |
| `game_premium_evidence` | INSERT | S via resolve Character through owning aggregate/receipt | `0029_premium_entitlement_fence.sql` |
| `game_premium_security_audit` | INSERT | S via resolve Character through owning aggregate/receipt | `0057_premium_semantic_conflict.sql` |
| `game_prey_expansion_deliveries` | INSERT | S via `character_id` | `0755_prey_list_expansion.sql` |
| `game_prey_expansion_purchases` | INSERT, DELETE | S via `character_id` | `0755_prey_list_expansion.sql` |
| `game_private_chat_channels` | INSERT, UPDATE, DELETE | S via `owner_session_id`; retained evidence: `game_session_id` | `0718_private_channel_world.sql` |
| `game_private_chat_members` | INSERT, UPDATE, DELETE | S via resolve Character through owning aggregate/receipt; retained evidence: `game_session_id` | `0718_private_channel_world.sql` |
| `game_private_chat_participants` | INSERT, UPDATE, DELETE | S via `character_id`; retained evidence: `game_session_id`, `connection_generation` | `0718_private_channel_world.sql` |
| `game_private_chat_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id`, `connection_generation` | `0718_private_channel_world.sql` |
| `game_proficiency_shaping_revisions` | UPDATE | S via resolve Character through owning aggregate/receipt | `0915_proficiency_shaping_r2.sql` |
| `game_public_character_profile_outbox` | DELETE | S via `character_id` | `0091_public_projections.sql` |
| `game_public_highscore_outbox` | DELETE | S via resolve Character through owning aggregate/receipt | `0091_public_projections.sql` |
| `game_public_name_handoffs` | DELETE | S via `character_id` | `0091_public_projections.sql` |
| `game_public_world_online` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0091_public_projections.sql` |
| `game_quest_callback_consumptions` | INSERT | S via `character_id` | `0218_quest_world_callbacks.sql` |
| `game_quick_loot_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0227_quick_loot_settings.sql` |
| `game_reward_claim_mint_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0012_reward_claim_backpack_mint.sql` |
| `game_reward_claim_mint_reservations` | INSERT, UPDATE | S via `character_id`; retained evidence: `game_session_id` | `0012_reward_claim_backpack_mint.sql` |
| `game_reward_claims` | INSERT | S via `character_id` | `0012_reward_claim_backpack_mint.sql` |
| `game_rotten_blood_access_fees` | INSERT | S via `character_id` | `0710_rotten_blood_quest.sql` |
| `game_runtime_readiness_attestations` | DELETE | S via resolve Character through owning aggregate/receipt | `0006_node_boot_control_plane.sql` |
| `game_runtime_scope_assignment_writer` | UPDATE | S via resolve Character through owning aggregate/receipt | `0006_node_boot_control_plane.sql` |
| `game_runtime_scope_assignments` | UPDATE | S via resolve Character through owning aggregate/receipt | `0006_node_boot_control_plane.sql` |
| `game_soulpit_encounters` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0652_soulpit_encounters.sql` |
| `game_soulpit_events` | INSERT | Instance owner: `handoff_id`, current assignment/scope generation, expected `revision`; S for affected participants through `game_soulpit_participants.character_id` | `0652_soulpit_encounters.sql` |
| `game_soulpit_participants` | INSERT | S via `character_id`; current committed `handoff_id` participant binding | `0652_soulpit_encounters.sql` |
| `game_soulpit_survival_rewards` | INSERT | S via `character_id`; `handoff_id`, `occurrence_id` and exact committed terminal event | `0652_soulpit_encounters.sql` |
| `game_spell_companion_acquisition_receipts` | INSERT | S via `master_character_id` | `0034_spell_item_transactions.sql` |
| `game_spell_direct_companion_acquisition_receipts` | INSERT | S via `actor_placement_identity`, `master_character_id` | `0052_spell_direct_companion_acquisition.sql` |
| `game_spell_equipment_consumptions` | INSERT | S via `character_id` | `0042_spell_inventory_consumption.sql` |
| `game_spell_field_temporal_audit` | INSERT | S via resolve Character through owning aggregate/receipt | `0050_spell_item_temporal_chain.sql` |
| `game_spell_field_temporal_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id`, `connection_generation`, `character_lease_generation` | `0050_spell_item_temporal_chain.sql` |
| `game_spell_field_temporal_schedules` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0050_spell_item_temporal_chain.sql`, `0088_temporal_schedule_row_lock.sql` |
| `game_spell_item_audit_outbox` | INSERT | S via resolve Character through owning aggregate/receipt | `0034_spell_item_transactions.sql` |
| `game_spell_item_expiry_receipts` | INSERT | S via resolve Character through owning aggregate/receipt | `0034_spell_item_transactions.sql` |
| `game_spell_item_lines` | INSERT | S via resolve Character through owning aggregate/receipt | `0034_spell_item_transactions.sql` |
| `game_spell_item_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0034_spell_item_transactions.sql` |
| `game_spell_item_source_descriptions` | INSERT | S via resolve Character through owning aggregate/receipt | `0051_spell_item_source_description.sql` |
| `game_spell_item_temporal_audit` | INSERT | S via resolve Character through owning aggregate/receipt | `0045_spell_item_temporal.sql` |
| `game_spell_item_temporal_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id`, `connection_generation`, `character_lease_generation` | `0045_spell_item_temporal.sql` |
| `game_spell_item_temporal_schedules` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0045_spell_item_temporal.sql`, `0088_temporal_schedule_row_lock.sql` |
| `game_spell_parameter_results` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0044_spell_parameter_results.sql` |
| `game_spell_premium_accounts` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0040_spell_premium_projection.sql` |
| `game_spell_premium_entitlements` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0040_spell_premium_projection.sql` |
| `game_spell_premium_history` | INSERT | S via resolve Character through owning aggregate/receipt | `0040_spell_premium_projection.sql` |
| `game_spell_premium_security_audit` | INSERT | S via resolve Character through owning aggregate/receipt | `0040_spell_premium_projection.sql` |
| `game_staff_action_outbox` | INSERT | Staff control authority through (`environment`, `operation_id`) → `game_staff_operations`; same-transaction receipt binding | `0170_staff_actions.sql` |
| `game_staff_action_receipts` | INSERT | Staff control authority through (`environment`, `operation_id`) → `game_staff_operations`; exact `control_seq` link | `0170_staff_actions.sql` |
| `game_staff_control_heads` | UPDATE | S via resolve Character through owning aggregate/receipt | `0170_staff_actions.sql` |
| `game_staff_control_links` | INSERT | Staff control authority: `environment`, `control_seq`, `previous_link_sha256`, `activation_sha256`, `roster_sha256`, signed `principal`/`key_fingerprint`; no target player session required | `0170_staff_actions.sql` |
| `game_staff_kicks` | INSERT | S via `character_id` | `0295_staff_kicks.sql` |
| `game_staff_operations` | INSERT | Staff control authority: `environment`, `control_seq`, `operation_id`, `target_character_id`, `world_id`, `channel_id`; current signed activation/roster fence | `0170_staff_actions.sql` |
| `game_store_hireling_effects` | INSERT | S via `character_id` | `0720_store_hireling_delivery.sql` |
| `game_store_purchase_history` | INSERT | S via `character_id` | `0927_store_purchase_history.sql` |
| `game_task_board_cosmetic_grants` | INSERT | S via `character_id` | `0621_task_board_custody.sql` |
| `game_task_board_expansion_deliveries` | INSERT | S via `character_id` | `0620_task_board_weekly.sql` |
| `game_task_board_fee_proofs` | INSERT | S via `character_id` | `0621_task_board_custody.sql` |
| `game_task_board_item_grants` | INSERT | S via `character_id`; `transaction_id`, `cause_id`, `cause_ordinal` receipt binding | `0621_task_board_custody.sql` |
| `game_task_board_receipts` | INSERT | S via `character_id` | `0620_task_board_weekly.sql` |
| `game_task_board_stash_audit_proofs` | INSERT | S via `transaction_id` → `game_task_board_stash_sources.character_id`; same-transaction `event_id` audit binding | `0621_task_board_custody.sql` |
| `game_task_board_stash_sources` | INSERT | S via `character_id`; `transaction_id`, `cause_id`, `cause_ordinal` receipt binding | `0621_task_board_custody.sql` |
| `game_task_board_token_sources` | INSERT | S via `character_id` | `0622_task_board_reroll_tokens.sql` |
| `game_team_finder_active_listings` | INSERT, UPDATE, DELETE | S via `leader_character_id`; retained evidence: `game_session_id`, `connection_generation`, `character_lease_generation`, `scope_ownership_generation` | `0644_team_finder_members.sql` |
| `game_team_finder_leader_listings` | INSERT, UPDATE, DELETE | S via `leader_character_id` | `0578_team_finder_leader_listing.sql` |
| `game_team_finder_leader_receipts` | INSERT | S via `leader_character_id`; retained evidence: `game_session_id` | `0578_team_finder_leader_listing.sql` |
| `game_team_finder_member_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0644_team_finder_members.sql` |
| `game_team_finder_members` | INSERT, UPDATE, DELETE | S via `leader_character_id`, `member_character_id` | `0644_team_finder_members.sql` |
| `game_terrain_tool_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0334_terrain_tool_transforms.sql` |
| `game_terrain_tool_transforms` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0334_terrain_tool_transforms.sql` |
| `game_tutorial_corruptor_claims` | INSERT | S via `character_id` | `0136_character_tutorial.sql` |
| `game_world_cyclopedia` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0176_character_cyclopedia.sql` |
| `game_world_cyclopedia_epochs` | INSERT | S via resolve Character through owning aggregate/receipt | `0176_character_cyclopedia.sql` |
| `game_world_daily_boosts` | INSERT | S via resolve Character through owning aggregate/receipt | `0333_daily_boost.sql` |
| `game_world_event_epochs` | INSERT | S via resolve Character through owning aggregate/receipt | `0172_world_event.sql` |
| `game_world_event_query_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id` | `0172_world_event.sql` |
| `game_world_party_audit_outbox` | INSERT | S via resolve Character through owning aggregate/receipt | `0046_world_parties.sql` |
| `game_world_party_receipts` | INSERT | S via `character_id`; retained evidence: `game_session_id`, `connection_generation` | `0046_world_parties.sql` |
| `game_world_raid_activity` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0782_raid_live_effects.sql` |
| `game_world_raid_channel_runs` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0329_raid_runner.sql` |
| `game_world_raid_creature_placements` | INSERT | S via resolve Character through owning aggregate/receipt | `0782_raid_live_effects.sql` |
| `game_world_raid_creatures` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0329_raid_runner.sql` |
| `game_world_raid_effects` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0782_raid_live_effects.sql` |
| `game_world_raid_firings` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0153_raid_schedule.sql` |
| `game_world_raid_lua_states` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0782_raid_live_effects.sql` |
| `game_world_raid_schedule_sources` | INSERT | S via resolve Character through owning aggregate/receipt | `0782_raid_live_effects.sql` |
| `game_world_raid_scheduler_clocks` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0782_raid_live_effects.sql` |
| `game_world_raid_schedules` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0153_raid_schedule.sql` |
| `game_world_raid_step_receipts` | INSERT | S via resolve Character through owning aggregate/receipt | `0329_raid_runner.sql` |
| `game_world_raid_targets` | INSERT | S via resolve Character through owning aggregate/receipt | `0782_raid_live_effects.sql` |
| `game_world_reset_active` | UPDATE | S via resolve Character through owning aggregate/receipt | `0838_world_pass_role_privileges.sql` |
| `store_delivery_receipts` | INSERT | S via `character_id` | `0345_store_delivery_receipts.sql` |
| `store_purchase_requests` | INSERT, UPDATE | S via `character_id` | `0345_store_delivery_receipts.sql` |
| `store_receipt_outbox` | INSERT, UPDATE | S via resolve Character through owning aggregate/receipt | `0345_store_delivery_receipts.sql` |
