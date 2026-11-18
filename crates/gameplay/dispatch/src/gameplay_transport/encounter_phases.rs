//! One phase registry on the existing encounter, native death queue and quest writer.
use super::{actor_spell::ChannelSpellStates, grizzly_kills, kill_reward::PendingKillSettlement};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
    MovementPositionSnapshot,
};
use oteryn_gameplay_encounters::bosses::phase_controller::count_vlarkorth::CountVlarkorthEncounter;
use oteryn_gameplay_encounters::bosses::phase_controller::cults::{self, CultsEncounter};
use oteryn_gameplay_encounters::bosses::phase_controller::duke_krule::DukeKruleEncounter;
use oteryn_gameplay_encounters::bosses::phase_controller::lord_azaram::LordAzaramEncounter;
use oteryn_gameplay_encounters::bosses::phase_controller::zelos::ZelosEncounter;
use oteryn_gameplay_encounters::bosses::phase_controller::{
    earl_osam::{self, EarlOsamEncounter},
    sirs::SirsEncounter,
};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, EncounterPhase};
use oteryn_gameplay_world::map::{boot::BundleCollisionIndexExt, view::MapFacts};
use std::sync::Mutex;

mod duke_krule;
mod zelos;

const MAX_ENCOUNTERS: usize = 64;

#[derive(Debug)]
pub(super) struct ActivePhase {
    key: u64,
    fight: Fight,
    credits: Vec<grizzly_kills::EncounterCredit>,
    frozen: Option<(ExactActorRef, Vec<super::quest_kills::Recipient>)>,
    cells: crate::content::NativeEntryMovementCells,
    timed_room: Option<TimedRoom>,
    victory_exit_us: Option<u64>,
    scoped_deaths: Vec<ExactActorRef>,
}

#[derive(Debug)]
struct TimedRoom {
    exit: MovementLocalPosition,
    north_vortex: Option<(MovementLocalPosition, MovementLocalPosition)>,
    corridors: Vec<(MovementLocalPosition, MovementLocalPosition)>,
    min: MovementLocalPosition,
    max: MovementLocalPosition,
    lever: PhaseLever,
}

#[derive(Debug)]
struct PhaseLever {
    key: u64,
    started_revision: u64,
    reset: crate::map::view::EntryFacts,
}

impl TimedRoom {
    fn contains(&self, at: MovementLocalPosition) -> bool {
        self.min.floor == at.floor
            && (self.min.x..=self.max.x).contains(&at.x)
            && (self.min.y..=self.max.y).contains(&at.y)
    }
}

#[derive(Debug)]
enum Fight {
    CountVlarkorth(CountVlarkorthEncounter),
    Cults(CultsEncounter),
    DukeKrule(DukeKruleEncounter),
    EarlOsam(EarlOsamEncounter),
    LordAzaram(LordAzaramEncounter),
    Sirs(SirsEncounter),
    Zelos(ZelosEncounter),
}

