//! Exact admitted defender identities; only committed native deaths can settle them.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncounterDefenders {
    participants: Vec<ExactActorRef>,
    defenders: Vec<ExactActorRef>,
    defeated: u8,
}
impl EncounterDefenders {
    pub fn new(
        participants: &[ExactActorRef],
        defenders: &[ExactActorRef],
    ) -> Result<Self, CarrierError> {
        if !(1..=8).contains(&participants.len()) || !(1..=8).contains(&defenders.len()) {
            return Err(CarrierError::PlanConflict);
        }
        Self::from_admission(participants.to_vec(), defenders.to_vec())
    }
    /// Transfer preallocated admission vectors without allocating after publication.
    pub fn from_admission(
        participants: Vec<ExactActorRef>,
        defenders: Vec<ExactActorRef>,
    ) -> Result<Self, CarrierError> {
        if participants.is_empty()
            || participants.len() > 8
            || defenders.is_empty()
            || defenders.len() > 8
        {
            return Err(CarrierError::PlanConflict);
        }
        let first = participants[0];
        let all = participants.iter().chain(&defenders);
        if all.clone().enumerate().any(|(index, actor)| {
            actor.runtime_scope() != first.runtime_scope()
                || actor.scope_generation() != first.scope_generation()
                || all.clone().take(index).any(|other| other == actor)
        }) {
            return Err(CarrierError::PlanConflict);
        }
        Ok(Self {
            participants,
            defenders,
            defeated: 0,
        })
    }
    pub fn defenders(&self) -> &[ExactActorRef] {
        &self.defenders
    }
    pub fn participants(&self) -> &[ExactActorRef] {
        &self.participants
    }
    pub fn complete_for(&self, participant: ExactActorRef) -> bool {
        self.participants.contains(&participant)
            && self.defeated.count_ones() as usize == self.defenders.len()
    }
    /// Returns the admitted defender ordinal once, after a physical owner committed death.
    pub fn record_death(&mut self, death: CreatureDeathOccurrenceKey) -> Option<usize> {
        let index = self
            .defenders
            .iter()
            .position(|actor| actor.0 == death.actor)?;
        let bit = 1_u8.checked_shl(u32::try_from(index).ok()?)?;
        if self.defeated & bit != 0 {
            return None;
        }
        self.defeated |= bit;
        Some(index)
    }
}
