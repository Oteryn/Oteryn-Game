//! Crystal 201189f8, forgotten_knowledge/creaturescripts_thorn_knight_death.lua:12-23
//! and the_first_dragon/creaturescripts_death_dragon_essence.lua:3-12.
use super::{CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_entities::carrier::QuestCorpseStep;
use oteryn_gameplay_quests::quest::world_callbacks::{DeathArea, DeathPhases, Encounter, Position};
use oteryn_gameplay_world::map::boot::BundleCollisionIndexExt;
use oteryn_gameplay_world::map::view::MapFacts;
use std::collections::BTreeSet;

use super::terminal_portal::Portal;
use super::terminal_transfer::{Movement, Transfer};

#[derive(Debug)]
struct Member {
    actor: ExactActorRef,
    creature: String,
    death_at: Option<Death>,
}

#[derive(Debug, Clone, Copy)]
struct Death {
    at: MovementLocalPosition,
    now_us: u64,
    contributor_at: Option<MovementLocalPosition>,
}

#[derive(Debug)]
pub(super) struct Replacements {
    plan: DeathPhases,
    participants: Vec<(ExactActorRef, GameSessionId)>,
    members: Vec<Member>,
    completed_sweeps: BTreeSet<String>,
    crystal_charges: super::crystal_circuit::Charges,
    room: DeathArea,
    vortex_consumed: u16,
    scope: (RuntimeScopeRefV1, ScopeOwnershipGeneration),
    activation: u64,
    finished: bool,
    transfer: Option<Transfer>,
    portal: Option<Portal>,
}

impl Replacements {
    pub(super) fn bind(
        runtime: &mut ChannelRuntimeV1,
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
        if let Some(recovery) = &plan.heal_before_damage {
            let vacant_cell = super::super::quest_callbacks::local(recovery.vacant_cell);
            for (&actor, monster) in actors.iter().zip(&encounter.monsters) {
                if recovery.creatures.contains(&monster.creature) {
                    runtime.bind_quest_damage_recovery(actor, vacant_cell, recovery.amount)?;
                }
            }
        }
        if let Some(circuit) = &plan.crystal_circuit {
            runtime.companion_policy(&circuit.defeated)?;
            runtime.companion_policy(&circuit.spawn)?;
            if let Some(vortex) = &circuit.vortex {
                runtime.companion_policy(&vortex.consume_creature)?;
                runtime.companion_policy(&vortex.spawn)?;
            }
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
            completed_sweeps: BTreeSet::new(),
            crystal_charges: super::crystal_circuit::Charges::default(),
            room: DeathArea {
                minimum: encounter.room_min,
                maximum: encounter.room_max,
            },
            vortex_consumed: 0,
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

    pub(super) fn accepts_regional_death(
        &self,
        runtime: &ChannelRuntimeV1,
        creature: &str,
        at: MovementLocalPosition,
    ) -> bool {
        !self.finished
            && self.is_current(runtime)
            && !self.completed_sweeps.contains(creature)
            && (self.plan.crystal_circuit.as_ref().is_some_and(|plan| {
                self.crystal_charges.crystal(plan, creature, at).is_some()
                    || (plan.defeated == creature
                        && plan.vortex.is_some()
                        && in_death_area(self.room, at))
            }) || self.plan.replacements.iter().any(|rule| {
                rule.defeated == creature
                    && rule
                        .retire_siblings_in
                        .or(rule.observe_deaths_in)
                        .is_some_and(|area| in_death_area(area, at))
            }))
    }

    pub(super) fn record_regional_member(
        &mut self,
        actor: ExactActorRef,
        creature: &str,
    ) -> Result<(), CarrierError> {
        self.members
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        self.members.push(Member {
            actor,
            creature: creature.to_owned(),
            death_at: None,
        });
        Ok(())
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
        let contributor_at = if self
            .plan
            .contributor_route
            .as_ref()
            .is_some_and(|route| route.creature == member.creature)
        {
            runtime
                .borrow_combat_death()
                .top_damage_contributor(actor)?
                .present()
                .and_then(|winner| runtime.read_actor_position(winner.actor).ok())
                .map(|position| position.position())
        } else {
            None
        };
        member.death_at = Some(Death {
            at,
            now_us,
            contributor_at,
        });
        self.finished = finished;
        Ok(true)
    }

    pub(super) fn transfer_pending(&self) -> bool {
        self.transfer.as_ref().is_some_and(Transfer::is_pending)
    }

    pub(super) fn terminal_spectators(
        &self,
        actor: ExactActorRef,
    ) -> Option<&[(ExactActorRef, GameSessionId)]> {
        if !self.members.iter().any(|member| {
            member.actor == actor
                && member.death_at.is_some()
                && member.creature == self.plan.terminal
        }) {
            return None;
        }
        self.transfer.as_ref()?.credit_recipients()
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
        if let Some(recovery) = &self.plan.heal_before_damage
            && let NativeMovementCollisionIndex::Bundle(index) = cells.index()
        {
            for member in &self.members {
                if recovery.creatures.contains(&member.creature)
                    && let Ok(view) = runtime.companion_snapshot_including_dead(member.actor)
                    && let Some(at) = super::super::quest_callbacks::native(view.position)
                    && let Ok(count) = runtime.take_quest_damage_recovery_cues(member.actor)
                {
                    for _ in 0..count.min(512) {
                        announce(
                            runtime,
                            cells,
                            member.actor,
                            view.position,
                            &recovery.announcement,
                        );
                        index.map().facts.emit_movement_effect(at, recovery.effect);
                    }
                }
            }
        }
        while !self.finished {
            let Some((
                index,
                Death {
                    at,
                    now_us: death_us,
                    contributor_at,
                },
            )) = self
                .members
                .iter()
                .enumerate()
                .find_map(|(index, member)| member.death_at.map(|at| (index, at)))
            else {
                break;
            };
            let defeated = &self.members[index].creature;
            if let Some(route) = &self.plan.contributor_route
                && &route.creature == defeated
            {
                // Crystal 201189f8, forgotten_knowledge/creaturescripts_bound_astral_power.lua:15-26.
                let stops: Vec<_> = route
                    .stops
                    .iter()
                    .filter(|stop| {
                        contributor_at.is_some_and(|position| {
                            within_source_distance(position, stop.near, route.maximum_distance)
                        })
                    })
                    .collect();
                if !stops.is_empty() {
                    let health = runtime.companion_policy(defeated)?.maximum_health;
                    let spawn = stops
                        .iter()
                        .map(|stop| {
                            let at = super::super::quest_callbacks::local(stop.spawn_at);
                            if !super::placement(cells, runtime, at) {
                                return Err(CarrierError::PlanConflict);
                            }
                            Ok((route.creature.as_str(), health, at))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    self.members
                        .try_reserve(spawn.len())
                        .map_err(|_| CarrierError::AllocationFailed)?;
                    let next = runtime.commit_quest_creature_spawn(&spawn)?;
                    for stop in stops {
                        announce(
                            runtime,
                            cells,
                            self.members[index].actor,
                            super::super::quest_callbacks::local(stop.near),
                            &route.announcement,
                        );
                    }
                    self.members.extend(next.into_iter().map(|actor| Member {
                        actor,
                        creature: route.creature.clone(),
                        death_at: None,
                    }));
                }
                self.members.swap_remove(index);
                continue;
            }
            if let Some(plan) = &self.plan.crystal_circuit
                && &plan.defeated == defeated
            {
                if let Some(crystal) = self.crystal_charges.crystal(plan, defeated, at) {
                    self.members
                        .try_reserve(plan.crystals.len())
                        .map_err(|_| CarrierError::AllocationFailed)?;
                    let actors = self
                        .crystal_charges
                        .count_death(plan, crystal, runtime, cells)?;
                    self.members.extend(actors.into_iter().map(|actor| Member {
                        actor,
                        creature: plan.spawn.clone(),
                        death_at: None,
                    }));
                }
                if in_death_area(self.room, at)
                    && let Some(vortex) = &plan.vortex
                    && let Some(at) = super::super::quest_callbacks::native(at)
                    && let NativeMovementCollisionIndex::Bundle(index) = cells.index()
                    && let Some(due) = death_us.checked_add(u64::from(vortex.duration_ms) * 1_000)
                {
                    // Crystal 201189f8, creaturescripts_rift_invader_death.lua:56-67.
                    let map = index.map();
                    if !map.facts.house_tile(at)
                        && map.overlay.base().tile(at.x, at.y, at.floor).is_some()
                    {
                        map.facts
                            .create_expiring_movement_item(at, vortex.appearance, due);
                    }
                }
                self.members.swap_remove(index);
                continue;
            }
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
                                .is_ok_and(|position| in_death_area(area, position.position()))
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
            if let Some(delay) = rule.clear_fixed_items_after_ms {
                super::terminal_portal::schedule_fixed_item_cleanup(cells, at, delay, death_us);
            }
            if let Some(announcement) = &rule.announcement {
                // Crystal 201189f8, creaturescripts_death_dragon_essence.lua:11-12
                // and creaturescripts_thorn_knight_death.lua:13-17.
                announce(
                    runtime,
                    cells,
                    self.members[index].actor,
                    announcement
                        .position
                        .map_or(at, super::super::quest_callbacks::local),
                    &announcement.text,
                );
            }
            self.members.swap_remove(index);
            if rule.retire_siblings_in.is_some() {
                self.completed_sweeps.insert(rule.defeated.clone());
            }
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

    /// Crystal 201189f8, ferumbras_ascension/movements_vortex.lua:5-18.
    pub(super) fn consume_vortex_step(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &crate::content::NativeEntryMovementCells,
        map: &crate::map::boot::BundleMap,
        step: crate::map::facts::MonsterStep,
        entries: &[(u64, crate::map::view::EntryFacts)],
        now_us: u64,
    ) -> Result<bool, CarrierError> {
        if !self.is_current(runtime) || self.finished || !in_death_area(self.room, step.to) {
            return Ok(false);
        }
        let Some((plan, vortex)) = self
            .plan
            .crystal_circuit
            .as_ref()
            .and_then(|plan| plan.vortex.as_ref().map(|vortex| (plan, vortex)))
        else {
            return Ok(false);
        };
        let Some(index) = self.members.iter().position(|member| {
            member.actor == step.actor
                && member.death_at.is_none()
                && member.creature == vortex.consume_creature
        }) else {
            return Ok(false);
        };
        let Some(position) = super::super::quest_callbacks::native(step.to) else {
            return Ok(false);
        };
        if map.facts.house_tile(position)
            || !entries.iter().any(|(key, facts)| {
                facts.appearance_id == vortex.appearance
                    && map.facts.live_expiring_movement_item(
                        *key,
                        position,
                        vortex.appearance,
                        now_us,
                    )
            })
        {
            return Ok(false);
        }
        let next = self
            .vortex_consumed
            .checked_add(1)
            .ok_or(CarrierError::PlanConflict)?;
        if next >= vortex.threshold {
            let at = super::super::quest_callbacks::local(plan.boss_position);
            if !super::placement(cells, runtime, at) {
                return Err(CarrierError::PlanConflict);
            }
            let health = runtime.companion_policy(&vortex.spawn)?.maximum_health;
            let actors = runtime.commit_quest_creature_replacement(
                &[step.actor],
                &[(vortex.spawn.as_str(), health, at)],
                || true,
            )?;
            self.members.swap_remove(index);
            self.members.extend(actors.into_iter().map(|actor| Member {
                actor,
                creature: vortex.spawn.clone(),
                death_at: None,
            }));
        } else {
            runtime.remove_encounter_actor(step.actor)?;
            self.members.swap_remove(index);
        }
        self.vortex_consumed = next;
        if let Some(effect) = vortex.consume_effect {
            map.facts.emit_movement_effect(position, effect);
        }
        Ok(true)
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

fn in_death_area(area: DeathArea, position: MovementLocalPosition) -> bool {
    super::super::quest_callbacks::native(position).is_some_and(|at| {
        area.contains(Position {
            x: at.x,
            y: at.y,
            floor: at.floor,
        })
    })
}

fn within_source_distance(at: MovementLocalPosition, center: Position, maximum: u8) -> bool {
    // Crystal 201189f8, src/lua/functions/map/position_functions.cpp:113-118.
    let center = super::super::quest_callbacks::local(center);
    at.x.abs_diff(center.x)
        .max(at.y.abs_diff(center.y))
        .max(u32::from(at.floor.abs_diff(center.floor)))
        <= u32::from(maximum)
}

fn announce(
    runtime: &ChannelRuntimeV1,
    cells: &crate::content::NativeEntryMovementCells,
    speaker: ExactActorRef,
    at: MovementLocalPosition,
    text: &str,
) {
    if let NativeMovementCollisionIndex::Bundle(map) = cells.index()
        && let Some(position) = super::super::quest_callbacks::native(at)
        && let Ok(creature) = runtime.companion_snapshot_including_dead(speaker)
        && let Ok(name) = oteryn_protocol_oteryn::entity_names::EntityDisplayName::new(
            &creature.state.policy.display_name,
        )
    {
        map.map()
            .facts
            .emit_movement_speech(crate::map::facts::MovementSpeech {
                speaker,
                position,
                talk_type: 36,
                text: text.to_owned(),
                captured_name: Some(name),
            });
    }
}
