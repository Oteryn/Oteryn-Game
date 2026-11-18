//! One phase registry on the existing encounter, native death queue and quest writer.
use super::{actor_spell::ChannelSpellStates, grizzly_kills, kill_reward::PendingKillSettlement};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
    MovementPositionSnapshot, RuntimeScopeRefV1, ScopeOwnershipGeneration, SourceStepCommitProof,
};
use oteryn_gameplay_encounters::bosses::phase_controller::cults::{self, CultsEncounter};
use oteryn_gameplay_quests::quest::{
    court_warlock,
    world_callbacks::{Callback, EncounterPhase, Position},
};
use oteryn_gameplay_world::map::boot::BundleCollisionIndexExt;
use std::sync::Mutex;

const MAX_ENCOUNTERS: usize = 64;

#[derive(Debug)]
pub(super) struct ActivePhase {
    key: u64,
    fight: CultsEncounter,
    credits: Vec<grizzly_kills::EncounterCredit>,
    frozen: Option<(ExactActorRef, Vec<super::quest_kills::Recipient>)>,
    cells: crate::content::NativeEntryMovementCells,
}

#[derive(Default, Debug)]
pub(super) struct Registry {
    pub(super) active: Mutex<Vec<ActivePhase>>,
    court_warlock: Mutex<Option<CourtWarlockPhase>>,
}

#[derive(Debug)]
struct CourtWarlockPhase {
    owner: (RuntimeScopeRefV1, ScopeOwnershipGeneration),
    cells: crate::content::NativeEntryMovementCells,
    participants: Vec<ExactActorRef>,
    first_wave_at: Option<u64>,
    timeout_at: u64,
    raubritters: Vec<ExactActorRef>,
}

impl Registry {
    #[allow(
        clippy::too_many_arguments,
        reason = "the exact party proofs and lever CAS publish the phase together"
    )]
    pub(super) fn admit_court_warlock<P: SourceStepCommitProof>(
        &self,
        runtime: &mut ChannelRuntimeV1,
        cells: &crate::content::NativeEntryMovementCells,
        now: u64,
        key: u64,
        participants: Vec<ExactActorRef>,
        proofs: Vec<P>,
        project: impl FnOnce() -> bool,
    ) -> Result<Vec<MovementPositionSnapshot>, CarrierError> {
        if participants.is_empty()
            || participants.len() != proofs.len()
            || key != court_lever_key().ok_or(CarrierError::PlanConflict)?
            || court_warlock::RAUBRITTERS
                .iter()
                .any(|creature| runtime.companion_policy(creature).is_err())
        {
            return Err(CarrierError::PlanConflict);
        }
        let mut active = self
            .court_warlock
            .lock()
            .map_err(|_| CarrierError::PlanConflict)?;
        let owner = runtime.encounter_owner_scope();
        if active.as_ref().is_some_and(|phase| phase.owner == owner) {
            return Err(CarrierError::PlanConflict);
        }
        let phase = CourtWarlockPhase {
            owner,
            cells: cells.clone(),
            participants,
            first_wave_at: Some(
                now.checked_add(court_warlock::INITIAL_WAVE_US)
                    .ok_or(CarrierError::PlanConflict)?,
            ),
            timeout_at: now
                .checked_add(court_warlock::TIMEOUT_US)
                .ok_or(CarrierError::PlanConflict)?,
            raubritters: Vec::new(),
        };
        let relocated = runtime.commit_source_steps_and_project(proofs, project)?;
        *active = Some(phase);
        Ok(relocated)
    }

    pub(super) fn court_warlock_due_us(&self) -> Option<u64> {
        let active = self.court_warlock.lock().ok()?;
        let phase = active.as_ref()?;
        Some(phase.first_wave_at.unwrap_or(phase.timeout_at))
    }

    pub(super) fn tick_court_warlock(&self, runtime: &mut ChannelRuntimeV1, now: u64) {
        let Ok(mut active) = self.court_warlock.lock() else {
            return;
        };
        if let Some(phase) = active.as_mut()
            && (phase.owner != runtime.encounter_owner_scope()
                || phase.advance(runtime, now).is_ok_and(|finished| finished))
        {
            *active = None;
        }
    }
}

fn court_lever_key() -> Option<u64> {
    let pos = court_warlock::LEVER;
    crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: pos.x,
            y: pos.y,
            floor: pos.floor,
        },
        1,
    )
}

