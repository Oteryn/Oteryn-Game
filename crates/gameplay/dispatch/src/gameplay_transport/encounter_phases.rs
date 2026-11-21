//! One phase registry on the existing encounter, native death queue and quest writer.
use super::{actor_spell::ChannelSpellStates, grizzly_kills, kill_reward::PendingKillSettlement};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use oteryn_gameplay_encounters::bosses::phase_controller::{
    arbaziloth::{self, ArbazilothEncounter},
    court_warlock::CourtWarlock,
    cults::{self, CultsEncounter},
};
use oteryn_gameplay_quests::quest::world_callbacks::{Encounter, EncounterPhase};
use oteryn_gameplay_world::map::boot::BundleCollisionIndexExt;
use std::sync::Mutex;

const MAX_ENCOUNTERS: usize = 64;
// Crystal 201189f8 boss_lever.lua uses the configured fallback for Arbaziloth.
// config.lua.dist sets bossDefaultTimeToDefeat to 20 minutes.
const ARBAZILOTH_ATTEMPT_US: u64 = 20 * 60 * 1_000_000;

#[derive(Debug)]
pub(super) struct ActivePhase {
    key: u64,
    fight: Fight,
    room: Encounter,
    participants: Vec<(ExactActorRef, GameSessionId)>,
    credits: Vec<grizzly_kills::EncounterCredit>,
    frozen: Option<(ExactActorRef, Vec<super::quest_kills::Recipient>)>,
    cells: crate::content::NativeEntryMovementCells,
    fire: Option<crate::map::overlay::TilePos>,
    arbaziloth_deadline: Option<u64>,
}

#[derive(Debug)]
enum Fight {
    Arbaziloth(Box<ArbazilothEncounter>),
    CourtWarlock(Box<CourtWarlock>),
    Cults(Box<CultsEncounter>),
}

impl Fight {
    fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        match self {
            Self::Arbaziloth(fight) => fight.is_current(runtime),
            Self::CourtWarlock(fight) => fight.is_current(runtime),
            Self::Cults(fight) => fight.is_current(runtime),
        }
    }

    fn contains(&self, actor: ExactActorRef) -> bool {
        match self {
            Self::Arbaziloth(fight) => fight.contains_terminal(actor),
            Self::CourtWarlock(fight) => fight.contains(actor),
            Self::Cults(fight) => fight.contains(actor),
        }
    }

    fn contains_terminal(&self, actor: ExactActorRef) -> bool {
        match self {
            Self::Arbaziloth(fight) => fight.contains_terminal(actor),
            Self::CourtWarlock(fight) => fight.contains_terminal(actor),
            Self::Cults(fight) => fight.contains_terminal(actor),
        }
    }

    fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        match self {
            Self::Arbaziloth(fight) => fight.projected_death(runtime, actor),
            Self::CourtWarlock(fight) => fight.projected_death(runtime, actor),
            Self::Cults(fight) => fight.projected_death(runtime, actor),
        }
    }

    fn victors<'a>(
        &'a self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&'a [(ExactActorRef, GameSessionId)]>, CarrierError> {
        match self {
            Self::Arbaziloth(fight) => fight.victors(runtime),
            Self::CourtWarlock(fight) => fight.victors(runtime),
            Self::Cults(fight) => fight.victors(runtime),
        }
    }
}

#[derive(Default, Debug)]
pub(super) struct Registry {
    pub(super) active: Mutex<Vec<ActivePhase>>,
}

