//! Crystal 201189f8, forgotten_knowledge/creaturescripts_thorn_knight_death.lua:12-23
//! and the_first_dragon/creaturescripts_death_dragon_essence.lua:3-12.
use super::{CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_entities::carrier::QuestCorpseStep;
use oteryn_gameplay_quests::quest::world_callbacks::{DeathPhases, Encounter};
use oteryn_gameplay_world::map::boot::BundleCollisionIndexExt;

use super::terminal_portal::Portal;
use super::terminal_transfer::{Movement, Transfer};

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
    transfer: Option<Transfer>,
    portal: Option<Portal>,
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
            transfer: None,
            portal: None,
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
        now_us: u64,
        cells: &crate::content::NativeEntryMovementCells,
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
        if let Some(portal) = &self.plan.corpse_portal
            && member.creature == portal.defeated
        {
            // Crystal 201189f8, creaturescripts_death_fallen.lua:10-14.
            runtime.bind_quest_corpse_step(
                actor,
                &portal.item,
                QuestCorpseStep::Portal(super::super::quest_callbacks::local(portal.destination)),
            )?;
            if let Some(cleanup) = portal.cleanup {
                super::terminal_portal::schedule_cleanup(cells, at, cleanup, now_us);
            }
        }
        if let Some(step) = &self.plan.corpse_monster_step
            && member.creature == step.defeated
        {
            // Crystal 201189f8, creaturescripts_death_angry_plant.lua:3-7.
            runtime.bind_quest_corpse_step(
                actor,
                &step.item,
                QuestCorpseStep::ReplaceMonster {
                    creature: step.creature.clone(),
                    replacement: step.replace_with.clone(),
                },
            )?;
        }
        let finished = member.creature == self.plan.terminal;
        if finished && let Some(plan) = self.plan.terminal_transfer {
            self.transfer = Some(Transfer::capture(runtime, plan)?);
        }
        if finished {
            self.portal = self
                .plan
                .terminal_portal
                .and_then(|plan| Portal::open(plan, now_us));
        }
        member.death_at = Some(at);
        self.finished = finished;
        Ok(true)
    }

    pub(super) fn transfer_pending(&self) -> bool {
        self.transfer.as_ref().is_some_and(Transfer::is_pending)
    }

    pub(super) fn replace_member(
        &mut self,
        runtime: &ChannelRuntimeV1,
        from: ExactActorRef,
        to: ExactActorRef,
        creature: &str,
    ) {
        if self.is_current(runtime)
            && !self.finished
            && let Some(member) = self.members.iter_mut().find(|member| member.actor == from)
            && member.death_at.is_none()
        {
            member.actor = to;
            member.creature = creature.to_owned();
        }
    }

    pub(super) fn completion_pending(&self) -> bool {
        self.portal.is_some() || self.transfer_pending()
    }

    pub(super) fn project_portal(
        &mut self,
        cells: &crate::content::NativeEntryMovementCells,
        now_us: u64,
    ) {
        if self
            .portal
            .as_ref()
            .is_some_and(|portal| portal.project(cells, now_us))
        {
            self.portal = None;
        }
    }

    pub(super) fn project_transfer(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &super::ChannelSpellStates,
        cells: &crate::content::NativeEntryMovementCells,
        map: &crate::map::boot::BundleMap,
        published: &mut Vec<Movement>,
    ) {
        if let Some(transfer) = &mut self.transfer {
            transfer.project(runtime, states, cells, map, published);
        }
    }

    pub(super) fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &crate::content::NativeEntryMovementCells,
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
            if let Some(relocate) = &rule.relocate {
                // Crystal 201189f8, forgotten_knowledge/creaturescripts_energy_prism_death.lua:4-11.
                let from = super::super::quest_callbacks::local(relocate.from);
                let to = super::super::quest_callbacks::local(relocate.to);
                let guarded_actor = self
                    .members
                    .iter()
                    .filter(|member| {
                        member.creature == relocate.creature && member.death_at.is_none()
                    })
                    .find_map(|member| {
                        let expected = runtime.read_actor_position(member.actor).ok()?;
                        (expected.position() == from
                            && runtime.contains_live_creature(member.actor))
                        .then_some((member.actor, expected))
                    });
                if let Some((actor, expected)) = guarded_actor {
                    if !super::placement(cells, runtime, to) {
                        return Err(CarrierError::PlanConflict);
                    }
                    runtime
                        .borrow_movement_position()
                        .commit_creature_relocation_facing(
                            actor,
                            expected,
                            to,
                            crate::movement::monster_map::source_teleport_facing(from, to),
                        )?;
                    if let Some(effect) = relocate.arrival_effect
                        && let Some(at) = super::super::quest_callbacks::native(to)
                        && let NativeMovementCollisionIndex::Bundle(map) = cells.index()
                    {
                        map.map().facts.emit_movement_effect(at, effect);
                    }
                }
                self.members.swap_remove(index);
                continue;
            }
            // Crystal 201189f8, the_first_dragon/creaturescripts_death_dragon_essence.lua:3-11.
            // Its sweep includes matching creatures that arrived after the lever was used.
            let retire: Vec<_> = if let Some(area) = rule.retire_siblings_in {
                runtime
                    .current_live_creature_census()?
                    .into_iter()
                    .filter(|actor| {
                        runtime.matches_live_creature_identity(*actor, defeated.as_bytes())
                            && runtime
                                .read_actor_position(*actor)
                                .ok()
                                .is_some_and(|position| {
                                    super::super::quest_callbacks::native(position.position())
                                        .is_some_and(|at| {
                                            area.contains(
                                        oteryn_gameplay_quests::quest::world_callbacks::Position {
                                            x: at.x, y: at.y, floor: at.floor,
                                        }
                                    )
                                        })
                                })
                    })
                    .collect()
            } else {
                self.members
                    .iter()
                    .filter(|member| {
                        rule.retire_siblings
                            && &member.creature == defeated
                            && runtime.contains_live_creature(member.actor)
                    })
                    .map(|member| member.actor)
                    .collect()
            };
            let spawn = rule
                .spawn
                .iter()
                .map(|actor| {
                    let at = actor
                        .position
                        .map_or(at, super::super::quest_callbacks::local);
                    let policy = runtime.companion_policy(&actor.creature)?;
                    if !super::placement(cells, runtime, at) {
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
