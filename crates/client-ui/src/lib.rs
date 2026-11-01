//! Native client UI composition seam. Feature behavior arrives in later migration steps.

include!(concat!(env!("OUT_DIR"), "/client_registrations.rs"));

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

/// Prey slots, candidate selection, bonuses, and automatic reroll controls.
#[path = "features/0140-prey/mod.rs"]
pub mod prey;
/// Direct player-trade panel state and controls.
#[path = "features/0060-player-trade/mod.rs"]
pub mod player_trade;
/// Guild state and actions.
#[path = "features/0040-guild/mod.rs"]
pub mod guild;
/// Server-confirmed party membership controls.
#[path = "features/0042-party/mod.rs"]
pub mod party;
/// Wheel allocation, vessels and Gem Atelier controls.
#[path = "features/0148-wheel-allocation/mod.rs"]
pub mod wheel_allocation;

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
