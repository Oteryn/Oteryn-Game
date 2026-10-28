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

/// Achievement advancement pop-up and session log.
#[path = "features/0008-achievement-notices/mod.rs"]
pub mod achievement_notices;
/// Bestiary and Charms page state, protocol mapping, views, and settings.
#[path = "features/0001-bestiary-charms/mod.rs"]
pub mod bestiary_charms;
