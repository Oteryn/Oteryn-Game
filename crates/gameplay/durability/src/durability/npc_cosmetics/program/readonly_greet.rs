use super::source;
// Closed registered source callbacks with no gameplay mutation.
pub(super) fn qualified(npc: &str, source: &str) -> bool {
    matches!(
        (npc, source),
        (
            "oteryn:npc.vescu",
            "oteryn:npc-cosmetic/vescu/assassin-returner-greet"
                | "oteryn:npc-cosmetic/vescu/assassin-drunken-beginner-greet"
                | "oteryn:npc-cosmetic/vescu/assassin-sober-refusal-greet"
        ) | (
            "oteryn:npc.bron",
            "oteryn:npc-cosmetic/bron/barbarian-remorse-greet"
                | "oteryn:npc-cosmetic/bron/barbarian-default-greet"
        ) | (
            "oteryn:npc.a_majestic_warwolf",
            "oteryn:npc-cosmetic/a_majestic_warwolf/druid-unqualified-refusal-greet"
                | "oteryn:npc-cosmetic/a_majestic_warwolf/druid-qualified-greet"
        ) | (
            "oteryn:npc.bozo",
            "oteryn:npc-cosmetic/bozo/jester-male-greet"
                | "oteryn:npc-cosmetic/bozo/jester-female-greet"
        )
    )
}

/// Dispatch routing hint only; Root independently chooses and qualifies the source.
pub(super) fn has_registered(npc: &str) -> bool {
    source::embedded()
        .ok()
        .and_then(|program| program.npc(npc))
        .is_some_and(|npc| {
            npc.transitions
                .iter()
                .any(|t| t.trigger == source::Trigger::Greet && qualified(&npc.npc, &t.source_key))
        })
}
