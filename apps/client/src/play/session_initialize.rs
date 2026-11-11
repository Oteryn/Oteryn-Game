use super::*;

pub(super) fn initialize<T: Stepper>(session: &T, outbox: &Outbox) -> bool {
    outbox.discovery_initialize(session);
    outbox.hud_stamina_initialize(session.feature_selected(199));
    outbox.properties_initialize(session);
    outbox.analyser_initialize(session.feature_selected(10));
    if let Err(error) = outbox
        .spell_catalog_initialize(session.feature_selected(287), session.feature_snapshot(100))
    {
        outbox.end(public_class(&error));
        return false;
    }
    outbox.battle_metadata_initialize(session.server_mechanic_selected(17));
    outbox.spell_bar_initialize(session.feature_snapshot(20));
    outbox.bar_item_cooldowns_initialize(session.feature_selected(463));
    outbox.appearance_initialize(session);
    outbox.equipment_initialize(session.equipment_selected());
    outbox.world_context_initialize(session.feature_selected(28), session.feature_selected(11));
    outbox.targeting_initialize(session.feature_selected(15));
    outbox.diagonal_movement_initialize(session.feature_selected(371));
    // Publish sources only after their equipment, movement and targeting routes are ready.
    outbox.inventory_initialize(session.inventory(), session.item_count_selected());
    outbox.container_initialize_with_house_capacity(
        session.container_views(),
        session.feature_selected(446),
    );
    outbox.target_walk_initialize(session.feature_selected(64));
    outbox.vitals(session.actor_vitals());
    outbox.channel_list_initialize(session.feature_snapshot(48));
    outbox.world_light_initialize(session.feature_snapshot(18));
    outbox.world_weather_initialize(session.feature_snapshot(22));
    outbox.map_initialize(session.feature_snapshot(17));
    outbox.animus_initialize(session.animus_selected());
    outbox.wheel_initialize(session.wheel_selected());
    outbox.gem_presentation_initialize(session.feature_selected(148));
    outbox.wheel_presentation_initialize(session.feature_selected(148));
    outbox.exiva_initialize(session.exiva_selected());
    outbox.loyalty_skills_initialize(session.loyalty_skills_selected());
    outbox.spellbook_details_initialize(session.feature_selected(449));
    outbox.mechanics_status_initialize(session.feature_selected(384));
    outbox.condition_status_initialize(session.feature_selected(276));
    outbox.rest_status_initialize(session.feature_selected(277));
    outbox.battle_instinct_status_initialize(session.feature_selected(278));
    outbox.skill_progress_initialize(session.feature_selected(279));
    outbox.skill_display_initialize(session.feature_selected(370));
    outbox.party_effects_initialize(session.feature_selected(280));
    outbox.potion_durations_initialize(session.feature_selected(381));
    outbox.death_protection_initialize(session.feature_selected(382));
    outbox.quest_initialize(session.quest_state());
    outbox.task_board_initialize(session.task_board_selected());
    outbox.task_board_weekly_results_initialize(session.feature_selected(474));
    outbox.stash_initialize(session.stash_selected());
    outbox.dialogue_initialize(session);
    outbox.npc_shop_initialize(session);
    outbox.prey_initialize(session.prey_selected());
    outbox.daily_reward_initialize(session.daily_reward_selected());
    outbox.boss_rewards_initialize(session.boss_rewards_selected());
    outbox.imbue_initialize(session.imbue_selected());
    outbox.player_trade_initialize(
        session.player_trade_selected(),
        session.player_trade_state(),
    );
    outbox.vip_initialize(session.vip_selected());
    outbox.guild_initialize(session.guild_selected());
    outbox.party_initialize(
        session.party_selected(),
        session.feature_selected(oteryn_session::CAPABILITY_CHAT_V1),
    );
    outbox.hireling_initialize(session.hireling_selected());
    outbox.hireling_management_initialize(session.feature_selected(250));
    outbox.market_initialize(session.market_selected());
    outbox.chat_with_movement_initialize(session);
    outbox.chat_private_publish(session);
    outbox.player_typing_initialize(session.player_typing_selected());
    outbox.arena_initialize(session.feature_selected(170));
    outbox.boss_encounter_initialize(session.feature_selected(117));
    outbox.boss_difficulty_initialize(session.feature_selected(225));
    outbox.cast_control_initialize(session.server_mechanic_selected(290));
    outbox.cast_consent_initialize(session.server_mechanic_selected(292));
    outbox.cast_chat_initialize(
        session.server_mechanic_selected(290) && session.server_mechanic_selected(291),
    );
    outbox.browse_field_initialize(session.server_mechanic_selected(230));
    if let Some(payload) = session.feature_snapshot(14) {
        outbox.browse_field_views(&payload);
    }
    if let Some(inventory) = session.inventory() {
        outbox.browse_field_inventory(inventory);
    }
    outbox.guild_chat_publish(session);
    outbox.chat_initialize(session.feature_selected(oteryn_session::CAPABILITY_CHAT_V1));
    outbox.chat_party_room_initialize(session.feature_selected(240));
    outbox.cyclopedia_character_initialize(session.feature_selected(199));
    outbox.cyclopedia_tracker_initialize(session.feature_selected(197));
    outbox.world_event_initialize(session.feature_selected(53));
    outbox.daily_boost_initialize(session.feature_selected(151));
    outbox.highscores_initialize(session.server_mechanic_selected(9));
    outbox.depot_search_initialize(session.server_mechanic_selected(153));
    outbox.target_move_initialize(session.feature_selected(64));
    outbox.creature_push_initialize(session.feature_selected(82));
    outbox.hotkey_equip_initialize(session.server_mechanic_selected(179));
    outbox.hotkey_equip_smart_mode(session.server_mechanic_selected(180));
    outbox.hotkey_equip_items(session.hotkey_equip_items());
    outbox.hotkey_own_actor(session.hotkey_own_actor());
    outbox.item_forge_initialize(session.server_mechanic_selected(152));
    outbox.store_initialize(
        session.server_mechanic_selected(154),
        session.feature_selected(450),
    );
    outbox.store_tokens_initialize(session.server_mechanic_selected(479));
    outbox.premium_game_initialize(session.feature_selected(97));
    outbox.vip_presence_initialize(session.feature_selected(258));
    outbox.weapon_proficiency_initialize(session.feature_selected(195));
    outbox.weapon_proficiency_state_initialize(session.feature_selected(448));
    outbox.depot_initialize(session.feature_selected(37));
    outbox.achievements_initialize();
    outbox.player_trade_items(session.player_trade_items());
    true
}
