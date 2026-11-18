//! Crystal 201189f8, forgotten_knowledge/creaturescripts_thorn_knight_death.lua:12-23
//! and the_first_dragon/creaturescripts_death_dragon_essence.lua:3-12.
use super::{CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_quests::quest::world_callbacks::{DeathPhases, Encounter};

#[derive(Debug)]
struct Member {
    actor: ExactActorRef,
    creature: String,
    death_at: Option<MovementLocalPosition>,
}

#[derive(Debug)]
pub(super) struct Replacements {
    plan: DeathPhases,
    participants: Vec<(ExactActorRef, GameSessionId)>,
    members: Vec<Member>,
    scope: (RuntimeScopeRefV1, ScopeOwnershipGeneration),
    activation: u64,
    finished: bool,
}

impl Replacements {
    pub(super) fn bind(
        runtime: &ChannelRuntimeV1,
        encounter: &Encounter,
        plan: &DeathPhases,
        actors: &[ExactActorRef],
        players: &[(ExactActorRef, GameSessionId)],
    ) -> Result<Self, CarrierError> {
        runtime.owner_fence()?;
        if actors.len() != encounter.monsters.len()
            || actors
                .iter()
                .zip(&encounter.monsters)
                .any(|(actor, monster)| {
                    !runtime.matches_live_creature_identity(*actor, monster.creature.as_bytes())
                })
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        for actor in plan.replacements.iter().flat_map(|rule| &rule.spawn) {
            runtime.companion_policy(&actor.creature)?;
        }
        Ok(Self {
            plan: plan.clone(),
            participants: players.to_vec(),
            members: actors
                .iter()
                .zip(&encounter.monsters)
                .map(|(actor, monster)| Member {
                    actor: *actor,
                    creature: monster.creature.clone(),
                    death_at: None,
                })
                .collect(),
            scope: runtime.encounter_owner_scope(),
            activation: runtime.content_pin().activation_sequence(),
            finished: false,
        })
    }

    pub(super) fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        runtime.owner_fence().is_ok()
            && runtime.encounter_owner_scope() == self.scope
            && runtime.content_pin().activation_sequence() == self.activation
    }

    pub(super) fn contains(&self, actor: ExactActorRef) -> bool {
        self.members.iter().any(|member| member.actor == actor)
    }

    pub(super) fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        if !self.is_current(runtime) || self.finished {
            return Ok(false);
        }
        let member = self
            .members
            .iter_mut()
            .find(|member| member.actor == actor)
            .ok_or(CarrierError::CreatureTargetMismatch)?;
        if member.death_at.is_some() {
            return Ok(false);
        }
        let (_, at) = runtime.borrow_combat_death().projected_death(actor)?;
        member.death_at = Some(at);
        self.finished = member.creature == self.plan.terminal;
        Ok(true)
    }

    pub(super) fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        map_admits: impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<(), CarrierError> {
        if !self.is_current(runtime) {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        while !self.finished {
            let Some((index, at)) = self
                .members
                .iter()
                .enumerate()
                .find_map(|(index, member)| member.death_at.map(|at| (index, at)))
            else {
                break;
            };
            let defeated = &self.members[index].creature;
            let Some(rule) = self
                .plan
                .replacements
                .iter()
                .find(|rule| &rule.defeated == defeated)
            else {
                self.members.swap_remove(index);
                continue;
            };
            let retire: Vec<_> = self
                .members
                .iter()
                .filter(|member| {
                    rule.retire_siblings
                        && &member.creature == defeated
                        && runtime.contains_live_creature(member.actor)
                })
                .map(|member| member.actor)
                .collect();
            let spawn = rule
                .spawn
                .iter()
                .map(|actor| {
                    let at = actor
                        .position
                        .map_or(at, super::super::quest_callbacks::local);
                    let policy = runtime.companion_policy(&actor.creature)?;
                    if !map_admits(runtime, at) {
                        return Err(CarrierError::PlanConflict);
                    }
                    Ok((actor.creature.as_str(), policy.maximum_health, at))
                })
                .collect::<Result<Vec<_>, CarrierError>>()?;
            self.members
                .try_reserve(spawn.len())
                .map_err(|_| CarrierError::AllocationFailed)?;
            let next = runtime.commit_quest_creature_replacement(&retire, &spawn, || true)?;
            self.members.swap_remove(index);
            if rule.retire_siblings {
                self.members
                    .retain(|member| member.creature != rule.defeated);
            }
            self.members.extend(
                next.into_iter()
                    .zip(&rule.spawn)
                    .map(|(actor, spawn)| Member {
                        actor,
                        creature: spawn.creature.clone(),
                        death_at: None,
                    }),
            );
        }
        Ok(())
    }

    pub(super) fn victors(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&[(ExactActorRef, GameSessionId)]>, CarrierError> {
        if !self.is_current(runtime) {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        Ok(self.finished.then_some(self.participants.as_slice()))
    }
}
