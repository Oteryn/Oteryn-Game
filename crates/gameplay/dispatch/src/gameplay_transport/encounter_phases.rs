//! One phase registry on the existing encounter, native death queue and quest writer.
use super::{actor_spell::ChannelSpellStates, grizzly_kills, kill_reward::PendingKillSettlement};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
    MovementPositionSnapshot,
};
use oteryn_gameplay_encounters::bosses::phase_controller::cults::{self, CultsEncounter};
use oteryn_gameplay_encounters::bosses::phase_controller::sirs::SirsEncounter;
use oteryn_gameplay_encounters::bosses::phase_controller::zelos::ZelosEncounter;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, EncounterPhase};
use oteryn_gameplay_world::map::boot::BundleCollisionIndexExt;
use std::sync::Mutex;

const MAX_ENCOUNTERS: usize = 64;

#[derive(Debug)]
pub(super) struct ActivePhase {
    key: u64,
    fight: Fight,
    credits: Vec<grizzly_kills::EncounterCredit>,
    frozen: Option<(ExactActorRef, Vec<super::quest_kills::Recipient>)>,
    cells: crate::content::NativeEntryMovementCells,
    timed_room: Option<TimedRoom>,
}

#[derive(Debug)]
struct TimedRoom {
    exit: MovementLocalPosition,
    north_vortex: Option<(MovementLocalPosition, MovementLocalPosition)>,
    min: MovementLocalPosition,
    max: MovementLocalPosition,
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
    Cults(CultsEncounter),
    Sirs(SirsEncounter),
    Zelos(ZelosEncounter),
}

impl Fight {
    fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        match self {
            Self::Cults(fight) => fight.is_current(runtime),
            Self::Sirs(fight) => fight.is_current(runtime),
            Self::Zelos(fight) => fight.is_current(runtime),
        }
    }

    fn contains(&self, actor: ExactActorRef) -> bool {
        match self {
            Self::Cults(fight) => fight.contains(actor),
            Self::Sirs(fight) => fight.contains(actor),
            Self::Zelos(fight) => fight.contains(actor),
        }
    }

    fn contains_terminal(&self, actor: ExactActorRef) -> bool {
        match self {
            Self::Cults(fight) => fight.contains_terminal(actor),
            Self::Sirs(_) => false,
            Self::Zelos(fight) => fight.contains_terminal(actor),
        }
    }

    fn victors(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&[(ExactActorRef, GameSessionId)]>, CarrierError> {
        match self {
            Self::Cults(fight) => fight.victors(runtime),
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
            Self::Sirs(fight) => fight.projected_death(runtime, actor),
            Self::Zelos(fight) => fight.projected_death(runtime, actor),
        }
    }

    fn timeout_participants(&self, now: u64) -> Option<&[(ExactActorRef, GameSessionId)]> {
        match self {
            Self::Cults(_) => None,
            Self::Sirs(fight) => fight.timeout_participants(now),
            Self::Zelos(fight) => fight.timeout_participants(now),
        }
    }

    fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now: Option<u64>,
        cells: &crate::content::NativeEntryMovementCells,
    ) -> Result<bool, CarrierError> {
        match self {
            Self::Cults(fight) => fight
                .advance(runtime, |runtime, at| placement(cells, runtime, at))
                .map(|_| true),
            Self::Sirs(fight) => match now {
                Some(now) => fight.advance(runtime, now, |runtime, actor, at| {
                    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(
                        index,
                    ) = cells.index()
                    else {
                        return Err(CarrierError::PlanConflict);
                    };
                    let doors = crate::map::floor::BundleDoors(&index.map().facts);
                    crate::movement::relocate::relocate_creature(runtime, actor, cells, &doors, at)
                        .map(|_| ())
                }),
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
            .filter_map(|phase| match &phase.fight {
                Fight::Sirs(fight) => Some(fight.next_due_us()),
                Fight::Zelos(fight) => Some(fight.next_due_us()),
                Fight::Cults(_) => None,
            })
            .min()
    }

    fn north_gate(
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
            (north_gate == at && zelos.is_current(runtime)).then(|| {
                zelos
                    .north_gate_open(runtime, actor, session, now)
                    .then_some(north_arrival)
            })
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
            if phase.fight.projected_death(runtime, actor).is_ok()
                && matches!(phase.fight, Fight::Cults(_))
            {
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
    let make_room = |exit, north_vortex| TimedRoom {
        exit: local(exit),
        north_vortex,
        min: local(encounter.room_min),
        max: local(encounter.room_max),
    };
    let timed = match encounter.phase {
        Some(EncounterPhase::GraveDangerSirs { exit, fight }) => {
            let room = make_room(exit, None);
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
        }) => {
            let gate = local(north_gate);
            let arrival = local(north_arrival);
            let room = make_room(exit, Some((gate, arrival)));
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
    tick_phase_kind(runtime, states, now, false);
}

pub(super) fn tick_deadlines(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    now: u64,
) {
    tick_phase_kind(runtime, states, now, true);
}

fn tick_phase_kind(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    now: u64,
    timed: bool,
) {
    let Ok(mut active) = states.encounter_phases.active.lock() else {
        return;
    };
    let mut exits = Vec::new();
    active.retain(|phase| phase.fight.is_current(runtime));
    active.retain_mut(|phase| {
        let ActivePhase {
            key,
            fight,
            cells,
            timed_room,
            ..
        } = phase;
        if timed != timed_room.is_some() {
            return true;
        }
        if let Some(room) = timed_room.as_ref()
            && let Some(participants) = fight.timeout_participants(now)
            && !evacuate(runtime, cells, room, participants, &mut exits)
        {
            return true;
        }
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
        fight
            .victors(runtime)
            .is_ok_and(|victors| victors.is_none())
    });
    drop(active);
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
    let Ok(snapshots) = runtime.commit_quest_relocation(proofs, || true) else {
        return false;
    };
    for ((actor, session, before), after) in moves.into_iter().zip(snapshots) {
        exits.push((cells.clone(), actor, session, before, after.position()));
    }
    true
}

impl super::ComposedFreshAdmission<'_, '_, '_> {
    /// Crystal 201189f grave_danger_king_zelos.lua:352–374.
    pub(super) async fn step_zelos_north_gate(
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
        let arrival = states.encounter_phases.north_gate(
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
