//! Native client UI composition seam. Feature behavior arrives in later migration steps.

#[path = "features/0019-world-light/mod.rs"]
pub mod world_light;

include!(concat!(env!("OUT_DIR"), "/client_registrations.rs"));

/// Account achievement pages from core command 10.
#[path = "features/local-account-achievements/mod.rs"]
pub mod account_achievements;
#[path = "features/0008-achievement-notices/mod.rs"]
pub mod achievement_notices;

#[path = "features/0225-boss-difficulty/mod.rs"]
pub mod boss_difficulty;
#[path = "features/0230-browse-field/mod.rs"]
pub mod browse_field;
/// Local speech and named private messages.
#[path = "features/0007-chat/mod.rs"]
pub mod chat;
#[path = "features/0240-chat-party-room/mod.rs"]
pub mod chat_party_room;
#[path = "features/0199-cyclopedia-character/mod.rs"]
pub mod cyclopedia_character;
#[path = "features/0197-cyclopedia-tracker/mod.rs"]
pub mod cyclopedia_tracker;
#[path = "features/0151-daily-boost/mod.rs"]
pub mod daily_boost;
#[path = "features/0153-depot-search/mod.rs"]
pub mod depot_search;
/// Live server highscores panel.
#[path = "features/0009-highscores/mod.rs"]
pub mod highscores;
#[path = "features/0179-hotkey-equip/mod.rs"]
pub mod hotkey_equip;
#[path = "features/0152-item-forge/mod.rs"]
pub mod item_forge;
#[path = "features/0154-store/mod.rs"]
pub mod store;
#[path = "features/0258-vip-presence/mod.rs"]
pub mod vip_presence;
#[path = "features/0195-weapon-proficiency/mod.rs"]
pub mod weapon_proficiency;

#[path = "features/0053-world-event/mod.rs"]
pub mod world_event;

/// Completed species and bounded Animus mastery pages.
#[path = "features/0168-animus-mastery/mod.rs"]
pub mod animus_mastery;

/// World-interaction click intent mapping.
pub mod world_interaction {
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
/// Local spell action-bar key intent mapping.
#[path = "features/local-spell-action-bar/mod.rs"]
pub mod spell_action_bar;

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

/// Market offer browsing and settlement controls.
#[path = "features/0063-market/mod.rs"]
pub mod market;

/// Negotiated public chat rooms, their live lines, and room controls.
#[path = "features/0259-chat-public-channels/mod.rs"]
pub mod chat_public_channels;
