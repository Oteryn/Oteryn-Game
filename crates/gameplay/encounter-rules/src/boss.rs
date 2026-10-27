//! Source-backed rules shared by reward-boss combat and reward settlement.

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
