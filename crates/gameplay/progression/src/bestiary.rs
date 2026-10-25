//! Pure Bestiary threshold and Charm stage derivation.
use oteryn_gameplay_domain::domain::{bestiary::BestiaryRace, charm::BestiaryStage};

/// The completed stage of an entry: the final stage at the last threshold, otherwise the
/// thresholds reached, capped below the final stage.
pub fn bestiary_stage(race: &BestiaryRace, kill_count: u32) -> BestiaryStage {
    if race.is_complete(kill_count) {
        return BestiaryStage::FINAL;
    }
    let reached = u8::try_from(race.stages_unlocked(kill_count)).unwrap_or(u8::MAX);
    BestiaryStage::new(reached.min(BestiaryStage::FINAL.get() - 1)).unwrap_or(BestiaryStage::NONE)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn bestiary_stage_is_final_only_at_the_last_threshold() {
        let race = BestiaryRace::new("oteryn:creature.rat", "definition-r1", vec![25, 250, 500])
            .expect("race");
        for (kills, stage) in [
            (0, 0),
            (24, 0),
            (25, 1),
            (249, 1),
            (250, 2),
            (499, 2),
            (500, 3),
        ] {
            assert_eq!(bestiary_stage(&race, kills).get(), stage, "{kills} kills");
        }
        // A one-threshold entry is complete or nothing.
        let single =
            BestiaryRace::new("oteryn:creature.boss", "definition-r1", vec![5]).expect("race");
        assert_eq!(bestiary_stage(&single, 4), BestiaryStage::NONE);
        assert_eq!(bestiary_stage(&single, 5), BestiaryStage::FINAL);
    }
}
