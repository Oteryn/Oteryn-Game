//! Native client UI composition seam. Feature behavior arrives in later migration steps.
#[macro_use]
#[path = "components/localization/mod.rs"]
pub mod localization;

#[path = "components/bulletin.rs"]
pub mod bulletin;
#[path = "components/guide.rs"]
pub mod guide;
#[path = "components/login_queue.rs"]
pub mod login_queue;

#[path = "features/0025-appearance/mod.rs"]
pub mod appearance;
#[path = "features/0291-cast-chat/mod.rs"]
pub mod cast_chat;
#[path = "features/0292-cast-consent/mod.rs"]
pub mod cast_consent;
#[path = "features/0290-cast-control/mod.rs"]
pub mod cast_control;
#[path = "features/0289-cast-view/mod.rs"]
pub mod cast_view;
#[path = "features/0083-channel-list/mod.rs"]
pub mod channel_list;
#[path = "features/0097-character-info/mod.rs"]
pub mod character_info;
#[path = "features/0067-bestiary-charm-progression/mod.rs"]
pub mod charm_progression;
#[path = "features/0146-container-sort/mod.rs"]
pub mod container_sort;
#[path = "features/0057-cyclopedia-map/mod.rs"]
pub mod cyclopedia_map;
#[path = "features/0371-diagonal-movement/input.rs"]
pub mod diagonal_movement;
#[path = "features/0055-guild-bank/mod.rs"]
pub mod guild_bank;
#[path = "features/0103-guild-war/mod.rs"]
pub mod guild_war;
#[path = "features/0099-guildhall/mod.rs"]
pub mod guildhall;
#[path = "features/0049-house/mod.rs"]
pub mod house;
#[path = "features/0446-house-container-capacity/mod.rs"]
pub mod house_container_capacity;
#[path = "features/0445-house-furniture-rotation/mod.rs"]
pub mod house_furniture_rotation;
#[path = "features/0207-house-furniture-wrap/mod.rs"]
pub mod house_furniture_wrap;
#[path = "features/0059-house-move-out/mod.rs"]
pub mod house_move_out;
#[path = "features/0150-house-podium/mod.rs"]
pub mod house_podium;
#[path = "features/0086-house-transfer/mod.rs"]
pub mod house_transfer;
#[path = "features/0115-mail/mod.rs"]
pub mod mail;
#[path = "features/0134-party-skull/mod.rs"]
pub mod party_skull;
#[path = "features/0076-player-report/mod.rs"]
pub mod player_report;
#[path = "features/0087-pvp-expert/mod.rs"]
pub mod pvp_expert;
#[path = "features/0149-raid-schedule/mod.rs"]
pub mod raid_schedule;
#[path = "features/0039-timed-item/mod.rs"]
pub mod timed_item;
#[path = "features/0054-training/mod.rs"]
pub mod training;

#[path = "features/0014-container-tree/mod.rs"]
pub mod container_tree;
#[path = "features/0082-creature-push/mod.rs"]
pub mod creature_push;
#[path = "features/0012-item-equip-drop/mod.rs"]
pub mod equipment;
#[path = "features/0004-item-view-move/mod.rs"]
pub mod inventory;

#[path = "features/0017-attack/mod.rs"]
pub mod battle;
#[path = "features/0015-item-use/mod.rs"]
pub mod item_use;

#[path = "features/0288-ambient-sound/mod.rs"]
pub mod ambient_sound;
#[path = "features/0350-item-effects/mod.rs"]
pub mod item_effects;
#[path = "features/0019-world-light/mod.rs"]
pub mod world_light;
#[path = "features/0026-world-weather/mod.rs"]
pub mod world_weather;

#[path = "features/0132-inspection/mod.rs"]
pub mod inspection;
#[path = "features/0285-store-purchase-history/mod.rs"]
pub mod purchase_history;
#[path = "features/0098-quick-loot/mod.rs"]
pub mod quick_loot;
#[path = "features/0232-team-finder-members/mod.rs"]
pub mod team_finder;
#[path = "features/0200-team-finder-leader/mod.rs"]
pub mod team_finder_leader;
include!(concat!(env!("OUT_DIR"), "/client_registrations.rs"));

#[path = "features/local-reference-preferences/input.rs"]
pub mod reference_preferences;

