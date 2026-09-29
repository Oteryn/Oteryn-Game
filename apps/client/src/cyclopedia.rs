//! Bestiary and Charm views (CHARM-5 client half,
//! `docs/contracts/protocol-oteryn/CHARM5_BESTIARY_CHARM_WIRE_PROPOSAL_V1.md` §6).
//!
//! Pure client state over the session crate's types: the last Bestiary and Charm views the server
//! sent, the rows a panel draws from them, the two commands the player can send (unlock the next
//! stage, assign a charm to a race; there is no unassign in this version), and one feedback line
//! per result. The server decides every command; the local checks only keep the client from
//! sending a command it can already see will fail, and report it with the same disposition the
//! server would return. The session routes neither the commands nor the views until the protocol
//! owner registers their IDs.

use oteryn_session::{
    BestiaryRaceProgress, CharmAssignDisposition, CharmAssignIntent, CharmKind, CharmState,
    CharmUnlockDisposition, CharmUnlockStageIntent, CharmView,
};
use std::collections::BTreeMap;
use std::num::NonZeroU32;

/// Bestiary stage a race needs before a major charm can be assigned to it: the complete entry
/// (CHARM-0 §4.2). A local hint only; CHARM-3 decides.
pub const MAJOR_CHARM_RACE_STAGE: u8 = 3;
/// Bestiary stage a race needs before a minor charm can be assigned to it (CHARM-0 §4.2).
pub const MINOR_CHARM_RACE_STAGE: u8 = 2;
const FINAL_CHARM_STAGE: u8 = 3;

/// Player-facing line for one unlock result.
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

/// Player-facing line for one assign result.
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
    }
}

/// One Bestiary row: the counted race with its derived stage and progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BestiaryRow {
    pub race: NonZeroU32,
    pub kill_count: u32,
    pub unlocked_stage: u8,
    /// The next threshold, or `None` once the entry is complete.
    pub next_threshold: Option<u32>,
}

/// One Charm row with what the player can do with it now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharmRow {
    pub state: CharmState,
    /// The next stage exists and the balance of the charm's currency covers it.
    pub can_unlock: bool,
    /// Unlocked and not yet assigned.
    pub can_assign: bool,
}

/// The client's Bestiary and Charm state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cyclopedia {
    bestiary: BTreeMap<NonZeroU32, BestiaryRaceProgress>,
    charms: CharmView,
    feedback: Option<&'static str>,
}

impl Cyclopedia {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A Bestiary snapshot replaces every race.
    pub fn apply_bestiary_snapshot(&mut self, races: Vec<BestiaryRaceProgress>) {
        self.bestiary.clear();
        self.apply_bestiary_delta(races);
    }

    /// A Bestiary delta upserts the listed races and leaves the others unchanged.
    pub fn apply_bestiary_delta(&mut self, races: Vec<BestiaryRaceProgress>) {
        self.bestiary
            .extend(races.into_iter().map(|race| (race.race, race)));
    }

    /// A Charm snapshot or delta: both carry the whole view.
    pub fn apply_charm_view(&mut self, view: CharmView) {
        self.charms = view;
    }

    /// Races in ascending order.
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

