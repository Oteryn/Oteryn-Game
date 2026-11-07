//! The settled-kill seam gives full points to each eligible contributor, once per death.
use super::super::weapon_proficiency::{
    LiveProficiencyTrack, ProficiencyAttackEffects, WeaponProficiencyDispatch,
};
use super::*;
use crate::domain::weapon_proficiency::{ProficiencySelectionShape, ProficiencyThresholdClass};

fn owner() -> WeaponProficiencyDispatch {
    let mut owner = WeaponProficiencyDispatch::default();
    owner.install_track(
        "weapon",
        LiveProficiencyTrack {
            shape: ProficiencySelectionShape::new(ProficiencyThresholdClass::Standard, &[1; 7])
                .expect("shape"),
            progress: 0,
            selections: vec![None; 7],
            modifications: [None, None],
            effects: ProficiencyAttackEffects::default(),
        },
    );
    owner
}

#[test]
fn proficiency_kill_credit_full_points_window_replay_and_checkpoint_decision() {
    let mut first = owner();
    let mut second = owner();
    let mut expired = owner();
    for owner in [&mut first, &mut second] {
        assert_eq!(
            credit_proficiency(owner, [1; 16], 400_000, Some(100), true, 100_000, "weapon"),
            Some(100)
        );
        assert_eq!(
            credit_proficiency(owner, [1; 16], 400_000, Some(100), true, 100_000, "weapon"),
            None
        );
        assert!(!owner.checkpoint_due(459_999_999, false));
        assert!(owner.checkpoint_due(460_000_000, false));
        assert!(owner.checkpoint_due(400_000_000, true));
    }
    assert_eq!(
        credit_proficiency(
            &mut expired,
            [1; 16],
            400_000,
            Some(100),
            true,
            99_999,
            "weapon"
        ),
        None
    );
    assert!(!expired.checkpoint_due(u64::MAX, true));
    assert_eq!(
        credit_proficiency(
            &mut expired,
            [2; 16],
            400_000,
            Some(100),
            false,
            399_999,
            "weapon"
        ),
        None
    );
    assert_eq!(
        credit_proficiency(
            &mut expired,
            [3; 16],
            400_000,
            None,
            true,
            399_999,
            "weapon"
        ),
        None
    );
    // Cross a proficiency threshold: the next save is due immediately, before cadence.
    assert_eq!(
        credit_proficiency(
            &mut first,
            [4; 16],
            400_001,
            Some(1650),
            true,
            400_000,
            "weapon"
        ),
        Some(1750)
    );
    assert!(first.checkpoint_due(400_001_000, false));
}

#[test]
fn daily_proficiency_bonus_credits_exact_chance_and_never_replays() {
    for (streak, chance) in [(0, 0), (1, 1), (7, 7), (14, 8), (21, 9), (28, 10), (60, 10)] {
        let mut doubled = 0;
        for roll in 0..100 {
            let points = daily_proficiency_points(Some(100), streak, roll);
            assert_eq!(points, Some(if roll < chance { 200 } else { 100 }));
            let mut owner = owner();
            assert_eq!(
                credit_proficiency(
                    &mut owner, [1; 16], 400_000, points, true, 100_000, "weapon"
                ),
                points
            );
            assert_eq!(
                credit_proficiency(
                    &mut owner, [1; 16], 400_000, points, true, 100_000, "weapon"
                ),
                None
            );
            doubled += u8::from(points == Some(200));
        }
        assert_eq!(doubled, chance);
    }
    assert_eq!(daily_proficiency_points(None, 28, 0), None);
    assert_eq!(daily_proficiency_points(Some(u32::MAX), 28, 0), None);
}
