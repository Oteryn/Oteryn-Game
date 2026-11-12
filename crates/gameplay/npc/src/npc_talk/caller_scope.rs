//! Historic source-only conversations require their explicit native caller.
pub(super) fn allows(key: &str, native_quest: bool) -> bool {
    native_quest
        || !matches!(
            key,
            "oteryn:npc.spectulus" | "oteryn:npc.palimuth" | "oteryn:npc.alesar"
        )
}
