//! The latest admitted encounter owns its outcome until the next admission.
use crate::foundation::{
    ChannelRuntimeV1, CreatureDeathOccurrenceKey, ExactActorRef, GameSessionId,
};
use oteryn_gameplay_quests::quest::world_callbacks::{EncounterOutcome, OutcomeRequirement};
use std::collections::BTreeMap;

const MAX_GROUPS: usize = 64;

#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct Outcomes(BTreeMap<String, Run>);

#[derive(Clone, Debug)]
pub(super) struct Run {
    result: OutcomeRequirement,
    target: ExactActorRef,
    source: String,
    participants: Vec<(ExactActorRef, GameSessionId)>,
    death: Option<CreatureDeathOccurrenceKey>,
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

    pub(in crate::gameplay_transport) fn start(
        &mut self,
        source: &str,
        outcome: &EncounterOutcome,
        terminal: ExactActorRef,
        party: &[(ExactActorRef, GameSessionId)],
    ) {
        self.0.insert(
            outcome.outcome.group.clone(),
            Run {
                result: outcome.outcome.clone(),
                target: terminal,
                source: source.to_owned(),
                participants: party.to_vec(),
                death: None,
            },
        );
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
