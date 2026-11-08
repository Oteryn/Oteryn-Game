//! The pinned donor's three Night Harpy corpse stages, not a beak possession shortcut.
use super::{GatheringTarget, GatheringYield, gathering_rule};

#[test]
fn m41_stag_beak_skinning_uses_exact_tool_corpse_and_donor_yield() {
    for corpse in [52678, 52679, 52680] {
        assert_eq!(
            gathering_rule(5908, corpse),
            Some((
                GatheringTarget::Corpse { spent_item: 52677 },
                GatheringYield {
                    item: 52632,
                    threshold: 10_000
                },
            )),
            "the pinned donor provides this row; its probability is not an official-stat claim"
        );
        for other_tool in [5942, 3483, 9306] {
            assert!(gathering_rule(other_tool, corpse).is_none());
        }
    }
    assert!(
        gathering_rule(5908, 52677).is_none(),
        "spent corpse cannot yield twice"
    );
}
