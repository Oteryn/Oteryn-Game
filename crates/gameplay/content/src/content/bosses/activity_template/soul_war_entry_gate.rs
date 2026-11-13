use super::PinnedActivitySourceRoom;

pub(super) fn callbacks_available(source: PinnedActivitySourceRoom) -> bool {
    // Owner ANSWER6839182665 selects effective Sphere15000 from retained Wiki
    // evidence. The decoder still requires the exact raw witness and the
    // template independently requires its qualified effective health policy.
    matches!(
        source,
        PinnedActivitySourceRoom::GoshnarsMalice
            | PinnedActivitySourceRoom::GoshnarsGreed
            | PinnedActivitySourceRoom::GoshnarsSpite
            | PinnedActivitySourceRoom::GoshnarsHatred
            | PinnedActivitySourceRoom::GoshnarsCruelty
            | PinnedActivitySourceRoom::GoshnarsMegalomania
    )
}
