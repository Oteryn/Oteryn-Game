use super::character_build::{
    MAX_BUILD_LEVEL, MAX_BUILD_PROGRESS, cumulative_progress, relevel, skill_tries_required,
};
use oteryn_test_support::property::for_each_case;

#[test]
fn property_skill_requirements_are_monotone_until_unreachable() {
    for_each_case("skill requirements", |rng, _| {
        let base = 1 + rng.below(u64::from(u16::MAX)) as u16;
        let multiplier = *rng.pick(&[1.0, 1.1, 1.4, 1.5, 2.0]);
        let mut previous = Some(0);
        for level in 11..=MAX_BUILD_LEVEL {
            let required = skill_tries_required(base, multiplier, level);
            match (previous, required) {
                (Some(before), Some(after)) => {
                    assert!(before <= after && after <= MAX_BUILD_PROGRESS)
                }
                (None, Some(_)) => {
                    return Err(format!("unreachable skill became reachable at {level}"));
                }
                _ => {}
            }
            previous = required;
        }
        for invalid in [0, 10, MAX_BUILD_LEVEL + 1, u16::MAX] {
            assert_eq!(skill_tries_required(base, multiplier, invalid), None);
        }
        for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(skill_tries_required(base, invalid, 12), None);
        }
        Ok(())
    });
}

#[test]
fn property_skill_relevel_conserves_progress_and_caps_at_maximum() {
    for_each_case("skill progress conservation", |rng, _| {
        let base = 1 + rng.below(100) as u16;
        let req = |level| skill_tries_required(base, 1.1, level);
        for progress in [0, rng.u64(), MAX_BUILD_PROGRESS, u64::MAX] {
            let (level, rest) = relevel(progress, 10, req);
            assert!((10..=MAX_BUILD_LEVEL).contains(&level));
            assert!(rest <= MAX_BUILD_PROGRESS);
            assert_eq!(
                cumulative_progress(level, rest, 10, req),
                progress.min(MAX_BUILD_PROGRESS)
            );
            if level < MAX_BUILD_LEVEL {
                assert!(req(level + 1).is_none_or(|required| rest < required));
            }
            assert_eq!(
                relevel(progress, MAX_BUILD_LEVEL, req),
                (MAX_BUILD_LEVEL, progress.min(MAX_BUILD_PROGRESS))
            );
        }
        assert_eq!(
            cumulative_progress(MAX_BUILD_LEVEL, u64::MAX, 10, req),
            MAX_BUILD_PROGRESS
        );
        Ok(())
    });
}
