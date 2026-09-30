//! Pure, persistence-neutral Bestiary kill progress (CHARM-2).
//!
//! The rules live in `rulesets/progression/bestiary/kill-progress.json`
//! (CHARM-0 §4.1, owner answers 2a and 6a):
//!
//! - a race is the Creature definition key of a definition that carries a
//!   Bestiary block; its `kill_thresholds` come from that definition;
//! - only `kill_count` is stored; unlocked stages are derived from it and the
//!   thresholds, never stored;
//! - the counter saturates at the last threshold;
//! - a kill is credited when the character damaged the creature at most
//!   [`BESTIARY_KILL_CREDIT_WINDOW_MS`] before its death.

/// CHARM-0 answer 6a: five minutes, in owner-clock milliseconds.
pub const BESTIARY_KILL_CREDIT_WINDOW_MS: u64 = 300_000;
/// Storage bound on thresholds per race, not a game rule. Every current
/// Bestiary race has three.
pub const BESTIARY_KILL_THRESHOLDS_MAX: usize = 8;
const MAX_KEY_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BestiaryError {
    /// The race key or definition revision is not a bounded key.
    InvalidRaceIdentity,
    /// The thresholds are empty, too many, zero, or not strictly increasing.
    InvalidKillThresholds,
    /// The credit evidence places the character's damage after the death.
    InvalidCreditEvidence,
}

impl std::fmt::Display for BestiaryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRaceIdentity => "invalid Bestiary race identity",
            Self::InvalidKillThresholds => "invalid Bestiary kill thresholds",
            Self::InvalidCreditEvidence => "Bestiary credit evidence is after the death",
        })
    }
}

impl std::error::Error for BestiaryError {}

/// One Bestiary race, as bound from its Creature definition at one
/// definition revision. Construction validates every field, so a value of
/// this type always has at least one threshold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BestiaryRace {
    key: String,
    definition_revision: String,
    kill_thresholds: Vec<u32>,
}

impl BestiaryRace {
    pub fn new(
        key: impl Into<String>,
        definition_revision: impl Into<String>,
        kill_thresholds: Vec<u32>,
    ) -> Result<Self, BestiaryError> {
        let key = key.into();
        let definition_revision = definition_revision.into();
        if !valid_key(&key) || !valid_key(&definition_revision) {
            return Err(BestiaryError::InvalidRaceIdentity);
        }
        if kill_thresholds.is_empty()
            || kill_thresholds.len() > BESTIARY_KILL_THRESHOLDS_MAX
            || kill_thresholds[0] == 0
            || kill_thresholds.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(BestiaryError::InvalidKillThresholds);
        }
        Ok(Self {
            key,
            definition_revision,
            kill_thresholds,
        })
    }

    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    #[must_use]
    pub fn definition_revision(&self) -> &str {
        &self.definition_revision
    }

    #[must_use]
    pub fn kill_thresholds(&self) -> &[u32] {
        &self.kill_thresholds
    }

    /// The saturation bound: the last threshold.
    #[must_use]
    pub fn final_kill_threshold(&self) -> u32 {
        // Construction guarantees at least one threshold.
        self.kill_thresholds.last().copied().unwrap_or(0)
    }

    /// The count after one more credited kill, or `None` when `kill_count`
    /// is already at (or, after a threshold change, above) the bound.
    #[must_use]
    pub fn next_kill_count(&self, kill_count: u32) -> Option<u32> {
        if kill_count >= self.final_kill_threshold() {
            return None;
        }
        kill_count.checked_add(1)
    }

    /// Derived, never stored: how many thresholds `kill_count` has reached.
    #[must_use]
    pub fn stages_unlocked(&self, kill_count: u32) -> usize {
        self.kill_thresholds
            .iter()
            .filter(|threshold| **threshold <= kill_count)
            .count()
    }

    /// Derived, never stored: the entry is complete at the last threshold.
    #[must_use]
    pub fn is_complete(&self, kill_count: u32) -> bool {
        kill_count >= self.final_kill_threshold()
    }
}

/// When the reward principal last damaged the creature and when the creature
/// died, both on the same owner clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BestiaryKillCredit {
    pub principal_last_damage_at_ms: u64,
    pub death_at_ms: u64,
}

