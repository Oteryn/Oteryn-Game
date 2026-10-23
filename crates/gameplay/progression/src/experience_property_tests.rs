#![allow(clippy::expect_used)]
use oteryn_gameplay_content::content::character_progression_content::CharacterProgressionContent;
use oteryn_gameplay_domain::domain::progression::{
    CurrentProgressionSnapshot, ProgressionOperation, calculate_progression,
};
use oteryn_simulation_determinism::ExactI64;
use oteryn_test_support::property::for_each_case;

#[test]
fn property_experience_table_is_monotone_and_inverts_at_every_level() {
    let content = CharacterProgressionContent::decode(include_bytes!(
        "../../../../rulesets/character/experience/native-section.json"
    ))
    .expect("admitted progression section");
    let policy = content
        .policy_for("profile", "ruleset", "content")
        .expect("finite policy");
    assert_eq!(policy.thresholds[0].minimum_experience.get(), 0);
    for pair in policy.thresholds.windows(2) {
        assert_eq!(pair[0].level + 1, pair[1].level);
        assert!(pair[0].minimum_experience.get() < pair[1].minimum_experience.get());
    }
    // The complete admitted table is small: cover every inverse, not only the
    // random sample. Award from level 1 through the production calculation.
    let initial = CurrentProgressionSnapshot {
        level: 1,
        total_experience: ExactI64::new(0),
        skill_progression: (),
        magic_progression: (),
    };
    for entry in &policy.thresholds {
        // A zero award is intentionally invalid. XP 1 also belongs to level 1.
        let amount = ExactI64::new(entry.minimum_experience.get().max(1));
        let result = calculate_progression(
            &initial,
            &policy.context,
            &policy.policy_revision,
            &ProgressionOperation::AwardExperience {
                source_occurrence: (),
                reward_revision: policy.reward_revision.clone(),
                amount,
            },
            &policy,
        )
        .expect("table inverse");
        assert_eq!(result.level_after, entry.level);
    }
    for_each_case("experience threshold intervals", |rng, _| {
        let index = rng.below(policy.thresholds.len() as u64) as usize;
        let entry = &policy.thresholds[index];
        let end = policy
            .thresholds
            .get(index + 1)
            .map_or(policy.terminal_exclusive_experience.get(), |next| {
                next.minimum_experience.get()
            });
        let xp = rng.range_i64(entry.minimum_experience.get().max(1), end - 1);
        let award = |amount| ProgressionOperation::AwardExperience {
            source_occurrence: (),
            reward_revision: policy.reward_revision.clone(),
            amount: ExactI64::new(amount),
        };
        let result = calculate_progression(
            &initial,
            &policy.context,
            &policy.policy_revision,
            &award(xp),
            &policy,
        )
        .expect("within table");
        assert_eq!(result.level_after, entry.level);
        let last = policy.thresholds.last().expect("nonempty");
        let terminal = CurrentProgressionSnapshot {
            level: last.level,
            total_experience: ExactI64::new(policy.terminal_exclusive_experience.get() - 1),
            skill_progression: (),
            magic_progression: (),
        };
        for amount in [1, i64::MAX] {
            assert!(
                calculate_progression(
                    &terminal,
                    &policy.context,
                    &policy.policy_revision,
                    &award(amount),
                    &policy
                )
                .is_err()
            );
        }
        Ok(())
    });
}
