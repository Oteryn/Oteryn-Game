//! One phase registry on the existing encounter, native death queue and quest writer.
use super::{actor_spell::ChannelSpellStates, grizzly_kills, kill_reward::PendingKillSettlement};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use oteryn_gameplay_encounters::bosses::phase_controller::arbaziloth::{self, ArbazilothEncounter};
use oteryn_gameplay_encounters::bosses::phase_controller::cults::{self, CultsEncounter};
use oteryn_gameplay_quests::quest::world_callbacks::{Encounter, EncounterPhase};
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
    fire: Option<crate::map::overlay::TilePos>,
}

#[derive(Debug)]
enum Fight {
    Arbaziloth(Box<ArbazilothEncounter>),
    Cults(Box<CultsEncounter>),
}

impl Fight {
    fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        match self {
            Self::Arbaziloth(fight) => fight.is_current(runtime),
            Self::Cults(fight) => fight.is_current(runtime),
        }
    }

    fn contains(&self, actor: ExactActorRef) -> bool {
        match self {
            Self::Arbaziloth(fight) => fight.contains_terminal(actor),
            Self::Cults(fight) => fight.contains(actor),
        }
    }

    fn contains_terminal(&self, actor: ExactActorRef) -> bool {
        match self {
            Self::Arbaziloth(fight) => fight.contains_terminal(actor),
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
            Self::Cults(fight) => fight.projected_death(runtime, actor),
        }
    }

    fn victors<'a>(
        &'a self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&'a [(ExactActorRef, GameSessionId)]>, CarrierError> {
        match self {
            Self::Arbaziloth(fight) => fight.victors(runtime),
            Self::Cults(fight) => fight.victors(runtime),
        }
    }

    fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now_ms: Option<u64>,
        map_admits: impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<(), CarrierError> {
        match self {
            Self::Arbaziloth(fight) => {
                if let Some(now_ms) = now_ms {
                    fight.advance(runtime, now_ms, map_admits)?;
                }
                Ok(())
            }
            Self::Cults(fight) => fight.advance(runtime, map_admits),
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
                return !phase.clear_fire();
            }
            match phase.fight.victors(runtime) {
                Ok(Some(_)) => {
                    // Credit was captured in the lethal turn. Release the lever
                    // only after exact terminal decorations retire; tick retries failures.
                    let ActivePhase { fight, cells, .. } = phase;
                    fight
                        .advance(runtime, None, |runtime, at| placement(cells, runtime, at))
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
                let _ = fight.advance(runtime, None, |runtime, at| placement(cells, runtime, at));
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
                        matches!(phase.fight, Fight::Arbaziloth(_)).then_some(entry.facts.death),
                    ),
                )
            });
            grizzly_kills::replace_encounter_native_credit(entry, recipients.clone());
        }
    }
}

pub(super) fn bind(
    runtime: &mut ChannelRuntimeV1,
    encounter: &Encounter,
    key: u64,
    actors: &[ExactActorRef],
    players: &[(ExactActorRef, GameSessionId)],
    credits: Vec<grizzly_kills::EncounterCredit>,
    cells: crate::content::NativeEntryMovementCells,
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
            credits,
            frozen: None,
            cells,
            fire: None,
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
        fire: None,
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

pub(super) fn tick(runtime: &mut ChannelRuntimeV1, states: &ChannelSpellStates, now_ms: u64) {
    let Ok(mut active) = states.encounter_phases.active.lock() else {
        return;
    };
    active.retain_mut(|phase| {
        if !phase.fight.is_current(runtime) {
            return !phase.clear_fire();
        }
        if let Err(error) = phase.tick_fire(runtime, now_ms) {
            super::operator_event(&format!(
                "encounter_phase_fire_deferred key={} reason={error:?}",
                phase.key
            ));
        }
        let ActivePhase {
            key, fight, cells, ..
        } = phase;
        if let Err(error) = fight.advance(runtime, Some(now_ms), |runtime, at| {
            placement(cells, runtime, at)
        }) {
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
