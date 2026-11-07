use super::{CharmKind, CharmState, Cyclopedia, MAJOR_CHARM_RACE_STAGE, MINOR_CHARM_RACE_STAGE};
use std::num::NonZeroU32;

pub const CHARM_EFFECT_NOT_ACTIVE_TEXT: &str = "Effect not yet active";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BestiaryRow {
    pub race: NonZeroU32,
    pub kill_count: u32,
    pub unlocked_stage: u8,
    pub next_threshold: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharmRow {
    pub state: CharmState,
    pub can_unlock: bool,
    pub can_assign: bool,
}

impl CharmRow {
    #[must_use]
    pub const fn effect_note(&self) -> Option<&'static str> {
        if self.state.effect_active {
            None
        } else {
            Some(CHARM_EFFECT_NOT_ACTIVE_TEXT)
        }
    }
}

impl Cyclopedia {
    pub fn bestiary_rows(&self) -> impl Iterator<Item = BestiaryRow> + '_ {
        self.bestiary.values().map(|race| BestiaryRow {
            race: race.race,
            kill_count: race.kill_count,
            unlocked_stage: race.unlocked_stage(),
            next_threshold: race.next_threshold(),
        })
    }
    #[must_use]
    pub const fn charm_points_available(&self) -> u32 {
        self.charms.charm_points_available
    }
    #[must_use]
    pub const fn minor_charm_echoes_available(&self) -> u32 {
        self.charms.minor_charm_echoes_available
    }
    #[must_use]
    pub fn assignment_slots_in_use(&self) -> usize {
        self.charms
            .charms
            .iter()
            .filter(|state| state.assigned_race.is_some())
            .count()
    }
    #[must_use]
    pub const fn assignment_slot_limit(&self) -> Option<NonZeroU32> {
        self.charms.assignment_slot_limit
    }
    pub fn charm_rows(&self) -> impl Iterator<Item = CharmRow> + '_ {
        let slot_free = !self.slots_full();
        self.charms.charms.iter().map(move |state| CharmRow {
            state: *state,
            can_unlock: self.unlock_request(state.charm).is_ok(),
            can_assign: state.unlocked_stage > 0 && state.assigned_race.is_none() && slot_free,
        })
    }
    pub fn assignable_races(&self, charm: NonZeroU32) -> impl Iterator<Item = NonZeroU32> + '_ {
        let required = self
            .charms
            .charms
            .iter()
            .find(|state| state.charm == charm)
            .map(|state| match state.kind {
                CharmKind::Major => MAJOR_CHARM_RACE_STAGE,
                CharmKind::Minor => MINOR_CHARM_RACE_STAGE,
            });
        self.bestiary
            .values()
            .filter(move |race| required.is_some_and(|stage| race.unlocked_stage() >= stage))
            .map(|race| race.race)
    }
}

/// Registration target for the generated Bestiary navigation entry.
pub fn bestiary_page() {}
/// Registration target for the generated Charms navigation entry.
pub fn charms_page() {}
