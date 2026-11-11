//! Existing session support allowlist and registry-derived raw routes.
use super::*;

/// The capabilities this client implements and advertises by default: 4 `ITEM_VIEW_MOVE_V1`
/// (domains 9 and 11, command 9, the USE item target and the item handle on domain-1 entities),
/// 6 `WORLD_SPATIAL_ENTITIES` (the domain-1 type 2 snapshot and delta with every visible entity), 7 `CHAT_V1` (command 13 and
/// domain 12), 8 `ACHIEVEMENT_NOTICES_V1` (domain 13 snapshot and `AchievementEarned` deltas),
/// 13 `PACED_MOVEMENT_V1` (the step result `TOO_EARLY`),
/// 16 `QUEST_LOG_V1` (domain 16 and command 22), and 17 `ATTACK_V1` (command
/// types 11 and 12, domain 10; requires 6) and 21 `LOGOUT_V1` (command type 23). Add an ID here only
/// together with its routing, and keep the set closed under the registry's `requires`.
/// 147 edits Exiva settings through command 170. Field runes (91) reuse USE command 2,
/// existing map/entity updates and the rune refusal decoder; 11, 15, 18, 22 and 30
/// complete their prerequisite closure on shipped bundle Worlds.
/// 272 adds live Atelier resource queries to the existing 148 panel. The mechanic panels added
/// by CLIENT-E2E-1 advertise their typed command routes so the shipped session loop and the
/// qualification client negotiate the same feature set. NPC service and staff use the registry
/// raw routes; negotiating staff never grants the signed-envelope authority it requires.
/// 92 consumes ordered server status, warning and advance messages on domain 57.
pub const CLIENT_SUPPORTED_CAPABILITIES: &[u32] = &[
    oteryn_protocol_oteryn::npc_service::CAPABILITY_NPC_SERVICE_V1,
    CAPABILITY_ITEM_VIEW_MOVE_V1,
    CAPABILITY_WORLD_SPATIAL_ENTITIES,
    CAPABILITY_CHAT_V1,
    CAPABILITY_ACHIEVEMENT_NOTICES_V1,
    highscores::CAPABILITY_HIGHSCORES_V1,
    CAPABILITY_ANALYSER_V1,
    oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1,
    oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_EQUIP_DROP_V1,
    CAPABILITY_PACED_MOVEMENT_V1,
    CAPABILITY_CONTAINER_TREE_V1,
    oteryn_protocol_oteryn::world_object::CAPABILITY_ITEM_USE_V1,
    quest_log::CAPABILITY_QUEST_LOG_V1,
    CAPABILITY_ATTACK_V1,
    oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1,
    oteryn_protocol_oteryn::world_clock::CAPABILITY_WORLD_LIGHT_V1,
    oteryn_protocol_oteryn::connection_trace::CAPABILITY_CONNECTION_TRACE_V1,
    logout::CAPABILITY_LOGOUT_V1,
    oteryn_protocol_oteryn::world_object::CAPABILITY_RUNE_USE_V1,
    oteryn_protocol_oteryn::presentation::CAPABILITY_PRESENTATION_V1,
    oteryn_protocol_oteryn::appearance::CAPABILITY_APPEARANCE_V1,
    oteryn_protocol_oteryn::world_weather::CAPABILITY_WORLD_WEATHER_V1,
    oteryn_protocol_oteryn::capability_tables::CAPABILITY_ITEM_LOOK_V1,
    oteryn_protocol_oteryn::capability_tables::CAPABILITY_WORLD_SPATIAL_FIELDS,
    oteryn_protocol_oteryn::tutorial_notices::CAPABILITY_TUTORIAL_NOTICES_V1,
    chat::guild_room::CAPABILITY_CHAT_GUILD_ROOM_V1,
    oteryn_protocol_oteryn::depot::CAPABILITY_DEPOT_V1,
    property_services::timed_item::CAPABILITY_TIMED_ITEM_V1,
    guild::CAPABILITY_GUILD_V1,
    party::CAPABILITY_PARTY_V1,
    vip::CAPABILITY_VIP_V1,
    // House is the registry prerequisite for the podium editor.
    oteryn_protocol_oteryn::house::CAPABILITY_HOUSE_V1,
    oteryn_protocol_oteryn::staff::CAPABILITY_STAFF_V1,
    world_event::CAPABILITY_WORLD_EVENT_V1,
    property_services::loop_training::CAPABILITY_TRAINING_V1,
    property_services::guild_bank::CAPABILITY_GUILD_BANK_V1,
    property_services::cyclopedia_map::CAPABILITY_CYCLOPEDIA_MAP_V1,
    oteryn_protocol_oteryn::house_move_out::CAPABILITY_HOUSE_MOVE_OUT_V1,
    player_trade::CAPABILITY_PLAYER_TRADE_V1,
    oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_STACK_COUNT_V1,
    stash::CAPABILITY_STASH_V1,
    market::CAPABILITY_MARKET_V1,
    oteryn_protocol_oteryn::target_move::CAPABILITY_TARGET_MOVE_V1,
    oteryn_protocol_oteryn::refusal_detail::CAPABILITY_REFUSAL_DETAIL_V1,
    property_services::charm_progression::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
    imbuement::CAPABILITY_IMBUE_V1,
    task_board::CAPABILITY_TASK_BOARD_V1,
    oteryn_protocol_oteryn::server_save_countdown::CAPABILITY_SERVER_SAVE_COUNTDOWN_V1,
    // CL26s1: private-channel prerequisite of public channels (259); ids stay ascending.
    chat::private_channel::CAPABILITY_CHAT_PRIVATE_CHANNEL_V1,
    oteryn_protocol_oteryn::npc_shop::CAPABILITY_NPC_SHOP_V1,
    oteryn_protocol_oteryn::creature_push::CAPABILITY_CREATURE_PUSH_V1,
    oteryn_protocol_oteryn::channel_list::CAPABILITY_CHANNEL_LIST_V1,
    oteryn_protocol_oteryn::house_transfer::CAPABILITY_HOUSE_TRANSFER_V1,
    property_services::pvp_expert::CAPABILITY_PVP_EXPERT_V1,
    oteryn_protocol_oteryn::world_object::CAPABILITY_FIELD_RUNES_V1,
    oteryn_protocol_oteryn::zones_nologout::CAPABILITY_ZONES_NOLOGOUT_V1,
    // CL26s4: boss encounter prerequisite of boss rewards (224).
    property_services::character_info::CAPABILITY_CHARACTER_INFO_V1,
    oteryn_protocol_oteryn::quick_loot::CAPABILITY_QUICK_LOOT_V1,
    oteryn_protocol_oteryn::house::CAPABILITY_GUILDHALL_V1,
    oteryn_protocol_oteryn::appearance::CAPABILITY_FORGE_CREATURE_V1,
    property_services::mail::CAPABILITY_MAIL_V1 as u32,
    oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
    oteryn_protocol_oteryn::inspection::CAPABILITY_INSPECTION_V1,
    property_services::party_skull::CAPABILITY_PARTY_SKULL_V1,
    prey::CAPABILITY_PREY_V1,
    oteryn_protocol_oteryn::exiva::CAPABILITY_EXIVA_SETTINGS_V1,
    wheel::CAPABILITY_WHEEL_ALLOCATION_V1,
    property_services::raid_schedule::CAPABILITY_RAID_SCHEDULE_V1,
    property_services::house_podium::CAPABILITY_HOUSE_PODIUM_V1,
    oteryn_protocol_oteryn::daily_boost::CAPABILITY_DAILY_BOOST_V1,
    oteryn_protocol_oteryn::item_forge::CAPABILITY_ITEM_FORGE_V1,
    oteryn_protocol_oteryn::depot_search::CAPABILITY_DEPOT_SEARCH_V1,
    oteryn_protocol_oteryn::store::CAPABILITY_STORE_V1,
    hireling::CAPABILITY_HIRELING_V1,
    animus_mastery::CAPABILITY_ANIMUS_MASTERY_V1,
    // CL26s3: native daily reward wall, query, and claim controls.
    oteryn_protocol_oteryn::hotkey_equip::CAPABILITY_HOTKEY_EQUIP_V1,
    oteryn_protocol_oteryn::hotkey_equip::CAPABILITY_HOTKEY_EQUIP_CARRIED_V1,
    oteryn_protocol_oteryn::daily_reward::CAPABILITY_DAILY_REWARD_V2,
    oteryn_protocol_oteryn::weapon_proficiency::CAPABILITY_WEAPON_PROFICIENCY_V1,
    oteryn_protocol_oteryn::cyclopedia_character::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
    // CL26s4: personal boss reward commands.
    oteryn_protocol_oteryn::team_finder::CAPABILITY_TEAM_FINDER_LEADER_V1,
    property_services::house_wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_V1,
    boss_rewards::CAPABILITY_BOSS_REWARDS_V1,
    // CL26s2: native player typing command and CHAT delta support.
    oteryn_protocol_oteryn::browse_field::CAPABILITY_BROWSE_FIELD_V1,
    oteryn_protocol_oteryn::team_finder_members::CAPABILITY_TEAM_FINDER_MEMBERS_V1,
    player_typing::CAPABILITY_PLAYER_TYPING_V1,
    // CL26s1: public channels.
    chat::party_room::CAPABILITY_CHAT_PARTY_ROOM_V1,
    oteryn_protocol_oteryn::vip_presence::CAPABILITY_VIP_PRESENCE_V1,
    chat::public_channels::CAPABILITY_CHAT_PUBLIC_CHANNELS_V1,
    atelier::resources::CAPABILITY_ATELIER_RESOURCES_V1,
    loyalty_skills::CAPABILITY_LOYALTY_SKILLS_V1,
    oteryn_protocol_oteryn::appearance::CAPABILITY_EMPTY_OUTFIT_V1,
    condition_status::CAPABILITY_CONDITION_STATUS_V1,
    rest_status::CAPABILITY_REST_STATUS_V1,
    battle_instinct_status::CAPABILITY_BATTLE_INSTINCT_STATUS_V1,
    skill_progress::CAPABILITY_SKILL_PROGRESS_V1,
    party_effects::CAPABILITY_PARTY_EFFECTS_V1,
    oteryn_protocol_oteryn::store::CAPABILITY_STORE_PURCHASE_HISTORY_V1,
    oteryn_protocol_oteryn::entity_names::CAPABILITY_WORLD_SPATIAL_ENTITIES_V2,
    spell_catalog::CAPABILITY_ACTOR_SPELL_CATALOG_V1,
    oteryn_protocol_oteryn::presentation::CAPABILITY_AMBIENT_SOUND_V1,
    oteryn_protocol_oteryn::cast::CAPABILITY_CAST_CONTROL_V1,
    oteryn_protocol_oteryn::cast::CAPABILITY_CAST_CHAT_V1,
    oteryn_protocol_oteryn::cast::CAPABILITY_CAST_CONSENT_V1,
    oteryn_protocol_oteryn::staff_tools::CAPABILITY_STAFF_TOOLS_V1,
    oteryn_protocol_oteryn::presentation::CAPABILITY_ITEM_EFFECTS_V1,
    skill_display::CAPABILITY_SKILL_DISPLAY_V1,
    world_spatial::CAPABILITY_DIAGONAL_MOVEMENT_V1,
    potion_durations::CAPABILITY_POTION_DURATIONS_V1,
    death_protection::CAPABILITY_DEATH_PROTECTION_V1,
    party::CAPABILITY_PARTY_MEMBER_EFFECTS_V1,
    mechanics_status::CAPABILITY_MECHANICS_STATUS_V1,
    property_services::house_rotation::CAPABILITY_HOUSE_FURNITURE_ROTATION_V1,
    oteryn_protocol_oteryn::container_tree::CAPABILITY_HOUSE_CONTAINER_CAPACITY_V2,
    oteryn_protocol_oteryn::imbuement_details::CAPABILITY_IMBUEMENT_DETAILS_V1,
    oteryn_protocol_oteryn::weapon_proficiency_state::CAPABILITY_WEAPON_PROFICIENCY_STATE_V1,
    spell_catalog::spellbook_details::CAPABILITY_SPELLBOOK_DETAILS_V1,
    oteryn_protocol_oteryn::server_modal::CAPABILITY_SERVER_MODAL_V1,
    party::CAPABILITY_PARTY_LIST_V1,
    oteryn_protocol_oteryn::support_intake::CAPABILITY_SUPPORT_INTAKE_V1,
    oteryn_protocol_oteryn::action_bar_cooldowns::CAPABILITY_ACTION_BAR_COOLDOWNS_V1,
    imbuement::CAPABILITY_IMBUEMENT_SCROLL_CATALOGUE_V1,
    oteryn_protocol_oteryn::task_board::full::CAPABILITY_WEEKLY_RESULTS_V1,
    oteryn_protocol_oteryn::store::cosmetic_tokens::CAPABILITY,
    // WU26-14 spell aim mode and WU26-10 Gem Atelier v2 (requires 148).
    actor_spell::CAPABILITY_SPELL_AIM_MODE_V1,
    atelier::v2::CAPABILITY_GEM_ATELIER_V2,
    // WU26 package 2: tiered hotkey equip, hit attribution, promotion title, death details,
    // podium flags, server modal input and imbuement success chance.
    hotkey_equip::CAPABILITY_HOTKEY_EQUIP_UPGRADE_TIER_V1,
    analyser_transport::CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1,
    cyclopedia_character::CAPABILITY_CYCLOPEDIA_CHARACTER_VOCATION_V1,
    cyclopedia_character::CAPABILITY_CYCLOPEDIA_DEATH_DETAILS_V1,
    property_services::house_podium::CAPABILITY_HOUSE_PODIUM_FLAGS_V1,
    oteryn_protocol_oteryn::server_modal::input::CAPABILITY_SERVER_MODAL_INPUT_V1,
    imbuement::CAPABILITY_IMBUEMENT_SUCCESS_CHANCE_V1,
    // WU26 boss pity tracker in the boss difficulty window (requires 225).
    oteryn_protocol_oteryn::boss_difficulty::CAPABILITY_BOSS_DIFFICULTY_PITY_V1,
];

