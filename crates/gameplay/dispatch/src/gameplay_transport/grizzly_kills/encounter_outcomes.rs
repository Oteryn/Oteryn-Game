//! The latest admitted encounter owns its outcome until the next admission.
mod shields;

use crate::foundation::{
    CarrierError, ChannelRuntimeV1, CreatureDeathOccurrenceKey, ExactActorRef, GameSessionId,
    MovementLocalPosition,
};
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
            })
    }

    pub(in crate::gameplay_transport) fn prepare(
        runtime: &mut ChannelRuntimeV1,
        source: &str,
        outcome: &EncounterOutcome,
        room: &Encounter,
        actors: &[ExactActorRef],
        party: &[(ExactActorRef, GameSessionId)],
        map: &BundleMap,
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
        Ok(Run {
            result: outcome.outcome.clone(),
            target: terminal,
            source: source.to_owned(),
            participants: party.to_vec(),
            death: None,
            room: shield.is_some().then(|| room.clone()),
            shield,
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

    pub(in crate::gameplay_transport) fn tick_shields(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
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
            let Some(shield) = &mut run.shield else {
                continue;
            };
            if let Some(terminal) = shield.advance(runtime, run.target, map, now, |at, runtime| {
                super::super::encounter_phases::placement(cells, runtime, at)
            }) {
                run.target = terminal;
                run.result.mode = EncounterMode::Hard;
                run.shield = None;
            }
        }
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
