//! The latest admitted encounter owns its outcome until the next admission.
mod reinforcements;
mod shields;
mod tool_use;

use crate::foundation::{
    CarrierError, ChannelRuntimeV1, CreatureDeathOccurrenceKey, ExactActorRef, GameSessionId,
    MovementLocalPosition,
};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::map::boot::BundleMap;
use oteryn_gameplay_quests::quest::world_callbacks::{
    Encounter, EncounterMode, EncounterOutcome, OutcomeRequirement,
};
use std::collections::BTreeMap;

const MAX_GROUPS: usize = 64;

#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct Outcomes(BTreeMap<String, Run>);

#[derive(Clone, Debug)]
pub(in crate::gameplay_transport) struct Run {
    result: OutcomeRequirement,
    target: ExactActorRef,
    source: String,
    participants: Vec<(ExactActorRef, GameSessionId)>,
    death: Option<CreatureDeathOccurrenceKey>,
    room: Option<Encounter>,
    shield: Option<shields::Fight>,
    reinforcements: Option<reinforcements::Roster>,
}

impl Run {
    fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.target.runtime_scope() == runtime.binding().runtime_scope()
            && self.target.scope_generation() == runtime.binding().scope_generation()
    }

    pub(super) fn credit_source(
        &self,
        required: &OutcomeRequirement,
        player: (ExactActorRef, GameSessionId),
    ) -> Option<&str> {
        (&self.result == required && self.participants.contains(&player))
            .then_some(self.source.as_str())
    }
}

impl Outcomes {
    pub(in crate::gameplay_transport) fn can_start(
        &mut self,
        runtime: &ChannelRuntimeV1,
        outcome: &EncounterOutcome,
    ) -> bool {
        self.0.retain(|_, run| run.current(runtime));
        self.0
            .get(&outcome.outcome.group)
            .map_or(self.0.len() < MAX_GROUPS, |run| {
                !runtime.contains_live_creature(run.target)
                    && run
                        .reinforcements
                        .as_ref()
                        .is_none_or(|roster| !roster.pulse_pending())
            })
    }

    /// Crystal 201189f8 arbaziloth_fight.lua:348-373 retires the living
    /// remnants at the next lever. Corpse and loot ownership stays separate.
    pub(in crate::gameplay_transport) fn retiring_reinforcements(
        &self,
        runtime: &ChannelRuntimeV1,
        outcome: &EncounterOutcome,
    ) -> Vec<ExactActorRef> {
        self.0
            .get(&outcome.outcome.group)
            .filter(|run| run.current(runtime) && !runtime.contains_live_creature(run.target))
            .and_then(|run| run.reinforcements.as_ref())
            .map_or_else(Vec::new, |roster| roster.living(runtime))
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the roster is bound to the same exact admission"
    )]
    pub(in crate::gameplay_transport) fn prepare(
        runtime: &mut ChannelRuntimeV1,
        source: &str,
        outcome: &EncounterOutcome,
        room: &Encounter,
        actors: &[ExactActorRef],
        party: &[(ExactActorRef, GameSessionId)],
        map: &BundleMap,
        now_ms: u64,
    ) -> Result<Run, CarrierError> {
        let terminal = actors
            .iter()
            .copied()
            .find(|actor| {
                runtime.matches_live_creature_identity(*actor, outcome.terminal.as_bytes())
            })
            .ok_or(CarrierError::CreatureTargetMismatch)?;
        let shield = if let Some(rule) = &outcome.shield_handoff {
            let maximum_health = runtime.companion_policy(&outcome.terminal)?.maximum_health;
            Some(shields::Fight::bind(
                runtime,
                terminal,
                actors,
                rule,
                maximum_health,
                map,
            )?)
        } else {
            None
        };
        let reinforcements = outcome
            .shield_handoff
            .as_ref()
            .and_then(|shield| shield.reinforcements.as_ref())
            .map(|rule| reinforcements::Roster::bind(runtime, room, actors, rule, now_ms))
            .transpose()?;
        Ok(Run {
            result: outcome.outcome.clone(),
            target: terminal,
            source: source.to_owned(),
            participants: party.to_vec(),
            death: None,
            room: shield.is_some().then(|| room.clone()),
            shield,
            reinforcements,
        })
    }

    pub(in crate::gameplay_transport) fn start(&mut self, run: Run) {
        self.0.insert(run.result.group.clone(), run);
    }

    pub(in crate::gameplay_transport) fn blocks_channel_switch(
        &self,
        runtime: &ChannelRuntimeV1,
        player: (ExactActorRef, GameSessionId),
        position: MovementLocalPosition,
    ) -> bool {
        self.0.values().any(|run| {
            run.current(runtime)
                && run.participants.contains(&player)
                && runtime.contains_live_creature(run.target)
                && run.room.as_ref().is_some_and(|room| {
                    super::super::quest_callbacks::room_contains(room, position)
                })
        })
    }

    pub(in crate::gameplay_transport) fn tick(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        now: u64,
        cells: &crate::content::NativeEntryMovementCells,
    ) {
        if runtime.owner_fence().is_err() {
            return;
        }
        for run in self.0.values_mut() {
            if !run.current(runtime) {
                continue;
            }
            let placeable = |at, runtime: &ChannelRuntimeV1| {
                super::super::encounter_phases::placement(cells, runtime, at)
            };
            if let Some(shield) = &mut run.shield
                && let Some(terminal) = shield.advance(runtime, run.target, map, now, placeable)
            {
                run.target = terminal;
                run.result.mode = EncounterMode::Hard;
                run.shield = None;
            }
            if let Some(roster) = &mut run.reinforcements {
                roster.tick_pulse(runtime, states, map, run.target, now);
                roster.advance(runtime, run.target, now, placeable);
            }
        }
    }

    fn for_tool(&mut self, runtime: &ChannelRuntimeV1, target: ExactActorRef) -> Option<&mut Run> {
        self.0.values_mut().find(|run| {
            run.target == target
                && run.current(runtime)
                && run.shield.is_some()
                && run.death.is_none()
        })
    }

    pub(in crate::gameplay_transport) fn observe(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
    ) {
        if !self
            .0
            .values()
            .any(|run| run.target == target && run.current(runtime))
        {
            return;
        }
        let Ok((death, _)) = runtime.borrow_combat_death().projected_death(target) else {
            return;
        };
        for run in self.0.values_mut().filter(|run| run.target == target) {
            run.death.get_or_insert(death);
        }
    }

    pub(in crate::gameplay_transport) fn permits_exit(
        &self,
        runtime: &ChannelRuntimeV1,
        required: &OutcomeRequirement,
    ) -> bool {
        runtime.owner_fence().is_ok()
            && self.0.get(&required.group).is_some_and(|run| {
                run.current(runtime) && &run.result == required && run.death.is_some()
            })
    }

    pub(super) fn for_death(&self, death: CreatureDeathOccurrenceKey) -> Option<Run> {
        self.0
            .values()
            .find(|run| run.death == Some(death))
            .cloned()
    }
}

impl super::super::ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) fn tick_encounter_outcomes(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        now_ms: u64,
    ) {
        if let Some(map) = self.bundle_map()
            && let Ok(attack) = self.states.attack.try_lock()
            && let Ok(mut capture) = attack.kills().1.lock()
        {
            capture
                .outcomes
                .tick(runtime, states, map, now_ms, self.movement_cells());
        }
    }
}
