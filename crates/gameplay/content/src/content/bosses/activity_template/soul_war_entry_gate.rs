use super::PinnedActivitySourceRoom;

pub(super) fn callbacks_available(source: PinnedActivitySourceRoom) -> bool {
    // Greed remains source-only until the Sphere health conflict is settled:
    // compiled15, both pinned donors10000, retained wiki15.000 (15000).
    // This gate changes no HP and does not qualify an invented balance rule.
    matches!(source, PinnedActivitySourceRoom::GoshnarsMalice)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_greed_entry_refuses_unsettled_soul_sphere_health_without_changing_other_hooks() {
        assert!(
            !callbacks_available(PinnedActivitySourceRoom::GoshnarsGreed),
            "canonical Sphere15 versus donor10000 and retained wiki15.000 is not a qualified production health rule"
        );
        assert!(callbacks_available(
            PinnedActivitySourceRoom::GoshnarsMalice
        ));
        for source in [
            PinnedActivitySourceRoom::GoshnarsSpite,
            PinnedActivitySourceRoom::GoshnarsHatred,
            PinnedActivitySourceRoom::GoshnarsCruelty,
            PinnedActivitySourceRoom::GoshnarsMegalomania,
        ] {
            assert!(
                !callbacks_available(source),
                "unfinished hook qualification stays unchanged"
            );
        }
    }
}
