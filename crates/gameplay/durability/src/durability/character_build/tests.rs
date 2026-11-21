#[cfg(test)]
mod tests {
    use super::{
        BuildCause, BuildFormula, DurableBuildState, MAX_BUILD_PROGRESS, MIN_SKILL_LEVEL,
        convert_vocation, cumulative_progress, normalized, relevel, skill_tries_required,
    };

    /// Skills: base 50, multiplier 2.0 without a vocation and 1.1 with one. Magic: 100 x L.
    struct Table;

    impl BuildFormula for Table {
        fn required(&self, vocation: &str, family: usize, level: u16) -> Option<u64> {
            match (family, vocation) {
                (0, _) => Some(100 * u64::from(level)),
                (_, "none") => skill_tries_required(50, 2.0, level),
                _ => skill_tries_required(50, 1.1, level),
            }
        }

        fn digest(&self) -> [u8; 32] {
            [1; 32]
        }

        fn content_revision(&self) -> &str {
            "content-1"
        }
    }

    #[test]
    fn vocation_choice_keeps_cumulative_progress_under_the_new_multipliers() {
        let mut skills = [(10, 0); 7];
        // Sword 50 + 100 + 3 = 153 tries without a vocation.
        skills[2] = (12, 3);
        let before = DurableBuildState::new("none", (2, 40), skills).unwrap_or_default();
        let after = convert_vocation(&Table, &before, "knight").unwrap_or_default();
        // As a knight 153 = 50 + 55 + 48, below req(13) = 60; magic keeps (2, 40).
        assert_eq!(after.vocation(), "knight");
        assert_eq!(after.skills()[2], (12, 48));
        assert_eq!(after.magic(), (2, 40));
        assert!(normalized(&Table, &after));
        skills[2] = (10, 50);
        let unpaid = DurableBuildState::new("knight", (0, 0), skills).unwrap_or_default();
        assert!(!normalized(&Table, &unpaid), "50 tries pay for level 11");
        skills[2] = (1000, 0);
        let unreachable = DurableBuildState::new("none", (0, 0), skills).unwrap_or_default();
        assert!(!normalized(&Table, &unreachable), "req(69) is unreachable");
    }

    #[test]
    fn req_truncates_the_f64_power_and_stops_below_the_bound() {
        let sword = |level| skill_tries_required(50, 1.1, level);
        assert_eq!(sword(10), None);
        assert_eq!(sword(11), Some(50));
        assert_eq!(sword(12), Some(55));
        // 50 x 1.21 = 60.500000000000014 truncates to 60.
        assert_eq!(sword(13), Some(60));
        assert_eq!(sword(1001), None);
        // With 2.0, level 68 is the last reachable one (SKILLS-0 §3.1 "around level 67").
        assert_eq!(skill_tries_required(50, 2.0, 68), Some(50 << 57));
        assert_eq!(skill_tries_required(50, 2.0, 69), None);
        assert_eq!(skill_tries_required(50, f64::NAN, 12), None);
        assert_eq!(skill_tries_required(50, 0.0, 12), None);
    }

    #[test]
    fn cumulative_sums_saturate_and_relevel_round_trips() {
        let sword = |level| skill_tries_required(50, 1.1, level);
        assert_eq!(cumulative_progress(12, 3, MIN_SKILL_LEVEL, sword), 108);
        assert_eq!(relevel(108, MIN_SKILL_LEVEL, sword), (12, 3));
        assert_eq!(cumulative_progress(10, 7, MIN_SKILL_LEVEL, sword), 7);
        let doubling = |level| skill_tries_required(50, 2.0, level);
        // Levels 11..=68 total 50 x (2^58 - 1), above the bound: the sum saturates.
        assert_eq!(
            cumulative_progress(68, MAX_BUILD_PROGRESS, MIN_SKILL_LEVEL, doubling),
            MAX_BUILD_PROGRESS
        );
        assert_eq!(
            cumulative_progress(69, 0, MIN_SKILL_LEVEL, doubling),
            MAX_BUILD_PROGRESS
        );
        // The bound re-levels to 67 (levels 11..=67 total 50 x (2^57 - 1)).
        assert_eq!(
            relevel(MAX_BUILD_PROGRESS, MIN_SKILL_LEVEL, doubling),
            (67, MAX_BUILD_PROGRESS - 50 * ((1 << 57) - 1))
        );
        assert_eq!(
            relevel(u64::MAX, 0, |_| Some(1)),
            (1000, MAX_BUILD_PROGRESS - 1000)
        );
    }

    #[test]
    fn s16c_vocation_trial_switches_between_vocations_and_back_but_never_to_none() {
        let seed = DurableBuildState::default();
        let knight = DurableBuildState::new("knight", (0, 0), [(10, 0); 7]).unwrap_or_default();
        let sorcerer = DurableBuildState::new("sorcerer", (0, 0), [(10, 0); 7]).unwrap_or_default();
        assert!(BuildCause::VocationTrial.admits(&seed, &knight));
        assert!(BuildCause::VocationTrial.admits(&knight, &sorcerer));
        assert!(BuildCause::VocationTrial.admits(&sorcerer, &knight));
        assert!(!BuildCause::VocationTrial.admits(&knight, &knight));
        assert!(!BuildCause::VocationTrial.admits(&knight, &seed));
        assert!(BuildCause::VocationTrial.converts());
        assert_eq!(
            BuildCause::parse("vocation_trial"),
            Some(BuildCause::VocationTrial)
        );
    }

    #[test]
    fn state_bounds_and_cause_directions() {
        let seed = DurableBuildState::default();
        assert!(DurableBuildState::new("knight", (0, 0), [(9, 0); 7]).is_err());
        assert!(DurableBuildState::new("knight", (1001, 0), [(10, 0); 7]).is_err());
        assert!(
            DurableBuildState::new("knight", (0, MAX_BUILD_PROGRESS + 1), [(10, 0); 7]).is_err()
        );
        assert!(DurableBuildState::new("", (0, 0), [(10, 0); 7]).is_err());
        let knight = DurableBuildState::new("knight", (0, 0), [(10, 0); 7]).unwrap_or_default();
        let trained = DurableBuildState::new("knight", (1, 0), [(10, 0); 7]).unwrap_or_default();
        let elite =
            DurableBuildState::new("elite_knight", (1, 0), [(10, 0); 7]).unwrap_or_default();
        assert!(BuildCause::VocationChoice.admits(&seed, &knight));
        assert!(!BuildCause::VocationChoice.admits(&knight, &elite));
        assert!(BuildCause::Training.admits(&knight, &trained));
        assert!(!BuildCause::RestingStamina.admits(&knight, &knight));
        assert!(!BuildCause::RestingStamina.admits(&knight, &trained));
        assert!(!BuildCause::Training.admits(&trained, &knight));
        assert!(!BuildCause::Training.admits(&knight, &knight));
        assert!(BuildCause::Promotion.admits(&trained, &elite));
        assert!(!BuildCause::Promotion.admits(&knight, &elite));
        assert_eq!(BuildCause::parse("promotion"), Some(BuildCause::Promotion));
        assert_eq!(
            BuildCause::parse("resting_stamina"),
            Some(BuildCause::RestingStamina)
        );
        assert_eq!(BuildCause::parse("death"), None);
    }
}
