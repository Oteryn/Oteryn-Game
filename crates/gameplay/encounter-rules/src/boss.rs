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
        self.damage_dealt != 0
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

/// Oteryn deviation (owner decision 2026-10-27): a damage-dealing character must also reach
/// this percentage of the fair share `1 / n`, where `n` counts the damage-dealing characters.
/// The pinned loot factor floors every nonzero score at `n^(-1/3)`, so a single hit earned
/// about 0.55 of a full reward in a five-player team. See
/// `docs/reference/news-history/boss-resilience-and-participation.md`.
pub const REWARD_FAIR_SHARE_THRESHOLD_PERCENT: u64 = 10;

const SHARE_SCALE: u128 = 1_000_000_000;

fn share(value: u64, total: u128) -> u128 {
    (u128::from(value) * SHARE_SCALE)
        .checked_div(total)
        .unwrap_or(0)
}

/// Returns, in input order, whether each resilience-adjusted participant earns a reward.
///
/// The score is the pinned three-dimension score `(d/D + r/R + h/H) / 3` in fixed point,
/// with totals taken over every participant. A participant qualifies when it dealt damage
/// and its score is at least `REWARD_FAIR_SHARE_THRESHOLD_PERCENT` of `1 / n`.
pub fn reward_eligibility(participants: &[RewardParticipation]) -> Vec<bool> {
    let total = |pick: fn(&RewardParticipation) -> u64| -> u128 {
        participants.iter().map(|p| u128::from(pick(p))).sum()
    };
    let dealt = total(|p| p.damage_dealt);
    let received = total(|p| p.damage_received);
    let healed = total(|p| p.healing_others);
    let damage_dealers: u128 = participants
        .iter()
        .filter(|p| p.eligible())
        .map(|_| 1_u128)
        .sum();
    participants
        .iter()
        .map(|p| {
            let score_sum = share(p.damage_dealt, dealt)
                + share(p.damage_received, received)
                + share(p.healing_others, healed);
            // score_sum / (3 * SCALE) >= pct / (100 * n)
            p.eligible()
                && score_sum * 100 * damage_dealers
                    >= u128::from(REWARD_FAIR_SHARE_THRESHOLD_PERCENT) * 3 * SHARE_SCALE
        })
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
    fn single_hit_leech_is_excluded_and_team_keeps_rewards() {
        let mut parts = team();
        parts.push(part(1, 0, 0));
        assert_eq!(
            reward_eligibility(&parts),
            vec![true, true, true, true, true, false]
        );
    }

    #[test]
    fn weak_damage_dealer_and_healer_still_qualify() {
        // The 50k dealer holds about 2.6% of the score, above 10% of the 20% fair share.
        assert_eq!(reward_eligibility(&team()), vec![true; 5]);
        let healer_with_one_hit = [part(400_000, 100_000, 0), part(1, 0, 300_000)];
        assert_eq!(reward_eligibility(&healer_with_one_hit), vec![true, true]);
    }

    #[test]
    fn support_only_and_solo_cases() {
        assert_eq!(
            reward_eligibility(&[part(500_000, 0, 0), part(0, 900_000, 900_000)]),
            vec![true, false]
        );
        assert_eq!(reward_eligibility(&[part(1, 0, 0)]), vec![true]);
        assert_eq!(
            reward_eligibility(&[part(1_000_000, 300_000, 50_000), part(1, 0, 0)]),
            vec![true, false]
        );
        assert!(reward_eligibility(&[]).is_empty());
    }
}