impl Fight {
    fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        match self {
            Self::Cults(fight) => fight.is_current(runtime),
            Self::CountVlarkorth(fight) => fight.is_current(runtime),
            Self::DukeKrule(fight) => fight.is_current(runtime),
            Self::EarlOsam(fight) => fight.is_current(runtime),
            Self::LordAzaram(fight) => fight.is_current(runtime),
            Self::Sirs(fight) => fight.is_current(runtime),
            Self::Zelos(fight) => fight.is_current(runtime),
        }
    }

    fn contains(&self, actor: ExactActorRef) -> bool {
        match self {
            Self::Cults(fight) => fight.contains(actor),
            Self::CountVlarkorth(fight) => fight.contains(actor),
            Self::DukeKrule(fight) => fight.contains(actor),
            Self::EarlOsam(fight) => fight.contains(actor),
            Self::LordAzaram(fight) => fight.contains(actor),
            Self::Sirs(fight) => fight.contains(actor),
            Self::Zelos(fight) => fight.contains(actor),
        }
    }

    fn owns_scoped_credit(&self, actor: ExactActorRef) -> bool {
        match self {
            Self::Cults(_) => false,
            Self::CountVlarkorth(fight) => fight.contains(actor),
            Self::DukeKrule(fight) => fight.contains(actor),
            Self::EarlOsam(fight) => fight.contains(actor),
            Self::LordAzaram(fight) => fight.owns_scoped_credit(actor),
            Self::Sirs(fight) => fight.contains(actor),
            Self::Zelos(fight) => fight.contains_terminal(actor),
        }
    }

    fn contains_terminal(&self, actor: ExactActorRef) -> bool {
        match self {
            Self::Cults(fight) => fight.contains_terminal(actor),
            Self::CountVlarkorth(_)
            | Self::DukeKrule(_)
            | Self::EarlOsam(_)
            | Self::LordAzaram(_)
            | Self::Sirs(_) => false,
            Self::Zelos(fight) => fight.contains_terminal(actor),
        }
    }

    fn victors(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&[(ExactActorRef, GameSessionId)]>, CarrierError> {
        match self {
            Self::Cults(fight) => fight.victors(runtime),
            Self::CountVlarkorth(fight) => fight.victors(runtime),
            Self::DukeKrule(fight) => fight.victors(runtime),
            Self::EarlOsam(fight) => fight.victors(runtime),
            Self::LordAzaram(fight) => fight.victors(runtime),
            Self::Sirs(fight) => fight.victors(runtime),
            Self::Zelos(fight) => fight.victors(runtime),
        }
    }

    fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        match self {
            Self::Cults(fight) => fight.projected_death(runtime, actor),
            Self::CountVlarkorth(fight) => fight.projected_death(runtime, actor),
            Self::DukeKrule(fight) => fight.projected_death(runtime, actor),
            Self::EarlOsam(fight) => fight.projected_death(runtime, actor),
            Self::LordAzaram(fight) => fight.projected_death(runtime, actor),
            Self::Sirs(fight) => fight.projected_death(runtime, actor),
            Self::Zelos(fight) => fight.projected_death(runtime, actor),
        }
    }

    fn timeout_participants(&self, now: u64) -> Option<&[(ExactActorRef, GameSessionId)]> {
        match self {
            Self::Cults(_) => None,
            Self::CountVlarkorth(fight) => fight.timeout_participants(now),
            Self::DukeKrule(fight) => fight.timeout_participants(now),
            Self::EarlOsam(fight) => fight.timeout_participants(now),
            Self::LordAzaram(fight) => fight.timeout_participants(now),
            Self::Sirs(fight) => fight.timeout_participants(now),
            Self::Zelos(fight) => fight.timeout_participants(now),
        }
    }

    fn retire(&self, runtime: &mut ChannelRuntimeV1) -> Result<(), CarrierError> {
        match self {
            Self::Cults(_) => Err(CarrierError::PlanConflict),
            Self::CountVlarkorth(fight) => fight.retire(runtime),
            Self::DukeKrule(fight) => fight.retire(runtime),
            Self::EarlOsam(fight) => fight.retire(runtime),
            Self::LordAzaram(fight) => fight.retire(runtime),
            Self::Sirs(fight) => fight.retire(runtime),
            Self::Zelos(fight) => fight.retire(runtime),
        }
    }

    fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now: Option<u64>,
        cells: &crate::content::NativeEntryMovementCells,
    ) -> Result<bool, CarrierError> {
        let at = |runtime: &ChannelRuntimeV1, at| placement(cells, runtime, at);
        let relocate = |runtime: &mut ChannelRuntimeV1, actor, at| {
            if !placement(cells, runtime, at) || runtime.position_occupied_by_other(actor, at)? {
                return Err(CarrierError::WrongScope);
            }
            let before = runtime.read_actor_position(actor)?;
            runtime
                .borrow_movement_position()
                .commit_creature_relocation(actor, before, at)
                .map(|_| ())
        };
        match self {
            Self::Cults(fight) => fight.advance(runtime, at).map(|_| true),
            Self::CountVlarkorth(_) | Self::DukeKrule(_) => Err(CarrierError::PlanConflict),
            Self::EarlOsam(fight) => match now {
                Some(now) => fight.advance(runtime, now, at, relocate),
                None => Ok(true),
            },
            Self::LordAzaram(fight) => match now {
                Some(now) => fight.advance(runtime, now, at),
                None => Ok(true),
            },
            Self::Sirs(fight) => match now {
                Some(now) => fight.advance(runtime, now, relocate),
                None => Ok(true),
            },
            Self::Zelos(fight) => match now {
                Some(now) => {
                    fight.advance(runtime, now, |runtime, at| placement(cells, runtime, at))
                }
                None => Ok(true),
            },
        }
    }
}