impl CourtWarlockPhase {
    fn advance(&mut self, runtime: &mut ChannelRuntimeV1, now: u64) -> Result<bool, CarrierError> {
        let census = runtime.positioned_melee_census()?;
        let occupants: Vec<_> = census
            .into_iter()
            .filter_map(|(actor, at, session)| {
                (court_room_contains(at.position())).then_some((actor, at, session?))
            })
            .collect();
        if occupants.is_empty() || now >= self.timeout_at {
            self.eject(runtime, &occupants)?;
            let live = runtime.current_live_creature_census()?;
            self.raubritters.retain(|actor| {
                live.contains(actor) && runtime.remove_encounter_actor(*actor).is_err()
            });
            return Ok(self.raubritters.is_empty());
        }
        let Some(due) = self.first_wave_at.filter(|due| now >= *due) else {
            return Ok(false);
        };
        // The first occupied lever station is the reference target. Fall back to a
        // current room occupant if that actor has left since admission.
        let (_, origin, _) = self
            .participants
            .iter()
            .find_map(|participant| occupants.iter().find(|(actor, _, _)| actor == participant))
            .unwrap_or(&occupants[0]);
        let origin = origin.position();
        if !placement(&self.cells, runtime, origin) {
            self.first_wave_at = None;
            return Ok(false);
        }
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
            runtime.content_pin().server_artifact_digest(),
        );
        let mut wave = Vec::with_capacity(court_warlock::RAUBRITTERS_PER_WAVE);
        for index in 0..court_warlock::RAUBRITTERS_PER_WAVE {
            let mut occurrence = [0; 16];
            occurrence[..8].copy_from_slice(&due.to_be_bytes());
            occurrence[8..].copy_from_slice(&(index as u64).to_be_bytes());
            let draw = oteryn_simulation_determinism::deterministic_decision_u64(
                &root,
                oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(occurrence),
                "quest.court_warlock.first_wave",
                0,
            )
            .map_err(|_| CarrierError::PlanConflict)?;
            let creature =
                court_warlock::RAUBRITTERS[draw as usize % court_warlock::RAUBRITTERS.len()];
            wave.push((
                creature,
                runtime.companion_policy(creature)?.maximum_health,
                origin,
            ));
        }
        self.raubritters = runtime.commit_quest_creature_spawn(&wave)?;
        self.first_wave_at = None;
        Ok(false)
    }

    fn eject(
        &self,
        runtime: &mut ChannelRuntimeV1,
        occupants: &[(ExactActorRef, MovementPositionSnapshot, GameSessionId)],
    ) -> Result<(), CarrierError> {
        if occupants.is_empty() {
            return Ok(());
        }
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            self.cells.index()
        else {
            return Err(CarrierError::PlanConflict);
        };
        let closed = crate::map::floor::BundleDoors(&index.map().facts);
        let proofs = occupants
            .iter()
            .map(|&(actor, at, session)| {
                crate::map::floor::prepare_quest_encounter_ejection(
                    runtime,
                    &self.cells,
                    actor,
                    session,
                    at,
                    court_local(court_warlock::EXIT),
                    &closed,
                )
                .map_err(|_| CarrierError::PlanConflict)
            })
            .collect::<Result<Vec<_>, _>>()?;
        runtime.commit_quest_relocation(proofs, || true)?;
        Ok(())
    }
}

fn court_room_contains(at: MovementLocalPosition) -> bool {
    let min = court_local(court_warlock::ROOM_MIN);
    let max = court_local(court_warlock::ROOM_MAX);
    at.floor == min.floor && (min.x..=max.x).contains(&at.x) && (min.y..=max.y).contains(&at.y)
}

fn court_local(pos: Position) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(pos.x),
        y: i32::from(pos.y),
        floor: -i16::from(pos.floor),
    }
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
            match phase.fight.victors(runtime) {
                Ok(Some(_)) => {
                    // Credit was captured in the lethal turn. Release the lever
                    // only after exact terminal decorations retire; tick retries failures.
                    let ActivePhase { fight, cells, .. } = phase;
                    fight
                        .advance(runtime, |runtime, at| placement(cells, runtime, at))
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
            if phase.fight.projected_death(runtime, actor).is_ok() {
                let ActivePhase { fight, cells, .. } = phase;
                let _ = fight.advance(runtime, |runtime, at| placement(cells, runtime, at));
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
        fight: CultsEncounter::bind(runtime, players, pillars, stops, centre)?,
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

pub(super) fn tick(runtime: &mut ChannelRuntimeV1, states: &ChannelSpellStates) {
    let Ok(mut active) = states.encounter_phases.active.lock() else {
        return;
    };
    active.retain(|phase| phase.fight.is_current(runtime));
    active.retain_mut(|phase| {
        let ActivePhase {
            key, fight, cells, ..
        } = phase;
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