/// Capabilities supported by the raw command/snapshot/delta routes, including opt-in
/// capabilities absent from `CLIENT_SUPPORTED_CAPABILITIES`. Typed routes stay separate.
const CLIENT_GATED_CAPABILITIES: &[u32] = &[
    7,
    oteryn_protocol_oteryn::house::CAPABILITY_HOUSE_V1,
    daily_reward::CAPABILITY_DAILY_REWARD_V2,
    oteryn_protocol_oteryn::npc_service::CAPABILITY_NPC_SERVICE_V1,
    oteryn_protocol_oteryn::staff::CAPABILITY_STAFF_V1,
    // CLIENT-E2E-1 adapters use gated_command for their typed exchanges too.
    CAPABILITY_BESTIARY_CHARMS_V1,
    CAPABILITY_ACHIEVEMENT_NOTICES_V1,
    highscores::CAPABILITY_HIGHSCORES_V1,
    CAPABILITY_ANALYSER_V1,
    oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1,
    CAPABILITY_CONTAINER_TREE_V1,
    quest_log::CAPABILITY_QUEST_LOG_V1,
    CAPABILITY_ATTACK_V1,
    oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1,
    oteryn_protocol_oteryn::world_clock::CAPABILITY_WORLD_LIGHT_V1,
    oteryn_protocol_oteryn::presentation::CAPABILITY_PRESENTATION_V1,
    oteryn_protocol_oteryn::appearance::CAPABILITY_APPEARANCE_V1,
    oteryn_protocol_oteryn::world_weather::CAPABILITY_WORLD_WEATHER_V1,
    oteryn_protocol_oteryn::capability_tables::CAPABILITY_ITEM_LOOK_V1,
    oteryn_protocol_oteryn::tutorial_notices::CAPABILITY_TUTORIAL_NOTICES_V1,
    oteryn_protocol_oteryn::depot::CAPABILITY_DEPOT_V1,
    property_services::timed_item::CAPABILITY_TIMED_ITEM_V1,
    guild::CAPABILITY_GUILD_V1,
    party::CAPABILITY_PARTY_V1,
    vip::CAPABILITY_VIP_V1,
    world_event::CAPABILITY_WORLD_EVENT_V1,
    property_services::loop_training::CAPABILITY_TRAINING_V1,
    property_services::guild_bank::CAPABILITY_GUILD_BANK_V1,
    property_services::cyclopedia_map::CAPABILITY_CYCLOPEDIA_MAP_V1,
    oteryn_protocol_oteryn::house_move_out::CAPABILITY_HOUSE_MOVE_OUT_V1,
    player_trade::CAPABILITY_PLAYER_TRADE_V1,
    stash::CAPABILITY_STASH_V1,
    market::CAPABILITY_MARKET_V1,
    oteryn_protocol_oteryn::target_move::CAPABILITY_TARGET_MOVE_V1,
    property_services::charm_progression::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
    imbuement::CAPABILITY_IMBUE_V1,
    task_board::CAPABILITY_TASK_BOARD_V1,
    oteryn_protocol_oteryn::server_save_countdown::CAPABILITY_SERVER_SAVE_COUNTDOWN_V1,
    oteryn_protocol_oteryn::npc_shop::CAPABILITY_NPC_SHOP_V1,
    oteryn_protocol_oteryn::creature_push::CAPABILITY_CREATURE_PUSH_V1,
    oteryn_protocol_oteryn::channel_list::CAPABILITY_CHANNEL_LIST_V1,
    oteryn_protocol_oteryn::house_transfer::CAPABILITY_HOUSE_TRANSFER_V1,
    property_services::pvp_expert::CAPABILITY_PVP_EXPERT_V1,
    oteryn_protocol_oteryn::zones_nologout::CAPABILITY_ZONES_NOLOGOUT_V1,
    oteryn_protocol_oteryn::house::CAPABILITY_GUILDHALL_V1,
    property_services::mail::CAPABILITY_MAIL_V1 as u32,
    oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
    property_services::character_info::CAPABILITY_CHARACTER_INFO_V1,
    oteryn_protocol_oteryn::quick_loot::CAPABILITY_QUICK_LOOT_V1,
    oteryn_protocol_oteryn::inspection::CAPABILITY_INSPECTION_V1,
    property_services::party_skull::CAPABILITY_PARTY_SKULL_V1,
    prey::CAPABILITY_PREY_V1,
    oteryn_protocol_oteryn::exiva::CAPABILITY_EXIVA_SETTINGS_V1,
    wheel::CAPABILITY_WHEEL_ALLOCATION_V1,
    property_services::raid_schedule::CAPABILITY_RAID_SCHEDULE_V1,
    property_services::house_podium::CAPABILITY_HOUSE_PODIUM_V1,
    property_services::house_wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_V1,
    property_services::house_rotation::CAPABILITY_HOUSE_FURNITURE_ROTATION_V1,
    oteryn_protocol_oteryn::container_tree::CAPABILITY_HOUSE_CONTAINER_CAPACITY_V2,
    oteryn_protocol_oteryn::daily_boost::CAPABILITY_DAILY_BOOST_V1,
    oteryn_protocol_oteryn::item_forge::CAPABILITY_ITEM_FORGE_V1,
    oteryn_protocol_oteryn::depot_search::CAPABILITY_DEPOT_SEARCH_V1,
    oteryn_protocol_oteryn::store::CAPABILITY_STORE_V1,
    hireling::CAPABILITY_HIRELING_V1,
    animus_mastery::CAPABILITY_ANIMUS_MASTERY_V1,
    oteryn_protocol_oteryn::hotkey_equip::CAPABILITY_HOTKEY_EQUIP_V1,
    oteryn_protocol_oteryn::hotkey_equip::CAPABILITY_HOTKEY_EQUIP_CARRIED_V1,
    oteryn_protocol_oteryn::weapon_proficiency::CAPABILITY_WEAPON_PROFICIENCY_V1,
    oteryn_protocol_oteryn::cyclopedia_tracker::CAPABILITY_CYCLOPEDIA_TRACKER_V1,
    oteryn_protocol_oteryn::cyclopedia_character::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
    oteryn_protocol_oteryn::boss_difficulty::CAPABILITY_BOSS_DIFFICULTY_V1,
    oteryn_protocol_oteryn::team_finder::CAPABILITY_TEAM_FINDER_LEADER_V1,
    oteryn_protocol_oteryn::browse_field::CAPABILITY_BROWSE_FIELD_V1,
    oteryn_protocol_oteryn::team_finder_members::CAPABILITY_TEAM_FINDER_MEMBERS_V1,
    player_typing::CAPABILITY_PLAYER_TYPING_V1,
    atelier::resources::CAPABILITY_ATELIER_RESOURCES_V1,
    loyalty_skills::CAPABILITY_LOYALTY_SKILLS_V1,
    condition_status::CAPABILITY_CONDITION_STATUS_V1,
    rest_status::CAPABILITY_REST_STATUS_V1,
    battle_instinct_status::CAPABILITY_BATTLE_INSTINCT_STATUS_V1,
    skill_progress::CAPABILITY_SKILL_PROGRESS_V1,
    party_effects::CAPABILITY_PARTY_EFFECTS_V1,
    oteryn_protocol_oteryn::store::CAPABILITY_STORE_PURCHASE_HISTORY_V1,
    spell_catalog::CAPABILITY_ACTOR_SPELL_CATALOG_V1,
    oteryn_protocol_oteryn::presentation::CAPABILITY_AMBIENT_SOUND_V1,
    oteryn_protocol_oteryn::cast::CAPABILITY_CAST_CONTROL_V1,
    oteryn_protocol_oteryn::cast::CAPABILITY_CAST_CHAT_V1,
    oteryn_protocol_oteryn::cast::CAPABILITY_CAST_CONSENT_V1,
    oteryn_protocol_oteryn::staff_tools::CAPABILITY_STAFF_TOOLS_V1,
    oteryn_protocol_oteryn::presentation::CAPABILITY_ITEM_EFFECTS_V1,
    skill_display::CAPABILITY_SKILL_DISPLAY_V1,
    mechanics_status::CAPABILITY_MECHANICS_STATUS_V1,
    oteryn_protocol_oteryn::imbuement_details::CAPABILITY_IMBUEMENT_DETAILS_V1,
    imbuement::CAPABILITY_IMBUEMENT_SCROLL_CATALOGUE_V1,
    potion_durations::CAPABILITY_POTION_DURATIONS_V1,
    death_protection::CAPABILITY_DEATH_PROTECTION_V1,
    party::CAPABILITY_PARTY_MEMBER_EFFECTS_V1,
    oteryn_protocol_oteryn::weapon_proficiency_state::CAPABILITY_WEAPON_PROFICIENCY_STATE_V1,
    spell_catalog::spellbook_details::CAPABILITY_SPELLBOOK_DETAILS_V1,
    oteryn_protocol_oteryn::server_modal::CAPABILITY_SERVER_MODAL_V1,
    oteryn_protocol_oteryn::store::cosmetic_tokens::CAPABILITY,
    oteryn_protocol_oteryn::action_bar_cooldowns::CAPABILITY_ACTION_BAR_COOLDOWNS_V1,
    party::CAPABILITY_PARTY_LIST_V1,
    oteryn_protocol_oteryn::support_intake::CAPABILITY_SUPPORT_INTAKE_V1,
    oteryn_protocol_oteryn::task_board::full::CAPABILITY_WEEKLY_RESULTS_V1,
    atelier::v2::CAPABILITY_GEM_ATELIER_V2,
];