#[derive(Default, Debug)]
pub(super) struct Registry {
    pub(super) active: Mutex<Vec<ActivePhase>>,
}

impl Registry {
    pub(super) fn next_due_us(&self) -> Option<u64> {
        self.active
            .lock()
            .ok()?
            .iter()
            .filter_map(|phase| {
                phase.victory_exit_us.or_else(|| match &phase.fight {
                    Fight::CountVlarkorth(fight) => Some(fight.next_due_us()),
                    Fight::Cults(_) => None,
                    Fight::DukeKrule(fight) => Some(fight.next_due_us()),
                    Fight::EarlOsam(fight) => Some(fight.next_due_us()),
                    Fight::LordAzaram(fight) => Some(fight.next_due_us()),
                    Fight::Sirs(fight) => Some(fight.next_due_us()),
                    Fight::Zelos(fight) => Some(fight.next_due_us()),
                })
            })
            .min()
    }

    fn zelos_vortex(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        at: MovementLocalPosition,
        now: u64,
    ) -> Option<Option<MovementLocalPosition>> {
        self.active.lock().ok()?.iter().find_map(|phase| {
            let (Fight::Zelos(zelos), Some(room)) = (&phase.fight, &phase.timed_room) else {
                return None;
            };
            let (north_gate, north_arrival) = room.north_vortex?;
            if !zelos.is_current(runtime) || zelos.timeout_participants(now).is_some() {
                return None;
            }
            if north_gate == at {
                return Some(
                    zelos
                        .north_gate_open(runtime, actor, session, now)
                        .then_some(north_arrival),
                );
            }
            room.corridors
                .iter()
                .find_map(|&(from, to)| (from == at).then_some(Some(to)))
        })
    }

    pub(super) fn timed_lever_busy(&self, key: u64) -> bool {
        self.active.lock().map_or(true, |active| {
            active
                .iter()
                .any(|phase| phase.key == key && phase.timed_room.is_some())
        })
    }

