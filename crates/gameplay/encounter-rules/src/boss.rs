//! Source-backed rules shared by reward-boss combat and reward settlement.

pub mod reward_chest;

/// Participation accumulated by the combat owner for one character.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BossParticipation {
    pub damage_dealt: u64,
    pub damage_received: u64,
    pub healing_others: u64,
}

/// Participation after applying the boss's current bonus resilience.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RewardParticipation {
    pub damage_dealt: u64,
    pub damage_received: u64,
    pub healing_others: u64,
}

impl RewardParticipation {
    pub const fn eligible(self) -> bool {
        self.damage_dealt != 0 || self.damage_received != 0 || self.healing_others != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParticipationError {
    InvalidBonusResilience,
    Overflow,
}

/// Applies NEWS 7719's rule: bonus resilience does not reduce damage-dealt
/// participation, but reduces every other participation source by the same percentage.
///
/// The percentage is an encounter fact rather than a constant because neither pinned
/// server supplies the initial resilience curve or a universal boss value.
pub fn reward_participation(
    raw: BossParticipation,
    bonus_resilience_percent: u8,
) -> Result<RewardParticipation, ParticipationError> {
    if bonus_resilience_percent > 100 {
        return Err(ParticipationError::InvalidBonusResilience);
    }
    let retained = 100_u64 - u64::from(bonus_resilience_percent);
    let scale = |value: u64| {
        value
            .checked_mul(retained)
            .map(|scaled| scaled / 100)
            .ok_or(ParticipationError::Overflow)
    };
    Ok(RewardParticipation {
        damage_dealt: raw.damage_dealt,
        damage_received: scale(raw.damage_received)?,
        healing_others: scale(raw.healing_others)?,
    })
}

/// Crystal 201189f `data/scripts/systems/reward_chest.lua:28-69` sums all
/// three normalized contributions and rewards every nonzero score. Since each
/// denominator is positive (seeded with 0.1), this is exactly a nonzero input
/// in any dimension, without rounding tiny contributions away.
pub fn reward_eligibility(participants: &[RewardParticipation]) -> Vec<bool> {
    participants
        .iter()
        .map(|participant| participant.eligible())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(damage_dealt: u64, damage_received: u64, healing_others: u64) -> RewardParticipation {
        RewardParticipation {
            damage_dealt,
            damage_received,
            healing_others,
        }
    }

    fn team() -> Vec<RewardParticipation> {
        vec![
            part(150_000, 600_000, 0),
            part(300_000, 60_000, 0),
            part(350_000, 40_000, 0),
            part(150_000, 30_000, 500_000),
            part(50_000, 20_000, 0),
        ]
    }

    #[test]
    fn crystal_nonzero_single_hit_and_team_keep_rewards() {
        let mut parts = team();
        parts.push(part(1, 0, 0));
        assert_eq!(reward_eligibility(&parts), vec![true; 6]);
    }

    #[test]
    fn weak_damage_dealer_and_healer_still_qualify() {
        // Crystal reward_chest.lua:54-69: any nonzero contribution score qualifies.
        assert_eq!(reward_eligibility(&team()), vec![true; 5]);
        let healer_with_one_hit = [part(400_000, 100_000, 0), part(1, 0, 300_000)];
        assert_eq!(reward_eligibility(&healer_with_one_hit), vec![true, true]);
    }

    #[test]
    fn support_only_and_solo_cases() {
        // Crystal 201189f data/scripts/systems/reward_chest.lua:54-69,156-165.
        // Healing-only and tanking-only scores qualify; zero contribution does not.
        assert_eq!(
            reward_eligibility(&[part(0, 0, 1), part(0, 1, 0), part(0, 0, 0)]),
            vec![true, true, false]
        );
        assert_eq!(
            reward_eligibility(&[part(500_000, 0, 0), part(0, 900_000, 900_000)]),
            vec![true, true]
        );
        assert_eq!(reward_eligibility(&[part(1, 0, 0)]), vec![true]);
        assert_eq!(
            reward_eligibility(&[part(1_000_000, 300_000, 50_000), part(1, 0, 0)]),
            vec![true, true]
        );
        assert!(reward_eligibility(&[]).is_empty());
    }
}
