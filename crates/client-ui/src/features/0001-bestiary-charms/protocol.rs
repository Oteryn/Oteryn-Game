use super::{
    CharmAssignDisposition, CharmAssignIntent, CharmKind, CharmState, CharmUnlockDisposition,
    CharmUnlockStageIntent, Cyclopedia,
};
use std::num::NonZeroU32;

/// Bestiary stage a race needs before a major charm can be assigned to it.
pub const MAJOR_CHARM_RACE_STAGE: u8 = 3;
/// Bestiary stage a race needs before a minor charm can be assigned to it.
pub const MINOR_CHARM_RACE_STAGE: u8 = 2;
const FINAL_CHARM_STAGE: u8 = 3;

#[must_use]
pub const fn unlock_feedback_text(disposition: CharmUnlockDisposition) -> &'static str {
    match disposition {
        CharmUnlockDisposition::Unlocked => "Charm stage unlocked",
        CharmUnlockDisposition::NotEnoughCharmPoints => "Not enough Charm Points",
        CharmUnlockDisposition::NotEnoughMinorCharmEchoes => "Not enough Minor Charm Echoes",
        CharmUnlockDisposition::StageMismatch => "Charm already changed; view refreshed",
        CharmUnlockDisposition::UnknownCharm => "Unknown charm",
        CharmUnlockDisposition::Rejected => "Charms unavailable",
    }
}

#[must_use]
pub const fn assign_feedback_text(disposition: CharmAssignDisposition) -> &'static str {
    match disposition {
        CharmAssignDisposition::Assigned => "Charm assigned",
        CharmAssignDisposition::CharmLocked => "Unlock the charm first",
        CharmAssignDisposition::AlreadyAssigned => "Charm is already assigned",
        CharmAssignDisposition::RaceStageTooLow => "Bestiary entry not far enough",
        CharmAssignDisposition::RaceCharmLimit => "Creature already holds a charm",
        CharmAssignDisposition::UnknownCharm => "Unknown charm",
        CharmAssignDisposition::UnknownRace => "Unknown creature",
        CharmAssignDisposition::Rejected => "Charms unavailable",
        CharmAssignDisposition::AssignmentSlotsFull => "All charm slots are in use",
    }
}

impl Cyclopedia {
    pub fn unlock_request(
        &self,
        charm: NonZeroU32,
    ) -> Result<CharmUnlockStageIntent, CharmUnlockDisposition> {
        let state = self
            .charm(charm)
            .ok_or(CharmUnlockDisposition::UnknownCharm)?;
        if let Some(shortfall) = self.unlock_shortfall(state) {
            return Err(shortfall);
        }
        Ok(CharmUnlockStageIntent {
            charm,
            expected_stage: state.unlocked_stage,
        })
    }

    pub fn assign_request(
        &self,
        charm: NonZeroU32,
        race: NonZeroU32,
    ) -> Result<CharmAssignIntent, CharmAssignDisposition> {
        let state = self
            .charm(charm)
            .ok_or(CharmAssignDisposition::UnknownCharm)?;
        if state.unlocked_stage == 0 {
            return Err(CharmAssignDisposition::CharmLocked);
        }
        if state.assigned_race.is_some() {
            return Err(CharmAssignDisposition::AlreadyAssigned);
        }
        let stage = self
            .bestiary
            .get(&race)
            .map_or(0, super::BestiaryRaceProgress::unlocked_stage);
        if stage < required_race_stage(state.kind) {
            return Err(CharmAssignDisposition::RaceStageTooLow);
        }
        if self.slots_full() {
            return Err(CharmAssignDisposition::AssignmentSlotsFull);
        }
        Ok(CharmAssignIntent { charm, race })
    }

    pub fn record_unlock(&mut self, disposition: CharmUnlockDisposition) {
        self.feedback = Some(unlock_feedback_text(disposition));
    }
    pub fn record_assign(&mut self, disposition: CharmAssignDisposition) {
        self.feedback = Some(assign_feedback_text(disposition));
    }
    #[must_use]
    pub const fn feedback(&self) -> Option<&'static str> {
        self.feedback
    }

    pub(super) fn slots_full(&self) -> bool {
        self.charms.assignment_slot_limit.is_some_and(|limit| {
            usize::try_from(limit.get()).is_ok_and(|limit| self.assignment_slots_in_use() >= limit)
        })
    }
    fn charm(&self, charm: NonZeroU32) -> Option<&CharmState> {
        self.charms.charms.iter().find(|state| state.charm == charm)
    }
    fn unlock_shortfall(&self, state: &CharmState) -> Option<CharmUnlockDisposition> {
        if state.unlocked_stage >= FINAL_CHARM_STAGE {
            return Some(CharmUnlockDisposition::StageMismatch);
        }
        let (balance, shortfall) = match state.kind {
            CharmKind::Major => (
                self.charms.charm_points_available,
                CharmUnlockDisposition::NotEnoughCharmPoints,
            ),
            CharmKind::Minor => (
                self.charms.minor_charm_echoes_available,
                CharmUnlockDisposition::NotEnoughMinorCharmEchoes,
            ),
        };
        (balance < state.next_stage_cost).then_some(shortfall)
    }
}

const fn required_race_stage(kind: CharmKind) -> u8 {
    match kind {
        CharmKind::Major => MAJOR_CHARM_RACE_STAGE,
        CharmKind::Minor => MINOR_CHARM_RACE_STAGE,
    }
}
