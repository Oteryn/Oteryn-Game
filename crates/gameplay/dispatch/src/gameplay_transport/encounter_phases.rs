//! One phase registry on the existing encounter, native death queue and quest writer.
use super::{actor_spell::ChannelSpellStates, grizzly_kills, kill_reward::PendingKillSettlement};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use oteryn_gameplay_encounters::bosses::phase_controller::cults::{self, CultsEncounter};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, EncounterPhase};
use oteryn_gameplay_world::map::boot::BundleCollisionIndexExt;
use std::sync::Mutex;

mod crystal_circuit;
mod replacements;
mod terminal_portal;
mod terminal_transfer;

#[derive(Debug)]
enum Fight {
    Cults(Box<CultsEncounter>),
    Replacements(Box<replacements::Replacements>),
}

impl Fight {
    fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        match self {
            Self::Cults(fight) => fight.is_current(runtime),
            Self::Replacements(fight) => fight.is_current(runtime),
        }
    }

    fn contains(&self, actor: ExactActorRef) -> bool {
        match self {
            Self::Cults(fight) => fight.contains(actor),
            Self::Replacements(fight) => fight.contains(actor),
        }
    }

    fn contains_terminal(&self, actor: ExactActorRef) -> bool {
        matches!(self, Self::Cults(fight) if fight.contains_terminal(actor))
    }

    fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        now_us: u64,
        cells: &crate::content::NativeEntryMovementCells,
    ) -> Result<bool, CarrierError> {
        match self {
            Self::Cults(fight) => fight.projected_death(runtime, actor),
            Self::Replacements(fight) => fight.projected_death(runtime, actor, now_us, cells),
        }
    }

    fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &crate::content::NativeEntryMovementCells,
    ) -> Result<(), CarrierError> {
        match self {
            Self::Cults(fight) => {
                fight.advance(runtime, |runtime, at| placement(cells, runtime, at))
            }
            Self::Replacements(fight) => fight.advance(runtime, cells),
        }
    }

    fn victors(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&[(ExactActorRef, GameSessionId)]>, CarrierError> {
        match self {
            Self::Cults(fight) => fight.victors(runtime),
            Self::Replacements(fight) => fight.victors(runtime),
        }
    }
}

const MAX_ENCOUNTERS: usize = 64;

#[derive(Debug)]
pub(super) struct ActivePhase {
    key: u64,
    fight: Fight,
    room: oteryn_gameplay_quests::quest::world_callbacks::Encounter,
    participants: Vec<(ExactActorRef, GameSessionId)>,
    credits: Vec<grizzly_kills::EncounterCredit>,
    frozen: Option<(ExactActorRef, Vec<super::quest_kills::Recipient>)>,
    cells: crate::content::NativeEntryMovementCells,
}

#[derive(Default, Debug)]
pub(super) struct Registry {
    pub(super) active: Mutex<Vec<ActivePhase>>,
}

impl Registry {
    pub(super) fn replace_member(
        &self,
        runtime: &ChannelRuntimeV1,
        from: ExactActorRef,
        to: ExactActorRef,
        creature: &str,
    ) {
        if let Ok(mut active) = self.active.lock() {
            for phase in active.iter_mut().filter(|phase| phase.fight.contains(from)) {
                if let Fight::Replacements(fight) = &mut phase.fight {
                    fight.replace_member(runtime, from, to, creature);
                }
            }
        }
    }

