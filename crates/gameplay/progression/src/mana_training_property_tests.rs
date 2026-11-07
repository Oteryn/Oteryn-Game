#![allow(clippy::expect_used)]
use crate::mana_training::advance;
use oteryn_gameplay_content::content::training_formula::CompiledTrainingFormula;
use oteryn_gameplay_domain::domain::character_build::{
    BuildFormula, MAX_BUILD_LEVEL, MAX_BUILD_PROGRESS,
};
use oteryn_test_support::property::for_each_case;

#[test]
fn property_training_is_monotone_for_all_admitted_vocations_and_saturates_at_max_level() {
    let formula = CompiledTrainingFormula::from_profile(
        include_bytes!("../../../../tools/content-schema/native-gameplay/build-training.json"),
        "content-r1",
    )
    .expect("training profile");
    for vocation in &formula.profile.vocations {
        for family in 0..=7 {
            let first = if family == 0 { 1 } else { 11 };
            let mut previous = Some(0);
            for level in first..=MAX_BUILD_LEVEL {
                let next = formula.required(&vocation.key, family, level);
                match (previous, next) {
                    (Some(before), Some(after)) => {
                        assert!(after >= before && after <= MAX_BUILD_PROGRESS)
                    }
                    (None, _) => assert_eq!(
                        next, None,
                        "unreachable requirement became reachable: {} family {family} level {level}",
                        vocation.key
                    ),
                    _ => {}
                }
                previous = next;
            }
        }
    }
    for_each_case("training paid progress", |rng, _| {
        let vocation = &rng.pick(&formula.profile.vocations).key;
        let family = rng.below(8) as usize;
        let floor = if family == 0 { 0 } else { 10 };
        let level = rng.range_i64(floor, i64::from(MAX_BUILD_LEVEL)) as u16;
        let stored = rng.below(100_000);
        let pending = rng.below(100_000);
        let before = advance(&formula, vocation, family, (level, stored), pending);
        let after = advance(&formula, vocation, family, (level, stored), pending + 1);
        assert!(before <= after);
        assert!((level..=MAX_BUILD_LEVEL).contains(&before.0));
        assert!(before.1 <= MAX_BUILD_PROGRESS);
        assert!(
            formula
                .required(vocation, family, before.0.saturating_add(1))
                .is_none_or(|required| before.0 == MAX_BUILD_LEVEL || before.1 < required)
        );
        for pending in [0, 1, MAX_BUILD_PROGRESS, u64::MAX] {
            assert_eq!(
                advance(
                    &formula,
                    vocation,
                    family,
                    (MAX_BUILD_LEVEL, stored),
                    pending
                ),
                (
                    MAX_BUILD_LEVEL,
                    stored.saturating_add(pending).min(MAX_BUILD_PROGRESS)
                )
            );
        }
        Ok(())
    });
}