    pub(super) fn reserve<'a>(
        &'a self,
        runtime: &mut ChannelRuntimeV1,
        key: u64,
    ) -> Result<std::sync::MutexGuard<'a, Vec<ActivePhase>>, CarrierError> {
        let mut active = self.active.lock().map_err(|_| CarrierError::PlanConflict)?;
        active.retain_mut(|phase| {
            if !phase.fight.is_current(runtime) {
                return false;
            }
            if phase.victory_exit_us.is_some() {
                return true;
            }
            match phase.fight.victors(runtime) {
                Ok(Some(_)) => {
                    // Credit was captured in the lethal turn. Release the lever
                    // only after exact terminal decorations retire; tick retries failures.
                    let ActivePhase { fight, cells, .. } = phase;
                    fight.advance(runtime, None, cells).is_err()
                }
                Ok(None) => phase
                    .frozen
                    .as_ref()
                    .is_none_or(|(actor, _)| runtime.retains_corpse_projection(*actor)),
                Err(_) => false,
            }
        });
        if active.len() >= MAX_ENCOUNTERS || active.iter().any(|p| p.key == key) {
            return Err(CarrierError::PlanConflict);
        }
        active
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        Ok(active)
    }

    pub(super) fn note_projected_death(
        &self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        now: u64,
    ) {
        let Ok(mut active) = self.active.lock() else {
            return;
        };
        for phase in active.iter_mut().filter(|p| p.fight.contains(actor)) {
            if phase.fight.timeout_participants(now).is_some() {
                continue;
            }
            let Ok(is_new_death) = phase.fight.projected_death(runtime, actor) else {
                continue;
            };
            if is_new_death && phase.fight.owns_scoped_credit(actor) {
                phase.scoped_deaths.push(actor);
            }
            if phase.timed_room.is_some() {
                if phase
                    .fight
                    .victors(runtime)
                    .is_ok_and(|group| group.is_some())
                {
                    phase
                        .victory_exit_us
                        .get_or_insert(now.saturating_add(60_000_000));
                }
            } else {
                let ActivePhase { fight, cells, .. } = phase;
                let _ = fight.advance(runtime, None, cells);
            }
        }
        // Keep terminal admission credit until the same lethal turn captures it.
        // The next reservation releases a completed lever key independently of
        // corpse lifetime, after this synchronous death publication finishes.
    }

    pub(super) fn has_terminal(&self, runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> bool {
        self.active.lock().is_ok_and(|active| {
            active.iter().any(|phase| {
                phase.fight.contains_terminal(actor)
                    && phase
                        .fight
                        .victors(runtime)
                        .is_ok_and(|victors| victors.is_some())
            })
        })
    }

    pub(super) fn capture_terminal(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        entry: &mut PendingKillSettlement,
    ) {
        let Ok(mut active) = self.active.lock() else {
            return;
        };
        let target = entry.target;
        if let Some(phase) = active
            .iter()
            .find(|phase| phase.fight.owns_scoped_credit(target))
        {
            if !phase.scoped_deaths.contains(&target)
                && let Some(frozen) = &mut entry.grizzly
            {
                use crate::durability::quest_state::native_kills::NativeDeathSource;
                std::sync::Arc::make_mut(frozen).native.retain(|recipient| {
                    !matches!(recipient.death.source, NativeDeathSource::Encounter(_))
                });
            }
            return;
        }

        if let Some(phase) = active
            .iter_mut()
            .find(|p| matches!(p.fight, Fight::Cults(_)) && p.fight.contains_terminal(target))
        {
            if phase.fight.victors(runtime).ok().flatten().is_none() {
                return;
            }
            let (_, recipients) = phase.frozen.get_or_insert_with(|| {
                (
                    entry.target,
                    grizzly_kills::freeze_encounter_credit(
                        runtime,
                        states,
                        &entry.creature,
                        &phase.credits,
                        &entry.damage_shares,
                    ),
                )
            });
            grizzly_kills::replace_encounter_native_credit(entry, recipients.clone());
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "shared source admission binds the owner clock with the map and roster"
)]
pub(super) fn bind(
    runtime: &mut ChannelRuntimeV1,
    callback: &Callback,
    key: u64,
    actors: &[ExactActorRef],
    players: &[(ExactActorRef, GameSessionId)],
    credits: Vec<grizzly_kills::EncounterCredit>,
    cells: crate::content::NativeEntryMovementCells,
    now: u64,
) -> Result<ActivePhase, CarrierError> {
    let encounter = callback
        .effects
        .encounter
        .as_ref()
        .ok_or(CarrierError::PlanConflict)?;
    let local =
        |at: oteryn_gameplay_quests::quest::world_callbacks::Position| MovementLocalPosition {
            x: i32::from(at.x),
            y: i32::from(at.y),
            floor: -i16::from(at.floor),
        };
    let make_room = |exit, north_vortex| -> Result<TimedRoom, CarrierError> {
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            cells.index()
        else {
            return Err(CarrierError::PlanConflict);
        };
        let facts = &index.map().facts;
        let [off, _] = callback.lever.ok_or(CarrierError::PlanConflict)?;
        let reset = facts
            .quest_callback_replacement(key, off)
            .ok_or(CarrierError::PlanConflict)?;
        let started_revision = facts
            .object_revision(key)
            .checked_add(1)
            .ok_or(CarrierError::PlanConflict)?;
        Ok(TimedRoom {
            exit: local(exit),
            north_vortex,
            corridors: Vec::new(),
            min: local(encounter.room_min),
            max: local(encounter.room_max),
            lever: PhaseLever {
                key,
                started_revision,
                reset,
            },
        })
    };
    let timed = match encounter.phase {
        Some(EncounterPhase::GraveDangerCountVlarkorth { exit }) => {
            let room = make_room(exit, None)?;
            if room.contains(room.exit) || !placement(&cells, runtime, room.exit) {
                return Err(CarrierError::PlanConflict);
            }
            Some((
                Fight::CountVlarkorth(CountVlarkorthEncounter::bind(
                    runtime, players, actors, now,
                )?),
                room,
            ))
        }
        Some(EncounterPhase::GraveDangerDukeKrule { exit }) => {
            let room = make_room(exit, None)?;
            if room.contains(room.exit) || !placement(&cells, runtime, room.exit) {
                return Err(CarrierError::PlanConflict);
            }
            Some((
                Fight::DukeKrule(DukeKruleEncounter::bind(runtime, players, actors, now)?),
                room,
            ))
        }
        Some(EncounterPhase::GraveDangerEarlOsam { exit, centre }) => {
            let room = make_room(exit, None)?;
            let centre = local(centre);
            if room.contains(room.exit)
                || !placement(&cells, runtime, room.exit)
                || !std::iter::once(centre)
                    .chain(earl_osam::sphere_positions(centre))
                    .all(|at| room.contains(at) && placement(&cells, runtime, at))
            {
                return Err(CarrierError::PlanConflict);
            }
            Some((
                Fight::EarlOsam(EarlOsamEncounter::bind(
                    runtime, players, actors, centre, now,
                )?),
                room,
            ))
        }
        Some(EncounterPhase::GraveDangerLordAzaram { exit, splinters }) => {
            let room = make_room(exit, None)?;
            let positions = splinters.map(local);
            if room.contains(room.exit)
                || !placement(&cells, runtime, room.exit)
                || !positions
                    .iter()
                    .all(|&at| room.contains(at) && placement(&cells, runtime, at))
            {
                return Err(CarrierError::PlanConflict);
            }
            Some((
                Fight::LordAzaram(LordAzaramEncounter::bind(
                    runtime, players, actors, positions, now,
                )?),
                room,
            ))
        }
        Some(EncounterPhase::GraveDangerSirs { exit, fight }) => {
            let room = make_room(exit, None)?;
            let fight = fight.map(local);
            if room.contains(room.exit)
                || !placement(&cells, runtime, room.exit)
                || !fight
                    .into_iter()
                    .all(|at| room.contains(at) && placement(&cells, runtime, at))
            {
                return Err(CarrierError::PlanConflict);
            }
            Some((
                Fight::Sirs(SirsEncounter::bind(runtime, players, actors, fight, now)?),
                room,
            ))
        }
        Some(EncounterPhase::GraveDangerZelos {
            exit,
            north_gate,
            north_arrival,
            passages,
        }) => {
            let gate = local(north_gate);
            let arrival = local(north_arrival);
            let mut room = make_room(exit, Some((gate, arrival)))?;
            if let Some(passages) = passages {
                let entrance = encounter
                    .players
                    .first()
                    .ok_or(CarrierError::PlanConflict)?
                    .to;
                let routes = passages
                    .routes(entrance)
                    .ok_or(CarrierError::PlanConflict)?;
                for (from, to) in routes {
                    let from = local(from);
                    let to = local(to);
                    if !room.contains(from)
                        || !room.contains(to)
                        || !placement(&cells, runtime, from)
                    {
                        return Err(CarrierError::PlanConflict);
                    }
                    room.corridors.push((from, to));
                }
            }
            if room.contains(room.exit)
                || !room.contains(gate)
                || !room.contains(arrival)
                || gate == arrival
                || ![room.exit, gate, arrival]
                    .into_iter()
                    .all(|at| placement(&cells, runtime, at))
            {
                return Err(CarrierError::PlanConflict);
            }
            Some((
                Fight::Zelos(ZelosEncounter::bind(runtime, players, actors, now)?),
                room,
            ))
        }
        _ => None,
    };
    if let Some((fight, room)) = timed {
        return Ok(ActivePhase {
            key,
            fight,
            credits,
            frozen: None,
            cells,
            timed_room: Some(room),
            victory_exit_us: None,
            scoped_deaths: Vec::new(),
        });
    }
    if encounter.phase != Some(EncounterPhase::Cults) || actors.len() != 10 {
        return Err(CarrierError::PlanConflict);
    }
    let plan = cults::plan();
    let mut pillars = [actors[0]; 5];
    let mut stops = [actors[0]; 5];
    for i in 0..5 {
        pillars[i] = *actors
            .iter()
            .find(|a| {
                runtime.matches_live_creature_identity(**a, plan.roles[i].creature.as_bytes())
            })
            .ok_or(CarrierError::CreatureTargetMismatch)?;
        let key = format!("{}_stop", plan.roles[i + 5].creature);
        stops[i] = *actors
            .iter()
            .find(|a| runtime.matches_live_creature_identity(**a, key.as_bytes()))
            .ok_or(CarrierError::CreatureTargetMismatch)?;
    }
    // The captured source uses these offsets from Essence, in the current map frame.
    let death = runtime.read_actor_position(pillars[1])?.position();
    let centre = MovementLocalPosition {
        x: death.x,
        y: death.y.checked_add(5).ok_or(CarrierError::PlanConflict)?,
        floor: death.floor,
    };
    let pillars_offsets = [(-5, -1), (0, -5), (5, -1), (3, 5), (-3, 5)];
    let stops_offsets = [(-4, -1), (0, -4), (4, -1), (3, 4), (-3, 4)];
    for (group, offsets) in [(&pillars, pillars_offsets), (&stops, stops_offsets)] {
        for (actor, (dx, dy)) in group.iter().zip(offsets) {
            let at = runtime.read_actor_position(*actor)?.position();
            if at.floor != centre.floor
                || at.x.checked_sub(centre.x) != Some(dx)
                || at.y.checked_sub(centre.y) != Some(dy)
            {
                return Err(CarrierError::PlanConflict);
            }
        }
    }
    if !(i32::from(encounter.room_min.x)..=i32::from(encounter.room_max.x)).contains(&centre.x)
        || !(i32::from(encounter.room_min.y)..=i32::from(encounter.room_max.y)).contains(&centre.y)
    {
        return Err(CarrierError::PlanConflict);
    }
    if !placement(&cells, runtime, centre) {
        return Err(CarrierError::PlanConflict);
    }
    Ok(ActivePhase {
        key,
        fight: Fight::Cults(CultsEncounter::bind(
            runtime, players, pillars, stops, centre,
        )?),
        credits,
        frozen: None,
        cells,
        timed_room: None,
        victory_exit_us: None,
        scoped_deaths: Vec::new(),
    })
}

fn placement(
    cells: &crate::content::NativeEntryMovementCells,
    runtime: &ChannelRuntimeV1,
    at: MovementLocalPosition,
) -> bool {
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        cells.index()
    else {
        return false;
    };
    let (Ok(x), Ok(y), Ok(floor)) = (
        u16::try_from(at.x),
        u16::try_from(at.y),
        i8::try_from(at.floor),
    ) else {
        return false;
    };
    let Some(floor) = floor.checked_neg() else {
        return false;
    };
    let map = index.map();
    cells.scope().world_id == runtime.content_pin().world_id()
        && cells.scope().generation_digest == runtime.content_pin().server_artifact_digest()
        && cells.index().lookup(
            cells.scope(),
            crate::content::LogicalCell {
                x: at.x,
                y: at.y,
                z: i32::from(at.floor),
            },
        ) == Ok(crate::content::CollisionClass::Walkable)
        && !map.facts.quest_callback_blocked(
            map.overlay.base(),
            crate::map::overlay::TilePos { x, y, floor },
            false,
        )
}

pub(super) fn tick(runtime: &mut ChannelRuntimeV1, states: &mut ChannelSpellStates, now: u64) {
    tick_phase_kind(runtime, states, now, None);
}

pub(super) fn tick_deadlines(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    now: u64,
    native: &crate::content::native_gameplay::NativeGameplayState,
) {
    tick_phase_kind(runtime, states, now, Some(native));
}

fn tick_phase_kind(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    now: u64,
    native: Option<&crate::content::native_gameplay::NativeGameplayState>,
) {
    let Ok(mut active) = states.encounter_phases.active.lock() else {
        return;
    };
    let mut exits = Vec::new();
    let mut duke_effects = Vec::new();
    let mut magnor_bursts = Vec::new();
    active.retain(|phase| phase.fight.is_current(runtime));
    active.retain_mut(|phase| {
        let ActivePhase {
            key,
            fight,
            cells,
            timed_room,
            victory_exit_us,
            ..
        } = phase;
        if native.is_some() != timed_room.is_some() {
            return true;
        }
        if let (Some(room), Some(due)) = (timed_room.as_ref(), *victory_exit_us) {
            if now < due {
                return true;
            }
            let Ok(Some(victors)) = fight.victors(runtime) else {
                return false;
            };
            return !evacuate(runtime, cells, room, victors, &mut exits)
                || fight.retire(runtime).is_err();
        }
        if let Some(room) = timed_room.as_ref() {
            if let Some(participants) = fight.timeout_participants(now) {
                return !evacuate(runtime, cells, room, participants, &mut exits)
                    || fight.retire(runtime).is_err();
            }
            if let Fight::CountVlarkorth(count) = fight {
                if let Err(error) = count.advance(
                    runtime,
                    now,
                    |at| room.contains(at),
                    |runtime, at| placement(cells, runtime, at),
                    |runtime, actor, session| {
                        use oteryn_gameplay_spell::spell::Vocation::*;
                        std::option::Option::Some(
                            match states
                                .get(runtime, actor, session)?
                                .character_facts()
                                .vocation
                            {
                                Druid | ElderDruid => "oteryn:creature.dark_druid",
                                Knight | EliteKnight => "oteryn:creature.dark_knight",
                                Monk | ExaltedMonk => "oteryn:creature.dark_merudri",
                                Paladin | RoyalPaladin => "oteryn:creature.dark_paladin",
                                Sorcerer | MasterSorcerer => "oteryn:creature.dark_sorcerer",
                                None => return std::option::Option::None,
                            },
                        )
                    },
                ) {
                    super::operator_event(&format!(
                        "encounter_phase_deferred key={key} reason={error:?}"
                    ));
                }
                return true;
            }
            if let Fight::DukeKrule(duke) = fight {
                if let Ok((source, effects)) =
                    duke.due_effects(runtime, now, |at| room.contains(at))
                {
                    duke_effects.extend(effects.into_iter().map(|effect| (source, effect)));
                }
                return true;
            }
        }
        if let Fight::Zelos(zelos) = fight {
            magnor_bursts.extend(zelos.take_magnor_bursts());
        }
        let osam_push = match fight {
            Fight::EarlOsam(osam) => osam
                .finish_push(now)
                .map(|(centre, group)| (centre, group.to_vec())),
            _ => None,
        };
        match fight.advance(runtime, Some(now), cells) {
            Ok(false) => return false,
            Ok(true) => {}
            Err(error) => {
                super::operator_event(&format!(
                    "encounter_phase_deferred key={key} reason={error:?}"
                ));
                return true;
            }
        }
        if let (Some(room), Some((centre, players))) = (timed_room.as_ref(), osam_push) {
            push_osam_adjacent(runtime, cells, room, centre, &players, &mut exits);
        }
        fight
            .victors(runtime)
            .is_ok_and(|victors| victors.is_none())
    });
    drop(active);
    if let Some(native) = native {
        for (source, effect) in duke_effects {
            duke_krule::apply(runtime, states, native, source, effect, now);
        }
    }
    for burst in magnor_bursts {
        zelos::apply_magnor_burst(runtime, states, burst, now);
    }
    for (cells, actor, session, before, after) in exits {
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            cells.index()
        else {
            continue;
        };
        let map = index.map();
        let underwater = super::paced_movement::underwater_drown::underwater;
        states.stage_special_movement_step(
            runtime,
            map,
            actor,
            session,
            now,
            Some(before),
            after,
            None,
        );
        states.stage_underwater_step(
            runtime,
            actor,
            session,
            now,
            underwater(map, before),
            underwater(map, after),
        );
    }
}

type EvacuatedPlayer = (
    crate::content::NativeEntryMovementCells,
    ExactActorRef,
    GameSessionId,
    MovementLocalPosition,
    MovementLocalPosition,
);

// Recheck each exact session and its current location before forced room exit.
fn evacuate(
    runtime: &mut ChannelRuntimeV1,
    cells: &crate::content::NativeEntryMovementCells,
    exit: &TimedRoom,
    participants: &[(ExactActorRef, GameSessionId)],
    exits: &mut Vec<EvacuatedPlayer>,
) -> bool {
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        cells.index()
    else {
        return false;
    };
    let map = index.map();
    let closed = crate::map::floor::BundleDoors(&map.facts);
    let mut moves = Vec::new();
    let mut proofs = Vec::new();
    for &(actor, session) in participants {
        if runtime.player_control_facts(actor, session).is_err() {
            continue;
        }
        let Ok(before) = runtime.read_actor_position(actor) else {
            return false;
        };
        if !exit.contains(before.position()) {
            continue;
        }
        let Ok(proof) = crate::map::floor::prepare_quest_encounter_ejection(
            runtime, cells, actor, session, before, exit.exit, &closed,
        ) else {
            return false;
        };
        moves.push((actor, session, before.position()));
        proofs.push(proof);
    }
    let lever = &exit.lever;
    let Ok(snapshots) = runtime.commit_quest_relocation(proofs, || {
        // A successful retry may find the previous ejection already published.
        if map.facts.object_revision(lever.key) != lever.started_revision {
            return true;
        }
        map.facts.apply_quest_callback(
            lever.key,
            lever.started_revision,
            lever.reset,
            &[],
            &[],
            true,
            None,
        )
    }) else {
        return false;
    };
    for ((actor, session, before), after) in moves.into_iter().zip(snapshots) {
        exits.push((cells.clone(), actor, session, before, after.position()));
    }
    true
}

// Crystal 201189f grave_danger_earl_osam.lua:39–65.
fn push_osam_adjacent(
    runtime: &mut ChannelRuntimeV1,
    cells: &crate::content::NativeEntryMovementCells,
    room: &TimedRoom,
    centre: MovementLocalPosition,
    players: &[(ExactActorRef, GameSessionId)],
    exits: &mut Vec<EvacuatedPlayer>,
) {
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        cells.index()
    else {
        return;
    };
    let doors = crate::map::floor::BundleDoors(&index.map().facts);
    for &(actor, session) in players {
        let Ok(before) = runtime.read_actor_position(actor) else {
            continue;
        };
        let at = before.position();
        if at.floor != centre.floor
            || at
                .x
                .abs_diff(centre.x)
                .saturating_add(at.y.abs_diff(centre.y))
                != 1
        {
            continue;
        }
        let landings = [
            (0, 3),
            (3, 0),
            (0, -3),
            (-3, 0),
            (3, 3),
            (-3, -3),
            (4, -4),
            (-4, 4),
        ];
        for (dx, dy) in landings {
            let landing = MovementLocalPosition {
                x: centre.x + dx,
                y: centre.y + dy,
                ..centre
            };
            if room.contains(landing)
                && let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
                    runtime, cells, actor, session, before, landing, false, &doors,
                )
                && let Ok(after) = runtime.commit_source_step(proof)
            {
                exits.push((cells.clone(), actor, session, at, after.position()));
                break;
            }
        }
    }
}