/// Account achievement pages from core command 10.
#[path = "features/local-account-achievements/mod.rs"]
pub mod account_achievements;
#[path = "features/0008-achievement-notices/mod.rs"]
pub mod achievement_notices;

#[path = "features/0170-arena/mod.rs"]
pub mod arena;

#[path = "features/0225-boss-difficulty/mod.rs"]
pub mod boss_difficulty;
#[path = "features/0230-browse-field/mod.rs"]
pub mod browse_field;
/// Local speech and named private messages.
#[path = "features/0007-chat/mod.rs"]
pub mod chat;
#[path = "features/0036-chat-guild-room/mod.rs"]
pub mod chat_guild_room;
#[path = "features/0240-chat-party-room/mod.rs"]
pub mod chat_party_room;
#[path = "features/0199-cyclopedia-character/mod.rs"]
pub mod cyclopedia_character;
#[path = "features/0197-cyclopedia-tracker/mod.rs"]
pub mod cyclopedia_tracker;
#[path = "features/0151-daily-boost/mod.rs"]
pub mod daily_boost;
#[path = "features/0037-depot/mod.rs"]
pub mod depot;
#[path = "features/0153-depot-search/mod.rs"]
pub mod depot_search;
#[path = "features/0104-forge-creature/mod.rs"]
pub mod forge_creature;
/// Live server highscores panel.
#[path = "features/0009-highscores/mod.rs"]
pub mod highscores;
#[path = "features/0179-hotkey-equip/mod.rs"]
pub mod hotkey_equip;
#[path = "features/0152-item-forge/mod.rs"]
pub mod item_forge;
#[path = "features/0003-npc-service/mod.rs"]
pub mod npc_dialogue;
#[path = "features/0154-store/mod.rs"]
pub mod store;
#[path = "features/0258-vip-presence/mod.rs"]
pub mod vip_presence;
#[path = "features/0195-weapon-proficiency/mod.rs"]
pub mod weapon_proficiency;
#[path = "features/0448-weapon-proficiency-state/mod.rs"]
pub mod weapon_proficiency_state;

#[path = "features/0053-world-event/mod.rs"]
pub mod world_event;

/// Completed species and bounded Animus mastery pages.
#[path = "features/0168-animus-mastery/mod.rs"]
pub mod animus_mastery;

/// World-interaction click intent mapping.
pub mod world_interaction {
    pub use oteryn_client_runtime::world_interaction as protocol;
    pub mod menu {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/features/0011-world-interaction/state.rs"
        ));
    }
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/features/0011-world-interaction/input.rs"
    ));
}
/// Paced-movement key intent mapping.
pub mod paced_movement {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/features/0013-paced-movement/input.rs"
    ));
}
/// Native skills window with the account Loyalty projection.
#[path = "features/0274-loyalty-skills/mod.rs"]
pub mod loyalty_skills;
/// Quest log panel and input bindings.
#[path = "features/0016-quest-log/mod.rs"]
pub mod quest_log;

/// Logout key intent mapping.
pub mod logout {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/features/0021-logout/input.rs"
    ));
}
#[path = "features/0010-analyser/mod.rs"]
pub mod analyser;
/// Local spell action-bar key intent mapping.
#[path = "features/local-spell-action-bar/mod.rs"]
pub mod spell_action_bar;
#[path = "features/0287-actor-spell-catalog/mod.rs"]
pub mod spell_catalog;
#[path = "features/0449-spellbook-details/mod.rs"]
pub mod spellbook_details;

/// Bestiary and Charms page state, protocol mapping, views, and settings.
#[path = "features/0001-bestiary-charms/mod.rs"]
pub mod bestiary_charms;

/// Editable server-owned Exiva player permissions.
#[path = "features/0147-exiva-settings/mod.rs"]
pub mod exiva_settings;

/// Boss encounter status and Bosstiary progress.
#[path = "features/0117-boss-encounter/mod.rs"]
pub mod boss_encounter;

