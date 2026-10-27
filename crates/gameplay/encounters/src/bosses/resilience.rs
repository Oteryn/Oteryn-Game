//! Reward-boss combat/reward dispatch for resilience-adjusted participation.

pub use oteryn_gameplay_encounter_rules::boss::{
    BossParticipation, ParticipationError, REWARD_FAIR_SHARE_THRESHOLD_PERCENT, RewardParticipation,
};
use oteryn_gameplay_encounter_rules::boss::{reward_eligibility, reward_participation};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BossDispatch {
    Combat(BossParticipation),
    Reward,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BossDispatchOutcome {
    CombatRecorded,
    Reward(Option<RewardParticipation>),
}

/// Encounter-owned state reached by both combat recording and reward settlement.
#[derive(Debug)]
pub struct BossResilienceOwner {
    bonus_resilience_percent: u8,
    participation: BossParticipation,
}

impl BossResilienceOwner {
    pub fn new(bonus_resilience_percent: u8) -> Result<Self, ParticipationError> {
        reward_participation(BossParticipation::default(), bonus_resilience_percent)?;
        Ok(Self {
            bonus_resilience_percent,
            participation: BossParticipation::default(),
        })
    }

    pub fn dispatch(
        &mut self,
        command: BossDispatch,
    ) -> Result<BossDispatchOutcome, ParticipationError> {
        match command {
            BossDispatch::Combat(delta) => {
                self.participation.damage_dealt = self
                    .participation
                    .damage_dealt
                    .checked_add(delta.damage_dealt)
                    .ok_or(ParticipationError::Overflow)?;
                self.participation.damage_received = self
                    .participation
                    .damage_received
                    .checked_add(delta.damage_received)
                    .ok_or(ParticipationError::Overflow)?;
                self.participation.healing_others = self
                    .participation
                    .healing_others
                    .checked_add(delta.healing_others)
                    .ok_or(ParticipationError::Overflow)?;
                Ok(BossDispatchOutcome::CombatRecorded)
            }
            BossDispatch::Reward => {
                let adjusted =
                    reward_participation(self.participation, self.bonus_resilience_percent)?;
                Ok(BossDispatchOutcome::Reward(
                    adjusted.eligible().then_some(adjusted),
                ))
            }
        }
    }
}

/// Settles one boss kill: applies each owner's resilience, then the damage gate and the
/// fair-share threshold across every participant. `None` means no reward.
pub fn settle_rewards(
    owners: &[BossResilienceOwner],
) -> Result<Vec<Option<RewardParticipation>>, ParticipationError> {
    let adjusted = owners
        .iter()
        .map(|owner| reward_participation(owner.participation, owner.bonus_resilience_percent))
        .collect::<Result<Vec<_>, _>>()?;
    let eligible = reward_eligibility(&adjusted);
    Ok(adjusted
        .into_iter()
        .zip(eligible)
        .map(|(participation, eligible)| eligible.then_some(participation))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settlement_drops_single_hit_leech() -> Result<(), ParticipationError> {
        let mut owners = Vec::new();
        for (dealt, received) in [(400_000, 300_000), (500_000, 100_000), (1, 0)] {
            let mut owner = BossResilienceOwner::new(25)?;
            owner.dispatch(BossDispatch::Combat(BossParticipation {
                damage_dealt: dealt,
                damage_received: received,
                healing_others: 0,
            }))?;
            owners.push(owner);
        }
        let settled = settle_rewards(&owners)?;
        assert_eq!(
            settled.iter().map(Option::is_some).collect::<Vec<_>>(),
            vec![true, true, false]
        );
        assert_eq!(
            settled.first(),
            Some(&Some(RewardParticipation {
                damage_dealt: 400_000,
                damage_received: 225_000,
                healing_others: 0,
            }))
        );
        Ok(())
    }

    #[test]
    fn combat_and_reward_dispatch_apply_resilience_and_require_damage()
    -> Result<(), ParticipationError> {
        let mut participant = BossResilienceOwner::new(25)?;
        assert_eq!(
            participant.dispatch(BossDispatch::Combat(BossParticipation {
                damage_dealt: 1,
                damage_received: 400,
                healing_others: 200,
            })),
            Ok(BossDispatchOutcome::CombatRecorded)
        );
        assert_eq!(
            participant.dispatch(BossDispatch::Reward),
            Ok(BossDispatchOutcome::Reward(Some(RewardParticipation {
                damage_dealt: 1,
                damage_received: 300,
                healing_others: 150,
            })))
        );

        let mut support_only = BossResilienceOwner::new(25)?;
        support_only.dispatch(BossDispatch::Combat(BossParticipation {
            damage_dealt: 0,
            damage_received: 400,
            healing_others: 200,
        }))?;
        assert_eq!(
            support_only.dispatch(BossDispatch::Reward),
            Ok(BossDispatchOutcome::Reward(None))
        );
        Ok(())
    }
}