impl super::ComposedFreshAdmission<'_, '_, '_> {
    /// Crystal 201189f grave_danger_king_zelos.lua:309–374.
    pub(super) async fn step_zelos_vortex(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> Option<MovementPositionSnapshot> {
        let map = self.bundle_map()?;
        let now = self.owner_now();
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let arrival = states.encounter_phases.zelos_vortex(
            &runtime,
            actor,
            session,
            entered.position(),
            now.get(),
        )?;
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let blocked = super::pvp_live::pz_blocked(&states, actor, session, now);
        let destination = arrival.unwrap_or_else(|| origin.position());
        let Ok(proof) = crate::map::floor::prepare_quest_encounter_admission(
            &runtime,
            self.movement_cells(),
            actor,
            session,
            entered,
            destination,
            blocked,
            &closed,
            true,
        ) else {
            return Some(entered);
        };
        let Ok(landed) = runtime.commit_source_step(proof) else {
            return Some(entered);
        };
        let underwater = super::paced_movement::underwater_drown::underwater;
        states.stage_special_movement_step(
            &runtime,
            map,
            actor,
            session,
            now.get(),
            Some(entered.position()),
            landed.position(),
            None,
        );
        states.stage_underwater_step(
            &runtime,
            actor,
            session,
            now.get(),
            underwater(map, entered.position()),
            underwater(map, landed.position()),
        );
        if arrival.is_none() {
            self.states.chat.push_quest_feedback(
                session,
                "The ritual of the Lich Knights still shields the king. Defeat all four of them first.",
            );
        }
        Some(landed)
    }
}
