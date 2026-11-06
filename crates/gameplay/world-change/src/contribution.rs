//! WCONTRIB: shared contribution transitions; durable owners supply committed tallies.

use serde::{Deserialize, Serialize};

pub const COAL_UNLOCK: u64 = 200;
/// Owner-authorized conservative assumption: 100 pouches cure the next saved day.
/// The retained donor defines the pouch and individual exchange, but no shared threshold.
pub const MEDICINE_CURE: u64 = 100;
pub const CORPSE_POLLUTION: u64 = 1000;
pub const PERSONAL_CATCH_INTERVAL_MS: i64 = 20 * 60 * 60 * 1000;
pub const HORSE_RAID_INTERVAL_MS: i64 = 3 * 60 * 60 * 1000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LakeState {
    #[default]
    Clean,
    Dirty,
    Stinking,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaveContributions {
    pub coal: u64,
    pub medicine: u64,
    pub corpses: u64,
    pub winning_ballot: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SharedChanges {
    pub steamship_open: bool,
    pub swamp_epidemic: bool,
    pub lake_state: LakeState,
    pub elected_candidate: Option<String>,
    pub horse: HorseStationState,
    pub noodles: NoodlesState,
}

impl Default for SharedChanges {
    fn default() -> Self {
        Self {
            steamship_open: false,
            swamp_epidemic: true,
            lake_state: LakeState::Clean,
            elected_candidate: None,
            horse: HorseStationState::default(),
            noodles: NoodlesState::default(),
        }
    }
}

impl SharedChanges {
    /// The server-save owner supplies the retiring epoch's committed tallies.
    /// No contribution can activate an effect before this immutable next-day projection.
    pub fn at_save(&self, contributions: &SaveContributions) -> Self {
        let mut next = self.clone();
        next.steamship_open = contributions.coal >= COAL_UNLOCK;
        next.swamp_epidemic = contributions.medicine < MEDICINE_CURE;
        // Owner-authorized conservative recovery: each day without 1000 deposits
        // restores one lake stage; each qualifying day worsens it by one stage.
        next.lake_state = match (self.lake_state, contributions.corpses >= CORPSE_POLLUTION) {
            (LakeState::Clean, false) | (LakeState::Dirty, false) => LakeState::Clean,
            (LakeState::Clean, true) | (LakeState::Stinking, false) => LakeState::Dirty,
            (LakeState::Dirty | LakeState::Stinking, true) => LakeState::Stinking,
        };
        if let Some(winner) = &contributions.winning_ballot {
            next.elected_candidate = Some(winner.clone());
        }
        next.noodles.at_save();
        next
    }
}

pub fn can_catch_shimmer(now_ms: i64, last_catch_ms: Option<i64>) -> bool {
    last_catch_ms.is_none_or(|last| {
        now_ms
            .checked_sub(last)
            .is_some_and(|elapsed| elapsed >= PERSONAL_CATCH_INTERVAL_MS)
    })
}

pub fn rank_for_spent_votes(spent: u64) -> u8 {
    match spent {
        0..50 => 0,
        50..150 => 1,
        150..300 => 2,
        300..500 => 3,
        _ => 4,
    }
}

/// Tallies are already grouped by ballot identity. A tie retains the incumbent.
/// This deterministic no-award tie rule is an owner-authorized conservative assumption.
pub fn election_winner(ballots: &[(String, u64)]) -> Option<String> {
    let mut winner: Option<&str> = None;
    let mut highest = 0;
    let mut tied = false;
    for (index, (ballot, votes)) in ballots.iter().enumerate() {
        if ballot.is_empty()
            || ballot.len() > 128
            || ballots[..index]
                .iter()
                .any(|(previous, _)| previous == ballot)
        {
            return None;
        }
        if *votes > highest {
            highest = *votes;
            winner = Some(ballot);
            tied = false;
        } else if *votes == highest {
            tied = true;
        }
    }
    if tied {
        None
    } else {
        winner.map(str::to_owned)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HorseStationState {
    pub stable_horses: u16,
    pub escape_started_ms: Option<i64>,
    pub raid_until_ms: Option<i64>,
}

impl HorseStationState {
    pub fn raid_active(&self, now_ms: i64) -> bool {
        self.raid_until_ms.is_some_and(|until| now_ms < until)
    }

    pub fn escape(&mut self, now_ms: i64) -> bool {
        if self.raid_active(now_ms) || self.stable_horses == 0 {
            return false;
        }
        self.stable_horses -= 1;
        self.escape_started_ms.get_or_insert(now_ms);
        true
    }

    /// The native stable owner supplies its sourced horse count on expiry.
    /// Owner-authorized assumption: escape activates at three hours or zero horses;
    /// a raid lasts three hours, then the existing stable projection is restored.
    pub fn advance(&mut self, now_ms: i64, restored_horses: u16) {
        if let Some(until) = self.raid_until_ms {
            if now_ms >= until {
                self.raid_until_ms = None;
                self.escape_started_ms = None;
                self.stable_horses = restored_horses;
            }
            return;
        }
        if self.stable_horses == 0
            || self.escape_started_ms.is_some_and(|started| {
                now_ms
                    .checked_sub(started)
                    .is_some_and(|elapsed| elapsed >= HORSE_RAID_INTERVAL_MS)
            })
        {
            self.raid_until_ms = now_ms.checked_add(HORSE_RAID_INTERVAL_MS);
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NoodlesState {
    pub missing: bool,
    pub returned: bool,
    pub dog_lady: bool,
}

impl NoodlesState {
    pub fn disappear(&mut self) {
        self.missing = true;
        self.returned = false;
        self.dog_lady = false;
    }

    /// Custody and escort admission belong to the fenced durable caller.
    pub fn return_to_palace(&mut self) -> bool {
        if !self.missing {
            return false;
        }
        self.missing = false;
        self.returned = true;
        true
    }

    pub fn at_save(&mut self) {
        self.dog_lady |= self.returned;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wcontrib_thresholds_take_effect_only_at_save() {
        let before = SharedChanges::default();
        assert!(!before.steamship_open);
        assert!(before.swamp_epidemic);
        let below = before.at_save(&SaveContributions {
            coal: 199,
            medicine: 99,
            corpses: 999,
            winning_ballot: None,
        });
        assert!(!below.steamship_open);
        assert!(below.swamp_epidemic);
        assert_eq!(below.lake_state, LakeState::Clean);
        let unlocked = before.at_save(&SaveContributions {
            coal: 200,
            medicine: 100,
            corpses: 1000,
            winning_ballot: None,
        });
        assert!(unlocked.steamship_open);
        assert!(!unlocked.swamp_epidemic);
        assert_eq!(unlocked.lake_state, LakeState::Dirty);
        let stinking = unlocked.at_save(&SaveContributions {
            corpses: 1000,
            ..SaveContributions::default()
        });
        assert_eq!(stinking.lake_state, LakeState::Stinking);
        let restored = stinking.at_save(&SaveContributions::default());
        assert!(!restored.steamship_open);
        assert!(restored.swamp_epidemic);
        assert_eq!(restored.lake_state, LakeState::Dirty);
        assert_eq!(
            restored.at_save(&SaveContributions::default()).lake_state,
            LakeState::Clean
        );
    }

    #[test]
    fn wcontrib_personal_catch_uses_twenty_hour_boundary() {
        assert!(can_catch_shimmer(0, None));
        assert!(!can_catch_shimmer(71_999_999, Some(0)));
        assert!(can_catch_shimmer(72_000_000, Some(0)));
        assert!(!can_catch_shimmer(-1, Some(0)));
        assert!(!can_catch_shimmer(i64::MAX, Some(i64::MAX)));
    }

    #[test]
    fn wcontrib_rank_and_election_have_exact_thresholds_and_no_tie_winner() {
        for (spent, rank) in [
            (0, 0),
            (49, 0),
            (50, 1),
            (149, 1),
            (150, 2),
            (299, 2),
            (300, 3),
            (499, 3),
            (500, 4),
        ] {
            assert_eq!(rank_for_spent_votes(spent), rank);
        }
        assert_eq!(election_winner(&[]), None);
        assert_eq!(election_winner(&[("a".into(), 0)]), None);
        assert_eq!(election_winner(&[("a".into(), 4), ("b".into(), 4)]), None);
        assert_eq!(
            election_winner(&[("a".into(), 4), ("b".into(), 5)]),
            Some("b".into())
        );
        let state = SharedChanges::default().at_save(&SaveContributions {
            winning_ballot: Some("b".into()),
            ..SaveContributions::default()
        });
        assert_eq!(state.elected_candidate.as_deref(), Some("b"));
    }

    #[test]
    fn wcontrib_horse_escape_activation_expiry_and_stable_restore() {
        let mut horses = HorseStationState {
            stable_horses: 2,
            ..HorseStationState::default()
        };
        assert!(horses.escape(100));
        assert_eq!(horses.stable_horses, 1);
        horses.advance(10_800_099, 2);
        assert!(!horses.raid_active(10_800_099));
        horses.advance(10_800_100, 2);
        assert!(horses.raid_active(10_800_100));
        horses.advance(21_600_100, 2);
        assert!(!horses.raid_active(21_600_100));
        assert_eq!(horses.stable_horses, 2);
        assert!(horses.escape(30_000_000));
        assert!(horses.escape(30_000_001));
        horses.advance(30_000_001, 2);
        assert!(horses.raid_active(30_000_001));
        assert!(!horses.escape(30_000_002));
    }

    #[test]
    fn wcontrib_noodles_return_projects_dog_lady_at_next_save() {
        let mut noodles = NoodlesState::default();
        assert!(!noodles.return_to_palace());
        noodles.disappear();
        assert!(noodles.missing);
        assert!(noodles.return_to_palace());
        assert!(!noodles.missing);
        assert!(!noodles.dog_lady);
        noodles.at_save();
        assert!(noodles.dog_lady);
        assert!(!noodles.return_to_palace());
    }
}
