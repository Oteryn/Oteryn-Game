//! One phase registry on the existing encounter, native death queue and quest writer.
use super::{actor_spell::ChannelSpellStates, grizzly_kills, kill_reward::PendingKillSettlement};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use oteryn_gameplay_encounters::bosses::phase_controller::{
    court_warlock::CourtWarlock,
    cults::{self, CultsEncounter},
};
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
}

#[derive(Debug)]
enum Fight {
    CourtWarlock(Box<CourtWarlock>),
    Cults(Box<CultsEncounter>),
}
impl Fight {
    fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        match self {
            Self::Cults(f) => f.is_current(runtime),
            Self::CourtWarlock(f) => f.is_current(runtime),
        }
    }
    fn contains(&self, actor: ExactActorRef) -> bool {
        match self {
            Self::Cults(f) => f.contains(actor),
            Self::CourtWarlock(f) => f.contains(actor),
        }
    }
    fn contains_terminal(&self, actor: ExactActorRef) -> bool {
        match self {
            Self::Cults(f) => f.contains_terminal(actor),
            Self::CourtWarlock(f) => f.contains_terminal(actor),
        }
    }
    fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        match self {
            Self::Cults(f) => f.projected_death(runtime, actor),
            Self::CourtWarlock(f) => f.projected_death(runtime, actor),
        }
    }
    fn victors(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&[(ExactActorRef, GameSessionId)]>, CarrierError> {
        match self {
            Self::Cults(f) => f.victors(runtime),
            Self::CourtWarlock(f) => f.victors(runtime),
        }
    }
}

#[derive(Default, Debug)]
pub(super) struct Registry {
    pub(super) active: Mutex<Vec<ActivePhase>>,
}

impl Registry {
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
            if matches!(phase.fight, Fight::CourtWarlock(_)) {
                return true;
            }
            match phase.fight.victors(runtime) {
                Ok(Some(_)) => {
                    // Credit was captured in the lethal turn. Release the lever
                    // only after exact terminal decorations retire; tick retries failures.
                    let Fight::Cults(fight) = &mut phase.fight else {
                        return true;
                    };
                    fight
                        .advance(runtime, |runtime, at| placement(&phase.cells, runtime, at))
                        .is_err()
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
    ) {
        let Ok(mut active) = self.active.lock() else {
            return;
        };
        for phase in active.iter_mut().filter(|p| p.fight.contains(actor)) {
            if phase.fight.projected_death(runtime, actor).is_ok()
                && let Fight::Cults(fight) = &mut phase.fight
            {
                let _ = fight.advance(runtime, |runtime, at| placement(&phase.cells, runtime, at));
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
            // Court Warlock uses the source room-player recipient rule already
            // captured by quest_batch3_c, including non-damaging participants.
            if matches!(phase.fight, Fight::CourtWarlock(_)) {
                return;
            }
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
    reason = "the existing atomic admission binds its clock in the same owner turn"
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
    if encounter.phase == Some(EncounterPhase::CourtWarlock) {
        let local =
            |p: oteryn_gameplay_quests::quest::world_callbacks::Position| MovementLocalPosition {
                x: i32::from(p.x),
                y: i32::from(p.y),
                floor: -i16::from(p.floor),
            };
        let low = local(encounter.room_min);
        let high = local(encounter.room_max);
        let stale: Vec<_> = runtime
            .positioned_melee_census()?
            .into_iter()
            .filter(|(actor, at, session)| {
                let at = at.position();
                session.is_none()
                    && runtime.contains_live_creature(*actor)
                    && at.floor == low.floor
                    && (low.x..=high.x).contains(&at.x)
                    && (low.y..=high.y).contains(&at.y)
            })
            .map(|(actor, _, _)| actor)
            .collect();
        for actor in stale {
            runtime.remove_encounter_actor(actor)?;
        }
        let center = encounter
            .players
            .first()
            .ok_or(CarrierError::PlanConflict)?
            .to;
        return Ok(ActivePhase {
            key,
            fight: Fight::CourtWarlock(Box::new(CourtWarlock::bind(
                runtime,
                players,
                now,
                local(encounter.room_min),
                local(encounter.room_max),
                local(center),
            )?)),
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

pub(super) fn tick(runtime: &mut ChannelRuntimeV1, states: &ChannelSpellStates, now: u64) {
    let Ok(mut active) = states.encounter_phases.active.lock() else {
        return;
    };
    active.retain(|phase| phase.fight.is_current(runtime));
    active.retain_mut(|phase| {
        let ActivePhase {
            key, fight, cells, ..
        } = phase;
        if let Fight::CourtWarlock(fight) = fight {
            let result = (|| {
                let census = runtime.positioned_melee_census()?;
                let players: Vec<_> = census.into_iter().filter(|(_,at,s)| s.is_some() && fight.in_room(at.position())).collect();
                if !players.is_empty() && !fight.exit_due(now) {
                    fight.advance(runtime, now, |runtime, at| placement(cells, runtime, at))?;
                    return Ok(true);
                }
                let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) = cells.index() else { return Err(CarrierError::PlanConflict); };
                let closed = crate::map::floor::BundleDoors(&index.map().facts);
                let exit = oteryn_gameplay_quests::quest::world_callbacks::court_warlock::EXIT;
                let mut proofs = Vec::new();
                for (actor, expected, session) in players {
                    let session = session.ok_or(CarrierError::PlanConflict)?;
                    proofs.push(crate::map::floor::prepare_quest_encounter_ejection(runtime, cells, actor, session, expected,
                        MovementLocalPosition { x:i32::from(exit.x), y:i32::from(exit.y), floor:-i16::from(exit.floor) }, &closed).map_err(|_| CarrierError::PlanConflict)?);
                }
                runtime.commit_quest_relocation(proofs, || true)?;
                fight.retire(runtime)?;
                Ok(false)
            })();
            return match result { Ok(keep) => keep, Err(error) => { super::operator_event(&format!("encounter_phase_deferred key={key} reason={error:?}")); true } };
        }
        let Fight::Cults(fight) = fight else { return false; };
        if let Err(error) = fight.advance(runtime, |runtime, at| placement(cells, runtime, at)) {
            super::operator_event(&format!(
                "encounter_phase_deferred key={key} reason={error:?}"
            ));
            return true;
        }
        fight
            .victors(runtime)
            .is_ok_and(|victors| victors.is_none())
    });
}
