//! Closed source bindings for ordinary creature-death quest callbacks.
//! Crystal 9f5a72c, global Renegade Orc definition (157b75d352a131c57f7dabda0bc2f9699d5d9b66)
//! registers RenegadeOrcDeath (8d1ecd7810b863df6f44e9e4aead98264fe50395).
//! The alternate data-crystal monster does not register this quest event.

/// One source-declared, damaging-player, explicit-storage transition. It is not
/// an ordinary XP principal or a party-sharing rule.
#[derive(Debug, Clone, Copy)]
pub struct Callback {
    pub creature: &'static str,
    pub transition: &'static str,
    pub track: &'static str,
    pub expected: i64,
}

pub const CALLBACKS: &[Callback] = &[Callback {
    creature: "oteryn:creature.renegade_orc",
    transition: "oteryn:quest-transition/quest/u8_54/an_uneasy_alliance/quest_door/creature_event_1",
    track: "oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/quest_door",
    expected: 0,
}];

#[must_use]
pub fn tracks_creature(creature: &str) -> bool {
    CALLBACKS
        .iter()
        .any(|callback| callback.creature == creature)
}
