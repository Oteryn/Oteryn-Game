#![allow(clippy::expect_used)]
use super::*;
use crate::quest::QuestRefusal;

fn apply(
    catalogue: &QuestStateCatalogue,
    tracks: &mut BTreeMap<String, i64>,
    key: &str,
) -> Result<(), QuestRefusal> {
    let changes = catalogue.evaluate(catalogue.transition(key).expect("transition"), tracks, 0)?;
    for change in changes {
        tracks.insert(change.track, change.after);
    }
    Ok(())
}

#[test]
fn hazard_selection_and_qualification_are_separate_and_bounded() {
    let base = QuestStateCatalogue::empty("content-1").expect("base");
    let catalogue = extend(&base).expect("hazard");
    let mut tracks = BTreeMap::new();
    assert_eq!(
        level(&tracks),
        Some(HazardLevel {
            current: 1,
            unlocked: 1
        })
    );
    assert_eq!(
        apply(&catalogue, &mut tracks, &select_transition(2)),
        Err(QuestRefusal::StageMismatch)
    );
    for current in 1..MAX_LEVEL {
        apply(&catalogue, &mut tracks, &advance_transition(current)).expect("qualification");
        assert_eq!(
            level(&tracks),
            Some(HazardLevel {
                current,
                unlocked: current + 1
            })
        );
        assert_eq!(
            apply(&catalogue, &mut tracks, &advance_transition(current)),
            Err(QuestRefusal::StageMismatch)
        );
        apply(&catalogue, &mut tracks, &select_transition(current + 1)).expect("select");
    }
    apply(&catalogue, &mut tracks, &select_transition(1)).expect("lower selection");
    assert_eq!(
        level(&tracks),
        Some(HazardLevel {
            current: 1,
            unlocked: MAX_LEVEL
        })
    );
    assert!(
        catalogue
            .transition(&advance_transition(MAX_LEVEL))
            .is_none()
    );
    assert!(catalogue.transition(&select_transition(0)).is_none());
    assert!(
        catalogue
            .transition(&select_transition(MAX_LEVEL + 1))
            .is_none()
    );
    for value in [-1, 0, 13, i64::MAX] {
        assert_eq!(level(&BTreeMap::from([(UNLOCKED.into(), value)])), None);
    }
    assert_eq!(level(&BTreeMap::from([(CURRENT.into(), 2)])), None);
}

#[test]
fn hazard_area_includes_edges_and_subtracts_both_magma_bubble_rectangles() {
    for z in 13..=15 {
        for (x, y) in [(33502, 32740), (33796, 32996), (33537, 32874)] {
            assert!(contains(x, y, z), "{x}/{y}/{z}");
        }
    }
    for (x, y, z) in [
        (33501, 32740, 13),
        (33797, 32996, 15),
        (33502, 32739, 14),
        (33796, 32997, 14),
        (33537, 32874, 12),
        (33537, 32874, 16),
        (33537, 32874, -14),
    ] {
        assert!(!contains(x, y, z));
    }
    for (x, y) in [
        (33633, 32915),
        (33649, 32928),
        (33630, 32887),
        (33672, 32921),
        (33640, 32920),
    ] {
        assert!(!contains(x, y, 15));
        assert!(contains(x, y, 14));
    }
    assert!(contains(33650, 32928, 15));
    assert!(contains(33629, 32887, 15));
}