pub(super) fn gated_routes() -> impl Iterator<Item = (u32, &'static [u32], &'static [u32])> {
    oteryn_protocol_oteryn::capability_tables::GATED
        .iter()
        .copied()
        .filter(|(capability, _, _)| CLIENT_GATED_CAPABILITIES.contains(capability))
}

pub(super) fn capability_of_domain(domain_id: u32) -> Option<u32> {
    gated_routes()
        .find(|(_, _, domains)| domains.contains(&domain_id))
        .map(|(capability, _, _)| capability)
}

#[cfg(test)]
mod target_move_tests {
    use super::*;

    #[test]
    fn target_walk_is_advertised_and_routed_with_its_prerequisites() {
        for capability in [6, 13, 64] {
            assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&capability));
        }
        assert!(gated_routes().any(|(id, commands, _)| id == 64 && commands.contains(&81)));
    }
}

#[cfg(test)]
mod creature_push_tests {
    use super::*;

    #[test]
    fn creature_push_is_advertised_and_routed_with_its_prerequisites() {
        for capability in [6, 13, 82] {
            assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&capability));
        }
        assert!(gated_routes().any(|(id, commands, _)| id == 82 && commands.contains(&98)));
    }
}

#[cfg(test)]
mod weapon_proficiency_tests {
    use super::*;