    /// Charms in the order the server sent them (ascending).
    pub fn charm_rows(&self) -> impl Iterator<Item = CharmRow> + '_ {
        self.charms.charms.iter().map(|state| CharmRow {
            state: *state,
            can_unlock: self.unlock_shortfall(state).is_none(),
            can_assign: state.unlocked_stage > 0 && state.assigned_race.is_none(),
        })
    }

    /// Races the charm could be assigned to now, by the local stage hint.
    pub fn assignable_races(&self, charm: NonZeroU32) -> impl Iterator<Item = NonZeroU32> + '_ {
        let required = self
            .charm(charm)
            .map(|state| required_race_stage(state.kind));
        self.bestiary
            .values()
            .filter(move |race| required.is_some_and(|stage| race.unlocked_stage() >= stage))
            .map(|race| race.race)
    }

    /// The intent to send, or the disposition the server would return for it.
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

    /// The intent to send, or the disposition the server would return for it. A race the view
    /// does not list has no kills, so its stage is too low.
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
            .map_or(0, BestiaryRaceProgress::unlocked_stage);
        if stage < required_race_stage(state.kind) {
            return Err(CharmAssignDisposition::RaceStageTooLow);
        }
        Ok(CharmAssignIntent { charm, race })
    }

    /// Records the result of an unlock, whether the server or the local check produced it.
    pub fn record_unlock(&mut self, disposition: CharmUnlockDisposition) {
        self.feedback = Some(unlock_feedback_text(disposition));
    }

    /// Records the result of an assign, whether the server or the local check produced it.
    pub fn record_assign(&mut self, disposition: CharmAssignDisposition) {
        self.feedback = Some(assign_feedback_text(disposition));
    }

    #[must_use]
    pub const fn feedback(&self) -> Option<&'static str> {
        self.feedback
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn index(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).expect("non-zero index")
    }

    fn race(race: u32, kill_count: u32) -> BestiaryRaceProgress {
        BestiaryRaceProgress {
            race: index(race),
            kill_count,
            kill_thresholds: [5, 10, 25],
        }
    }

    fn charm(charm: u32, kind: CharmKind, stage: u8, assigned: Option<u32>) -> CharmState {
        CharmState {
            charm: index(charm),
            kind,
            unlocked_stage: stage,
            assigned_race: assigned.map(index),
            next_stage_cost: if stage == 3 { 0 } else { 100 },
        }
    }

    /// Race 1 at stage 1, race 2 at stage 2, race 3 complete. Charm 1 major locked, 2 minor
    /// unlocked, 3 major unlocked and assigned, 4 minor complete. 150 points, 50 echoes.
    fn cyclopedia() -> Cyclopedia {
        let mut view = Cyclopedia::new();
        view.apply_bestiary_snapshot(vec![race(3, 25), race(1, 5), race(2, 10)]);
        view.apply_charm_view(CharmView {
            charms: vec![
                charm(1, CharmKind::Major, 0, None),
                charm(2, CharmKind::Minor, 1, None),
                charm(3, CharmKind::Major, 1, Some(3)),
                charm(4, CharmKind::Minor, 3, None),
            ],
            charm_points_available: 150,
            minor_charm_echoes_available: 50,
        });
        view
    }

    #[test]
    fn bestiary_rows_derive_stage_and_progress_and_apply_deltas() {
        let mut view = cyclopedia();
        let rows: Vec<_> = view.bestiary_rows().collect();
        assert_eq!(
            rows.iter()
                .map(|row| (row.race.get(), row.unlocked_stage, row.next_threshold))
                .collect::<Vec<_>>(),
            [(1, 1, Some(10)), (2, 2, Some(25)), (3, 3, None)]
        );
        // A delta upserts only its races.
        view.apply_bestiary_delta(vec![race(1, 24), race(7, 1)]);
        let rows: Vec<_> = view
            .bestiary_rows()
            .map(|row| (row.race.get(), row.kill_count))
            .collect();
        assert_eq!(rows, [(1, 24), (2, 10), (3, 25), (7, 1)]);
        // A snapshot replaces every race.
        view.apply_bestiary_snapshot(vec![race(9, 1)]);
        assert_eq!(view.bestiary_rows().count(), 1);
    }

    #[test]
    fn charm_rows_show_what_can_be_done() {
        let view = cyclopedia();
        let rows: Vec<_> = view
            .charm_rows()
            .map(|row| (row.state.charm.get(), row.can_unlock, row.can_assign))
            .collect();
        // Charm 2 is minor and 50 echoes do not cover 100; charm 4 has no next stage.
        assert_eq!(
            rows,
            [
                (1, true, false),
                (2, false, true),
                (3, true, false),
                (4, false, true)
            ]
        );
        assert_eq!(view.charm_points_available(), 150);
        assert_eq!(view.minor_charm_echoes_available(), 50);
    }

    #[test]
    fn unlock_requests_carry_the_seen_stage_or_the_servers_answer() {
        let view = cyclopedia();
        assert_eq!(
            view.unlock_request(index(3)),
            Ok(CharmUnlockStageIntent {
                charm: index(3),
                expected_stage: 1
            })
        );
        assert_eq!(
            view.unlock_request(index(1)),
            Ok(CharmUnlockStageIntent {
                charm: index(1),
                expected_stage: 0
            })
        );
        assert_eq!(
            view.unlock_request(index(2)),
            Err(CharmUnlockDisposition::NotEnoughMinorCharmEchoes)
        );
        assert_eq!(
            view.unlock_request(index(4)),
            Err(CharmUnlockDisposition::StageMismatch)
        );
        assert_eq!(
            view.unlock_request(index(5)),
            Err(CharmUnlockDisposition::UnknownCharm)
        );
        let mut poor = cyclopedia();
        poor.apply_charm_view(CharmView {
            charms: vec![charm(1, CharmKind::Major, 0, None)],
            charm_points_available: 99,
            minor_charm_echoes_available: 1000,
        });
        assert_eq!(
            poor.unlock_request(index(1)),
            Err(CharmUnlockDisposition::NotEnoughCharmPoints)
        );
    }

    #[test]
    fn assign_requests_follow_the_stage_hint_and_never_unassign() {
        let view = cyclopedia();
        // Minor charm 2 needs stage 2: races 2 and 3.
        assert_eq!(
            view.assignable_races(index(2))
                .map(NonZeroU32::get)
                .collect::<Vec<_>>(),
            [2, 3]
        );
        assert_eq!(
            view.assign_request(index(2), index(2)),
            Ok(CharmAssignIntent {
                charm: index(2),
                race: index(2)
            })
        );
        assert_eq!(
            view.assign_request(index(2), index(1)),
            Err(CharmAssignDisposition::RaceStageTooLow)
        );
        // A race with no kills is not in the view.
        assert_eq!(
            view.assign_request(index(2), index(50)),
            Err(CharmAssignDisposition::RaceStageTooLow)
        );
        // Major charm 3 needs a complete entry, but it is already assigned: no unassign.
        assert_eq!(
            view.assign_request(index(3), index(3)),
            Err(CharmAssignDisposition::AlreadyAssigned)
        );
        assert_eq!(
            view.assign_request(index(1), index(3)),
            Err(CharmAssignDisposition::CharmLocked)
        );
        assert_eq!(
            view.assign_request(index(9), index(3)),
            Err(CharmAssignDisposition::UnknownCharm)
        );
        assert_eq!(view.assignable_races(index(9)).count(), 0);
    }

    #[test]
    fn every_result_has_a_feedback_line() {
        let mut view = Cyclopedia::new();
        assert_eq!(view.feedback(), None);
        view.record_unlock(CharmUnlockDisposition::Rejected);
        assert_eq!(view.feedback(), Some("Charms unavailable"));
        view.record_assign(CharmAssignDisposition::Assigned);
        assert_eq!(view.feedback(), Some("Charm assigned"));
        for disposition in [
            CharmUnlockDisposition::Unlocked,
            CharmUnlockDisposition::NotEnoughCharmPoints,
            CharmUnlockDisposition::NotEnoughMinorCharmEchoes,
            CharmUnlockDisposition::StageMismatch,
            CharmUnlockDisposition::UnknownCharm,
            CharmUnlockDisposition::Rejected,
        ] {
            assert!(!unlock_feedback_text(disposition).is_empty());
        }
        for disposition in [
            CharmAssignDisposition::Assigned,
            CharmAssignDisposition::CharmLocked,
            CharmAssignDisposition::AlreadyAssigned,
            CharmAssignDisposition::RaceStageTooLow,
            CharmAssignDisposition::RaceCharmLimit,
            CharmAssignDisposition::UnknownCharm,
            CharmAssignDisposition::UnknownRace,
            CharmAssignDisposition::Rejected,
        ] {
            assert!(!assign_feedback_text(disposition).is_empty());
        }
    }
}