impl BestiaryKillCredit {
    /// CHARM-0 answer 6a. Damage exactly at the window's start still counts.
    /// Damage after the death is contradictory evidence and fails closed.
    pub fn is_credited(&self) -> Result<bool, BestiaryError> {
        let elapsed = self
            .death_at_ms
            .checked_sub(self.principal_last_damage_at_ms)
            .ok_or(BestiaryError::InvalidCreditEvidence)?;
        Ok(elapsed <= BESTIARY_KILL_CREDIT_WINDOW_MS)
    }
}

fn valid_key(value: &str) -> bool {
    let mut bytes = value.bytes();
    value.len() <= MAX_KEY_BYTES
        && bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn rat() -> BestiaryRace {
        BestiaryRace::new("oteryn:creature.rat", "definition-r1", vec![5, 50, 500]).expect("rat")
    }

    #[test]
    fn ruleset_file_matches_the_implemented_rules() {
        let ruleset: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../rulesets/progression/bestiary/kill-progress.json"
        ))
        .expect("ruleset json");
        assert_eq!(
            ruleset["schema"].as_str(),
            Some("OTERYN_GAME_BESTIARY_KILL_PROGRESS/v1")
        );
        assert_eq!(
            ruleset["credit"]["window_ms"].as_u64(),
            Some(BESTIARY_KILL_CREDIT_WINDOW_MS)
        );
        assert_eq!(ruleset["counter"]["increment"].as_u64(), Some(1));
        assert_eq!(ruleset["stages"]["stored"].as_bool(), Some(false));
    }

    #[test]
    fn race_requires_a_bounded_identity_and_strictly_increasing_positive_thresholds() {
        assert_eq!(rat().final_kill_threshold(), 500);
        for (key, revision) in [
            ("", "definition-r1"),
            ("oteryn:creature.rat", ""),
            ("-rat", "definition-r1"),
            ("oteryn:creature rat", "definition-r1"),
        ] {
            assert_eq!(
                BestiaryRace::new(key, revision, vec![5]),
                Err(BestiaryError::InvalidRaceIdentity)
            );
        }
        assert_eq!(
            BestiaryRace::new("a".repeat(129), "definition-r1", vec![5]),
            Err(BestiaryError::InvalidRaceIdentity)
        );
        assert!(BestiaryRace::new("a".repeat(128), "definition-r1", vec![5]).is_ok());
        for thresholds in [
            vec![],
            vec![0, 5],
            vec![5, 5],
            vec![50, 5],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
        ] {
            assert_eq!(
                BestiaryRace::new("oteryn:creature.rat", "definition-r1", thresholds),
                Err(BestiaryError::InvalidKillThresholds)
            );
        }
        assert!(
            BestiaryRace::new(
                "oteryn:creature.rat",
                "definition-r1",
                vec![1, 2, 3, 4, 5, 6, 7, 8]
            )
            .is_ok()
        );
    }

    #[test]
    fn counter_saturates_at_the_final_threshold() {
        let race = rat();
        assert_eq!(race.next_kill_count(0), Some(1));
        assert_eq!(race.next_kill_count(499), Some(500));
        assert_eq!(race.next_kill_count(500), None);
        // A later definition revision with a lower bound keeps the stored
        // count and counts no further.
        assert_eq!(race.next_kill_count(u32::MAX), None);
    }

    #[test]
    fn stages_are_derived_from_the_count_and_thresholds() {
        let race = rat();
        assert_eq!(race.stages_unlocked(0), 0);
        assert_eq!(race.stages_unlocked(4), 0);
        assert_eq!(race.stages_unlocked(5), 1);
        assert_eq!(race.stages_unlocked(49), 1);
        assert_eq!(race.stages_unlocked(50), 2);
        assert_eq!(race.stages_unlocked(499), 2);
        assert!(!race.is_complete(499));
        assert_eq!(race.stages_unlocked(500), 3);
        assert!(race.is_complete(500));
        assert_eq!(race.stages_unlocked(900), 3);
    }

    #[test]
    fn credit_window_is_five_minutes_inclusive_and_fails_closed_on_future_damage() {
        let credit = |last, death| BestiaryKillCredit {
            principal_last_damage_at_ms: last,
            death_at_ms: death,
        };
        assert_eq!(credit(1_000, 1_000).is_credited(), Ok(true));
        assert_eq!(credit(1_000, 301_000).is_credited(), Ok(true));
        assert_eq!(credit(1_000, 301_001).is_credited(), Ok(false));
        assert_eq!(credit(0, u64::MAX).is_credited(), Ok(false));
        assert_eq!(
            credit(1_001, 1_000).is_credited(),
            Err(BestiaryError::InvalidCreditEvidence)
        );
    }
}
