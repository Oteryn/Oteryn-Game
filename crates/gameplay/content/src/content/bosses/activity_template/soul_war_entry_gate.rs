use super::PinnedActivitySourceRoom;

pub(super) fn callbacks_available(source: PinnedActivitySourceRoom) -> bool {
    // Greed remains source-only until the Sphere health conflict is settled:
    // compiled15, both pinned donors10000, retained wiki15.000 (15000).
    // This gate changes no HP and does not qualify an invented balance rule.
    matches!(
        source,
        PinnedActivitySourceRoom::GoshnarsMalice
            | PinnedActivitySourceRoom::GoshnarsSpite
            | PinnedActivitySourceRoom::GoshnarsHatred
            | PinnedActivitySourceRoom::GoshnarsCruelty
            | PinnedActivitySourceRoom::GoshnarsMegalomania
    )
}