    #[test]
    fn proficiency_browse_is_advertised_and_routed_with_item_identity() {
        for capability in [4, 195] {
            assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&capability));
        }
        assert!(gated_routes().any(|(id, commands, _)| id == 195 && commands.contains(&641)));
    }
}

#[cfg(test)]
mod hotkey_equip_tests {
    use super::*;

    #[test]
    fn hotkey_equip_carried_is_advertised_with_its_command_owner() {
        for capability in [179, 180] {
            assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&capability));
        }
        assert!(gated_routes().any(|(id, commands, _)| id == 179 && commands.contains(&348)));
    }
}

#[cfg(test)]
mod native_advertisement_tests {
    use super::CLIENT_SUPPORTED_CAPABILITIES;
    use oteryn_protocol_oteryn::{
        ChannelId, CharacterId, ClientBootstrapValue, FoundationProtocolError, GameSessionId,
        ServerAcceptedValue, WorldId, decode_wire_envelope, encode_client_bootstrap,
        encode_server_accepted,
    };

    #[test]
    fn native_advertisement_is_bootstrap_encodable_and_dependency_closed()
    -> Result<(), FoundationProtocolError> {
        let id = |last: u8| {
            let mut bytes = [1; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes[15] = last;
            bytes
        };
        let bootstrap = encode_client_bootstrap(&ClientBootstrapValue {
            schema_revision: 1,
            character_id: CharacterId::decode(&id(1))?,
            admission_material: b"native-advertisement-regression",
            client_build_id: "native-advertisement-regression",
            supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
        })?;
        assert_eq!(
            decode_wire_envelope(&bootstrap)?
                .client_bootstrap()?
                .supported_capabilities,
            CLIENT_SUPPORTED_CAPABILITIES
        );
        // The real acceptance encoder additionally checks registration and every
        // transitive registry prerequisite when the full advertisement is selected.
        encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&id(2))?,
            world_id: WorldId::decode(&id(3))?,
            channel_id: ChannelId::decode(&id(4))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod house_furniture_tests {
    use super::*;
    #[test]
    fn house_furniture_actions_are_advertised_and_have_real_gated_routes() {
        for capability in [49, 207, 445] {
            assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&capability));
        }
        for (capability, command) in [(207, 402), (445, 730)] {
            assert!(
                gated_routes()
                    .any(|(id, commands, _)| id == capability && commands.contains(&command))
            );
        }
    }
}

#[cfg(test)]
#[test]
fn battle_metadata_uses_existing_attack_capability() -> Result<(), &'static str> {
    assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&CAPABILITY_ATTACK_V1));
    assert!(gated_routes().any(
        |(capability, commands, _)| capability == CAPABILITY_ATTACK_V1
            && commands.contains(
                &oteryn_protocol_oteryn::battle_metadata::COMMAND_TYPE_BATTLE_METADATA_QUERY
            )
    ));
    Ok(())
}

#[cfg(test)]
mod action_bar_cooldown_tests {
    use super::*;
    #[test]
    fn action_bar_item_cooldown_query_is_supported_and_gated() {
        assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&463));
        assert!(gated_routes().any(|(id, commands, _)| id == 463 && commands.contains(&6147)));
    }
}