    pub(super) fn blocks_channel_switch(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        position: MovementLocalPosition,
    ) -> bool {
        self.active.lock().map_or(true, |phases| {
            phases.iter().any(|phase| {
                phase.participants.contains(&(actor, session))
                    && super::quest_callbacks::room_contains(&phase.room, position)
                    && phase.fight.is_current(runtime)
                    && phase
                        .fight
                        .victors(runtime)
                        .is_ok_and(|victors| victors.is_none())
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
                    matches!(fight, Fight::Replacements(fight) if fight.completion_pending())
                        || fight.advance(runtime, cells).is_err()
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
        creature: Option<&str>,
        now_us: u64,
        owner_wake: &tokio::sync::Notify,
    ) {
        let Ok(mut active) = self.active.lock() else {
            return;
        };
        if !active.iter().any(|phase| phase.fight.contains(actor))
            && let Some(creature) = creature
            && let Ok((_, at)) = runtime.borrow_combat_death().projected_death(actor)
        {
            // Crystal's Essence and Ferumbras callbacks apply to creatures summoned after entry.
            // An exact admitted actor wins over a region; overlapping regions never guess.
            let mut regional = active
                .iter_mut()
                .filter_map(|phase| match &mut phase.fight {
                    Fight::Replacements(fight)
                        if fight.accepts_regional_death(runtime, creature, at) =>
                    {
                        Some(fight)
                    }
                    _ => None,
                });
            if let Some(fight) = regional.next()
                && regional.next().is_none()
            {
                let _ = fight.record_regional_member(actor, creature);
            }
        }
        for phase in active.iter_mut().filter(|p| p.fight.contains(actor)) {
            if phase
                .fight
                .projected_death(runtime, actor, now_us, &phase.cells)
                .is_ok()
            {
                let ActivePhase { fight, cells, .. } = phase;
                let _ = fight.advance(runtime, cells);
                if let Fight::Replacements(fight) = fight {
                    fight.project_portal(cells, now_us);
                    if let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(
                        index,
                    ) = cells.index()
                        && crate::map::boot::BundleCollisionIndexExt::map(index)
                            .facts
                            .next_movement_decay_time()
                            .is_some()
                    {
                        owner_wake.notify_one();
                    }
                }
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
            .find(|p| p.fight.contains_terminal(target))
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

pub(super) fn bind(
    runtime: &mut ChannelRuntimeV1,
    callback: &Callback,
    key: u64,
    actors: &[ExactActorRef],
    players: &[(ExactActorRef, GameSessionId)],
    credits: Vec<grizzly_kills::EncounterCredit>,
    cells: crate::content::NativeEntryMovementCells,
) -> Result<ActivePhase, CarrierError> {
    let encounter = callback
        .effects
        .encounter
        .as_ref()
        .ok_or(CarrierError::PlanConflict)?;
    if let Some(plan) = &encounter.death_phases {
        let blocked =
            |destination| !placement(&cells, runtime, super::quest_callbacks::local(destination));
        if plan
            .terminal_transfer
            .is_some_and(|transfer| blocked(transfer.destination))
            || plan
                .corpse_portal
                .as_ref()
                .is_some_and(|portal| blocked(portal.destination))
        {
            return Err(CarrierError::PlanConflict);
        }
        return Ok(ActivePhase {
            key,
            fight: Fight::Replacements(Box::new(replacements::Replacements::bind(
                runtime, encounter, plan, actors, players,
            )?)),
            room: encounter.clone(),
            participants: players.to_vec(),
            credits,
            frozen: None,
            cells,
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
        fight: Fight::Cults(Box::new(CultsEncounter::bind(
            runtime, players, pillars, stops, centre,
        )?)),
        room: encounter.clone(),
        participants: players.to_vec(),
        credits,
        frozen: None,
        cells,
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

pub(super) fn tick(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    map: Option<&crate::map::boot::BundleMap>,
    now_us: u64,
) {
    let Ok(mut active) = states.encounter_phases.active.lock() else {
        return;
    };
    let mut transfers = Vec::new();
    active.retain(|phase| phase.fight.is_current(runtime));
    active.retain_mut(|phase| {
        let ActivePhase {
            key, fight, cells, ..
        } = phase;
        if let Err(error) = fight.advance(runtime, cells) {
            super::operator_event(&format!(
                "encounter_phase_deferred key={key} reason={error:?}"
            ));
            return true;
        }
        if let Fight::Replacements(fight) = fight {
            fight.project_portal(cells, now_us);
            if fight.transfer_pending()
                && let Some(map) = map
            {
                fight.project_transfer(runtime, states, cells, map, &mut transfers);
            }
            if fight.completion_pending() {
                return true;
            }
        }
        fight
            .victors(runtime)
            .is_ok_and(|victors| victors.is_none())
    });
    drop(active);
    if let Some(map) = map {
        for movement in transfers {
            movement.publish(runtime, states, map, now_us);
        }
    }
}