impl Registry {
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
        states: &ChannelSpellStates,
        key: u64,
        now_us: u64,
    ) -> Result<std::sync::MutexGuard<'a, Vec<ActivePhase>>, CarrierError> {
        let mut active = self.active.lock().map_err(|_| CarrierError::PlanConflict)?;
        active.retain_mut(|phase| {
            match phase.retire_abandoned_arbaziloth(runtime, states, now_us) {
                Ok(true) => return false,
                Err(_) => return true,
                Ok(false) => {}
            }
            if !phase.fight.is_current(runtime) {
                return !phase.clear_fire();
            }
            if matches!(phase.fight, Fight::CourtWarlock(_)) {
                return true;
            }
            match phase.fight.victors(runtime) {
                Ok(Some(_)) => {
                    // Credit was captured in the lethal turn. Release the lever
                    // only after exact terminal decorations retire; tick retries failures.
                    let Fight::Cults(fight) = &mut phase.fight else {
                        return !phase.clear_fire();
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
                        matches!(phase.fight, Fight::Arbaziloth(_)).then_some(entry.facts.death),
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
    encounter: &Encounter,
    key: u64,
    actors: &[ExactActorRef],
    players: &[(ExactActorRef, GameSessionId)],
    credits: Vec<grizzly_kills::EncounterCredit>,
    cells: crate::content::NativeEntryMovementCells,
    now_us: u64,
) -> Result<ActivePhase, CarrierError> {
    if encounter.phase == Some(EncounterPhase::Arbaziloth) {
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            cells.index()
        else {
            return Err(CarrierError::PlanConflict);
        };
        if index
            .map()
            .facts
            .movement_item_facts(49362)
            .is_none_or(|fire| fire.pickupable)
        {
            return Err(CarrierError::PlanConflict);
        }

        let find_actor = |key: &str| {
            actors
                .iter()
                .copied()
                .find(|actor| runtime.matches_live_creature_identity(*actor, key.as_bytes()))
                .ok_or(CarrierError::CreatureTargetMismatch)
        };
        let boss = find_actor(arbaziloth::BOSS)?;
        let forgemaster = find_actor(arbaziloth::FORGEMASTER)?;
        let fight = ArbazilothEncounter::bind(runtime, players, boss, forgemaster)?;
        return Ok(ActivePhase {
            key,
            fight: Fight::Arbaziloth(Box::new(fight)),
            room: encounter.clone(),
            participants: players.to_vec(),
            credits,
            frozen: None,
            cells,
            fire: None,
            arbaziloth_deadline: Some(
                now_us
                    .checked_add(ARBAZILOTH_ATTEMPT_US)
                    .ok_or(CarrierError::CapacityArithmeticOverflow)?,
            ),
        });
    }
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
                now_us,
                local(encounter.room_min),
                local(encounter.room_max),
                local(center),
            )?)),
            room: encounter.clone(),
            participants: players.to_vec(),
            credits,
            frozen: None,
            cells,
            fire: None,
            arbaziloth_deadline: None,
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
        fire: None,
        arbaziloth_deadline: None,
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

impl ActivePhase {
    fn retire_abandoned_arbaziloth(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &ChannelSpellStates,
        now_us: u64,
    ) -> Result<bool, CarrierError> {
        let (Fight::Arbaziloth(fight), Some(deadline)) = (&self.fight, self.arbaziloth_deadline)
        else {
            return Ok(false);
        };
        match fight.victors(runtime) {
            Ok(None) => {}
            // Let the normal registry cleanup release a won or stale phase.
            Ok(Some(_)) | Err(CarrierError::ContentPinWorldMismatch) => return Ok(false),
            Err(error) => return Err(error),
        }
        if !fight.is_current(runtime) {
            fight.retire(runtime)?;
            return self
                .clear_fire()
                .then_some(true)
                .ok_or(CarrierError::PlanConflict);
        }
        if now_us < deadline
            && self.participants.iter().any(|&(actor, session)| {
                !states.is_dead(actor)
                    && runtime
                        .player_control_facts(actor, session)
                        .is_ok_and(|facts| facts.control_loss.is_none())
                    && runtime.read_actor_position(actor).is_ok_and(|at| {
                        super::quest_callbacks::room_contains(&self.room, at.position())
                    })
            })
        {
            return Ok(false);
        }

        let players: Vec<_> = runtime
            .positioned_melee_census()?
            .into_iter()
            .filter(|(_, at, session)| {
                session.is_some()
                    && super::quest_callbacks::room_contains(&self.room, at.position())
            })
            .collect();
        if !players.is_empty() {
            let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
                self.cells.index()
            else {
                return Err(CarrierError::PlanConflict);
            };
            let closed = crate::map::floor::BundleDoors(&index.map().facts);
            // Crystal 201189f8 arbaziloth_lever.lua, exit.
            let exit = MovementLocalPosition {
                x: 34058,
                y: 32403,
                floor: 14,
            };
            let proofs = players
                .into_iter()
                .map(|(actor, expected, session)| {
                    crate::map::floor::prepare_quest_encounter_ejection(
                        runtime,
                        &self.cells,
                        actor,
                        session.ok_or(CarrierError::PlanConflict)?,
                        expected,
                        exit,
                        &closed,
                    )
                    .map_err(|_| CarrierError::PlanConflict)
                })
                .collect::<Result<Vec<_>, _>>()?;
            runtime.commit_quest_relocation(proofs, || true)?;
        }
        fight.retire(runtime)?;
        if !self.clear_fire() {
            return Err(CarrierError::PlanConflict);
        }
        Ok(true)
    }

    fn clear_fire(&mut self) -> bool {
        let Some(fire) = self.fire else { return true };
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            self.cells.index()
        else {
            return false;
        };
        if !index
            .map()
            .facts
            .project_arbaziloth_fire(self.key, fire, true)
        {
            return false;
        }
        self.fire = None;
        true
    }

    fn tick_fire(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now_ms: u64,
    ) -> Result<(), CarrierError> {
        use crate::map::overlay::TilePos;
        let Fight::Arbaziloth(fight) = &self.fight else {
            return Ok(());
        };
        let Some(boss) = fight.normal_boss() else {
            return Ok(());
        };
        if !runtime
            .arbaziloth_shields(boss)
            .is_some_and(|shields| shields.needs_fire())
        {
            return Ok(());
        }
        let position = runtime.read_actor_position(boss)?.position();
        if let Some(fire) = self.fire {
            let on_fire = position.x == i32::from(fire.x)
                && position.y == i32::from(fire.y)
                && position.floor == -i16::from(fire.floor);
            if on_fire
                && let Some(shields) = runtime.arbaziloth_shields_mut(boss)
                && self.clear_fire()
            {
                shields.step_on_fire();
            }
            return Ok(());
        }

        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            self.cells.index()
        else {
            return Err(CarrierError::PlanConflict);
        };
        let map = index.map();
        // Crystal 201189f8 arbaziloth_fight.lua: bounded random search of walkable arena tiles.
        let candidates: Vec<_> = (34026..=34040)
            .flat_map(|x| (32324..=32340).map(move |y| TilePos { x, y, floor: -14 }))
            .filter(|at| {
                let candidate = MovementLocalPosition {
                    x: i32::from(at.x),
                    y: i32::from(at.y),
                    floor: 14,
                };
                candidate != position
                    && placement(&self.cells, runtime, candidate)
                    && map.floors.teleport(*at).is_none()
            })
            .collect();
        let count = i64::try_from(candidates.len()).map_err(|_| CarrierError::PlanConflict)?;
        if count == 0 {
            return Err(CarrierError::PlanConflict);
        }
        use oteryn_simulation_determinism::{
            DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
        };
        let mut occurrence = [0; 16];
        occurrence[..8].copy_from_slice(&self.key.to_be_bytes());
        occurrence[8..].copy_from_slice(&now_ms.to_be_bytes());
        let draw = deterministic_decision_u64(
            &GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest()),
            DecisionOccurrenceId::from_bytes(occurrence),
            "arbaziloth.arena_fire",
            0,
        )
        .map_err(|_| CarrierError::PlanConflict)?;
        let selected = crate::spell::uniform_draw(draw, 0, count - 1);
        let fire = candidates[usize::try_from(selected).map_err(|_| CarrierError::PlanConflict)?];
        if !map.facts.project_arbaziloth_fire(self.key, fire, false) {
            return Err(CarrierError::PlanConflict);
        }
        self.fire = Some(fire);
        Ok(())
    }
}

pub(super) fn tick(runtime: &mut ChannelRuntimeV1, states: &ChannelSpellStates, now_us: u64) {
    let Ok(mut active) = states.encounter_phases.active.lock() else {
        return;
    };
    active.retain_mut(|phase| {
        match phase.retire_abandoned_arbaziloth(runtime, states, now_us) {
            Ok(true) => return false,
            Err(error) => {
                super::operator_event(&format!(
                    "arbaziloth_retirement_deferred key={} reason={error:?}",
                    phase.key
                ));
                return true;
            }
            Ok(false) => {}
        }
        if !phase.fight.is_current(runtime) {
            return !phase.clear_fire();
        }
        let now_ms = now_us / 1_000;
        if let Err(error) = phase.tick_fire(runtime, now_ms) {
            super::operator_event(&format!(
                "encounter_phase_fire_deferred key={} reason={error:?}",
                phase.key
            ));
        }
        let ActivePhase {
            key, fight, cells, ..
        } = phase;
        let advance = match fight {
            Fight::Arbaziloth(fight) => fight
                .advance(runtime, now_ms, |runtime, at| placement(cells, runtime, at))
                .map(|_| ()),
            Fight::CourtWarlock(fight) => match tick_court_warlock(runtime, fight, cells, now_us) {
                Ok(keep) => return keep,
                Err(error) => Err(error),
            },
            Fight::Cults(fight) => {
                fight.advance(runtime, |runtime, at| placement(cells, runtime, at))
            }
        };
        if let Err(error) = advance {
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

fn tick_court_warlock(
    runtime: &mut ChannelRuntimeV1,
    fight: &mut CourtWarlock,
    cells: &crate::content::NativeEntryMovementCells,
    now_us: u64,
) -> Result<bool, CarrierError> {
    let players: Vec<_> = runtime
        .positioned_melee_census()?
        .into_iter()
        .filter(|(_, at, session)| session.is_some() && fight.in_room(at.position()))
        .collect();
    if !players.is_empty() && !fight.exit_due(now_us) {
        fight.advance(runtime, now_us, |runtime, at| placement(cells, runtime, at))?;
        return Ok(true);
    }

    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        cells.index()
    else {
        return Err(CarrierError::PlanConflict);
    };
    let closed = crate::map::floor::BundleDoors(&index.map().facts);
    let exit = oteryn_gameplay_quests::quest::world_callbacks::court_warlock::EXIT;
    let destination = MovementLocalPosition {
        x: i32::from(exit.x),
        y: i32::from(exit.y),
        floor: -i16::from(exit.floor),
    };
    let mut proofs = Vec::new();
    for (actor, expected, session) in players {
        let session = session.ok_or(CarrierError::PlanConflict)?;
        proofs.push(
            crate::map::floor::prepare_quest_encounter_ejection(
                runtime,
                cells,
                actor,
                session,
                expected,
                destination,
                &closed,
            )
            .map_err(|_| CarrierError::PlanConflict)?,
        );
    }
    runtime.commit_quest_relocation(proofs, || true)?;
    fight.retire(runtime)?;
    Ok(false)
}
