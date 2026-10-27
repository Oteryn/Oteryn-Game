//! Reward-boss combat/reward dispatch for resilience-adjusted participation.

use oteryn_gameplay_encounter_rules::boss::reward_participation;
pub use oteryn_gameplay_encounter_rules::boss::{
    BossParticipation, ParticipationError, RewardParticipation,
};

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

#[cfg(test)]
mod tests {
    use super::*;

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