/// Personal boss reward browsing and custody collection.
#[path = "features/0224-boss-rewards/mod.rs"]
pub mod boss_rewards;
/// Daily reward wall and one-click claim controls.
#[path = "features/0182-daily-reward/mod.rs"]
pub mod daily_reward;
/// Guild state and actions.
#[path = "features/0040-guild/mod.rs"]
pub mod guild;
/// Server-confirmed party membership controls.
#[path = "features/0042-party/mod.rs"]
pub mod party;
/// Direct player-trade panel state and controls.
#[path = "features/0060-player-trade/mod.rs"]
pub mod player_trade;
/// Transient player typing controls and visible-actor indicators.
#[path = "features/0233-player-typing/mod.rs"]
pub mod player_typing;
/// Prey slots, candidate selection, bonuses, and automatic reroll controls.
#[path = "features/0140-prey/mod.rs"]
pub mod prey;
/// Wheel allocation, vessels and Gem Atelier controls.
#[path = "features/0148-wheel-allocation/mod.rs"]
pub mod wheel_allocation;

/// Tutorial notices and their shared server-save history.
#[path = "features/0031-tutorial-notices/mod.rs"]
pub mod tutorial_notices;

#[path = "features/0075-server-save-countdown/protocol.rs"]
pub mod server_save_countdown;

/// Weekly Task Board state and controls.
#[path = "features/0072-task-board/mod.rs"]
pub mod task_board;
#[path = "features/0474-task-board-weekly-results/mod.rs"]
pub mod task_board_weekly_results;

#[path = "features/0051-staff/mod.rs"]
pub mod staff;
#[path = "features/0330-staff-tools/mod.rs"]
pub mod staff_tools;

/// Server-owned stash browsing and conversion controls.
#[path = "features/0062-stash/mod.rs"]
pub mod stash;

/// Native current-floor minimap geometry.
#[path = "features/local-minimap/mod.rs"]
pub mod minimap;

/// Target-walk input and receipt projection.
#[path = "features/0064-target-move/mod.rs"]
pub mod target_move;

/// NPC shop look and sell-all panel state.
#[path = "features/0081-npc-shop/mod.rs"]
pub mod npc_shop;

/// Imbuement shrine state, actions, and plain-text view.
#[path = "features/0068-imbue/mod.rs"]
pub mod imbue;

/// VIP list, group, and consent panel state.
#[path = "features/0043-vip/mod.rs"]
pub mod vip;

/// House banker hireling status and controls.
#[path = "features/0165-hireling/mod.rs"]
pub mod hireling;
#[path = "features/0250-hireling-management/mod.rs"]
pub mod hireling_management;

/// Market offer browsing and settlement controls.
#[path = "features/0063-market/mod.rs"]
pub mod market;

/// Private-channel membership controls and live transcript.
#[path = "features/0080-chat-private-channel/mod.rs"]
pub mod chat_private_channel;
/// Negotiated public chat rooms, their live lines, and room controls.
#[path = "features/0259-chat-public-channels/mod.rs"]
pub mod chat_public_channels;

#[path = "features/0278-battle-instinct-status/mod.rs"]
pub mod battle_instinct_status;
#[path = "features/0276-condition-status/mod.rs"]
pub mod condition_status;
#[path = "features/0382-death-protection/mod.rs"]
pub mod death_protection;
#[path = "features/0384-mechanics-status/mod.rs"]
pub mod mechanics_status;
#[path = "features/0280-party-effects/mod.rs"]
pub mod party_effects;
#[path = "features/0461-party-list/mod.rs"]
pub mod party_list;
#[path = "features/0383-party-member-effects/mod.rs"]
pub mod party_member_effects;
#[path = "features/0381-potion-durations/mod.rs"]
pub mod potion_durations;
#[path = "features/0277-rest-status/mod.rs"]
pub mod rest_status;
#[path = "features/0370-skill-display/mod.rs"]
pub mod skill_display;
#[path = "features/0279-skill-progress/mod.rs"]
pub mod skill_progress;

#[path = "features/0148-wheel-allocation/protocol/gem_presentation.rs"]
pub mod gem_presentation;

#[path = "components/modal_dialogs.rs"]
pub mod modal_dialogs;

#[path = "components/notifications.rs"]
pub mod notifications;
#[path = "features/0148-wheel-allocation/protocol/wheel_presentation.rs"]
pub mod wheel_presentation;

#[path = "features/0450-server-modal/mod.rs"]
pub mod server_modal;

#[path = "features/0479-store-cosmetic-tokens/mod.rs"]
pub mod store_cosmetic_tokens;

#[path = "features/0463-action-bar-cooldowns/mod.rs"]
pub mod action_bar_cooldowns;
#[path = "features/0462-support-intake/mod.rs"]
pub mod support_intake;
