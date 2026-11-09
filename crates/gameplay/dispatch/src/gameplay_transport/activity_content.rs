//! Instantiate only a qualified template in a fresh independently ready scope.
use super::activity_runtime::{ActivityRuntimeAssets, ActivityTemplateKey};
use super::{ComposedFreshAdmission, SeamRooms};
use crate::durability::activity_handoff::boss_room::BossRoomDefinition;
use crate::durability::activity_handoff::{ActivityHandoffOwner, ActivityRefusal};
use crate::foundation::{ChannelRuntimeV1, RuntimeScopeRefV1};
mod capacity;

impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    /// A pinned source map uses its own frame. Allocation and readiness still
    /// come only from the existing control owner; this method creates neither.
    pub(super) async fn ensure_pinned_source_runtime(
        &self,
        source: crate::content::bosses::activity_template::PinnedActivitySourceRoom,
        initial_boss: Option<(&str, crate::foundation::MovementLocalPosition)>,
        capacity: usize,
    ) -> Result<super::activity_runtime::RegisteredRuntime<'a>, ActivityRefusal> {
        let _transition = self.activity_runtimes.transitions.lock().await;
        let room = self
            .activity_runtimes
            .source_room(source)
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        let active = self
            .active_generation
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        let native = active
            .native_gameplay()
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        let source_scope = {
            let runtime = self.runtime.lock().await;
            runtime
                .owner_fence()
                .map_err(|_| ActivityRefusal::AuthorityRejected)?;
            runtime.binding().runtime_scope()
        };
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = source_scope
        else {
            return Err(ActivityRefusal::AuthorityRejected);
        };
        let current = self
            .root
            .read_current_content_activation(world_id, channel_id)
            .await
            .map_err(|_| ActivityRefusal::AuthorityRejected)?
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        if current.activation_sequence != active.activation_sequence()
            || current.server_artifact_digest != active.identity().server_artifact_digest()
            || current.client_artifact_digest != active.identity().client_artifact_digest()
            || room.compiled().server_digest() != active.identity().server_artifact_digest()
            || room.compiled().client_digest() != active.identity().client_artifact_digest()
        {
            return Err(ActivityRefusal::ContentChanged);
        }
        let template = ActivityTemplateKey {
            key: format!("oteryn:room.{}-source", source.name()),
            revision: "crystal-201189f".into(),
        };
        let owner = ActivityHandoffOwner::new(self.root.clone());
        let mut after = None;
        loop {
            let allocations = owner
                .ready_allocations(self.character, self.holder, world_id, after)
                .await
                .map_err(|_| ActivityRefusal::AuthorityRejected)?;
            for allocation in &allocations {
                if let Some(registered) = self
                    .activity_runtimes
                    .get(allocation.scope, allocation.generation.get())
                {
                    if registered.template.as_ref() == Some(&template)
                        && registered.assignment_source_revision == allocation.source_revision
                    {
                        let ready = {
                            let runtime = registered.runtime.lock().await;
                            let pin = runtime.content_pin();
                            runtime.owner_fence().is_ok()
                                && pin.server_artifact_digest()
                                    == active.identity().server_artifact_digest()
                                && pin.client_artifact_digest()
                                    == active.identity().client_artifact_digest()
                                && pin.frame_binding_digest() == room.frame_binding().digest()
                                && runtime.map_revision_digest() == room.map_revision_digest()
                                && runtime
                                    .activity_participant_sessions()
                                    .is_ok_and(|sessions| sessions.is_empty())
                                && runtime.positioned_actor_census().is_ok_and(|census| {
                                    match initial_boss {
                                        Some((key, at)) => {
                                            census.len() == 1
                                                && census.iter().any(
                                                    |(actor, position, session)| {
                                                        session.is_none()
                                                            && position.position() == at
                                                            && runtime
                                                                .companion_snapshot(*actor)
                                                                .is_ok_and(|view| {
                                                                    view.state.policy.definition_key
                                                                        == key
                                                                })
                                                    },
                                                )
                                        }
                                        None => census.is_empty(),
                                    }
                                })
                        };
                        if ready {
                            return Ok(registered);
                        }
                    }
                    continue;
                }
                let assignment = self
                    .root
                    .read_runtime_scope_assignment(allocation.scope)
                    .await
                    .map_err(|_| ActivityRefusal::AuthorityRejected)?
                    .ok_or(ActivityRefusal::NoCapacity)?;
                if assignment.ownership_generation != allocation.generation.get()
                    || assignment.source_revision != allocation.source_revision
                {
                    continue;
                }
                let owned_room = self
                    .activity_runtimes
                    .fresh_source_room(source, active, world_id, channel_id)
                    .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                let room = owned_room.as_ref();
                let start = room.entry_start();
                let pin = crate::foundation::ChannelContentPin::from_activation(
                    world_id,
                    active.activation_sequence(),
                    active.identity().server_artifact_digest(),
                    active.identity().client_artifact_digest(),
                    room.frame_binding().digest(),
                    room.map_revision_digest(),
                    (start.x, start.y, start.floor),
                );
                let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
                    allocation.scope,
                    self.holder.fact().node_id(),
                    self.holder.fact().registration_revision(),
                    assignment.ownership_generation,
                    assignment.source_revision,
                    &assignment.decision_identity,
                    capacity,
                    pin,
                )
                .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                runtime
                    .install_companion_policies(native.policies().clone())
                    .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                let mut source_callbacks = None;
                if let Some((key, position)) = initial_boss {
                    let spawned_at = self.owner_now().get();
                    let boss = runtime
                        .spawn_arena_opponent(key, position, |runtime, at| {
                            Some(
                                room.movement_cells().index().lookup(
                                    room.movement_cells().scope(),
                                    crate::content::LogicalCell {
                                        x: at.x,
                                        y: at.y,
                                        z: i32::from(at.floor),
                                    },
                                ) == Ok(crate::content::CollisionClass::Walkable)
                                    && runtime.native_summon_cell_free(at),
                            )
                        })
                        .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                    if matches!(source,
                        crate::content::bosses::activity_template::PinnedActivitySourceRoom::Vemiath
                        | crate::content::bosses::activity_template::PinnedActivitySourceRoom::Chagorz
                        | crate::content::bosses::activity_template::PinnedActivitySourceRoom::Bakragore) {
                        runtime.register_rotten_movement_source(native, room, boss, spawned_at)
                            .map_err(|_|ActivityRefusal::ContentUnavailable)?;
                    }
                    if source == crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsGreed {
                        source_callbacks = Some(super::soul_war_callbacks::SoulWarCallbacks::initialize_greed(
                            &mut runtime, native, room, boss, spawned_at,
                        ).map_err(|_| ActivityRefusal::ContentUnavailable)?);
                    }
                    if source == crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsMalice {
                        source_callbacks = Some(super::soul_war_callbacks::SoulWarCallbacks::initialize_malice(
                            &mut runtime, native, room, boss, spawned_at,
                        ).map_err(|_| ActivityRefusal::ContentUnavailable)?);
                    }
                    if source == crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsCruelty {
                        source_callbacks = Some(super::soul_war_callbacks::SoulWarCallbacks::initialize_cruelty(
                            &mut runtime, native, room, boss, spawned_at,
                        ).map_err(|_| ActivityRefusal::ContentUnavailable)?);
                    }
                    if source == crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsMegalomania {
                        source_callbacks = Some(super::soul_war_callbacks::SoulWarCallbacks::initialize_megalomania(
                            &mut runtime,native,room,boss,spawned_at,
                        ).map_err(|_|ActivityRefusal::ContentUnavailable)?);
                    }
                    if source == crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsHatred {
                        source_callbacks = Some(super::soul_war_callbacks::SoulWarCallbacks::initialize_hatred(
                            &mut runtime, native, room, boss, spawned_at,
                        ).map_err(|_| ActivityRefusal::ContentUnavailable)?);
                    }
                }
                let reference =
                    crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
                        .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                let fence = crate::world_runtime::ScopeContentGenerationFence::for_activation(
                    allocation.scope,
                    allocation.generation,
                    reference,
                );
                let door = crate::world_runtime::bind_native_entry_door(
                    room.door(),
                    &fence,
                    allocation.scope,
                    allocation.generation,
                )
                .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                self.register_source_activity_runtime(
                    template.clone(),
                    &assignment,
                    runtime,
                    door,
                    ActivityRuntimeAssets {
                        movement_cells: std::sync::Arc::new(room.movement_cells().clone()).into(),
                        // This sealed source runtime owns its freshly scoped map;
                        // source qualification never opens unrelated origin gates.
                        rooms: SeamRooms::owned(std::sync::Arc::clone(&owned_room), false),
                    },
                    source_callbacks,
                    owned_room,
                )
                .await
                .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                return self
                    .activity_runtimes
                    .get(allocation.scope, allocation.generation.get())
                    .ok_or(ActivityRefusal::NoCapacity);
            }
            if allocations.len() < 15 {
                return Err(ActivityRefusal::NoCapacity);
            }
            let Some(RuntimeScopeRefV1::Instance { instance_id, .. }) =
                allocations.last().map(|allocation| allocation.scope)
            else {
                return Err(ActivityRefusal::NoCapacity);
            };
            if after.is_some_and(|previous| previous >= instance_id) {
                return Err(ActivityRefusal::AuthorityRejected);
            }
            after = Some(instance_id);
        }
    }

    pub(super) async fn ensure_activity_template_runtime(
        &self,
        definition: &BossRoomDefinition,
    ) -> Result<(), ActivityRefusal> {
        if super::primal_entry::is_primal(&definition.boss_key) {
            return self.ensure_primal_definition_runtime(definition).await;
        }
        let _transition = self.activity_runtimes.transitions.lock().await;
        let Some(template) = self
            .activity_runtimes
            .template(&definition.room_template_key, &definition.template_revision)
        else {
            // Isolated PG fixtures supply their own real activated owner. This
            // bypass does not exist in a production build.
            #[cfg(test)]
            {
                let present = self
                    .activity_runtimes
                    .activity_owners()
                    .map_err(|_| ActivityRefusal::AuthorityRejected)?
                    .iter()
                    .any(|owner| {
                        owner.template.as_ref().is_some_and(|key| {
                            key.key == definition.room_template_key
                                && key.revision == definition.template_revision
                        })
                    });
                if present {
                    return Ok(());
                }
            }
            return Err(ActivityRefusal::ContentUnavailable);
        };
        // Selected recipe record23 s5/s6: do not instantiate the timed room
        // without its actual once-only native death/completion consumer.
        if template.source_path() == "content/encounters/bosses/selected-first-dragon.json" {
            let key = "oteryn:quest-transition/native-kills/first-dragon-selected";
            let counter = oteryn_gameplay_quests::quest::native_kills::counter(key)
                .ok_or(ActivityRefusal::ContentUnavailable)?;
            let catalogue = self
                .quest_catalogue
                .as_ref()
                .ok_or(ActivityRefusal::ContentUnavailable)?;
            if counter.creature != template.creature_key()
                || counter.gate_value() != 2
                || counter.maximum != 1
                || counter.completion_track().is_none()
                || catalogue
                    .transition(key)
                    .is_none_or(|transition| !transition.completes)
            {
                return Err(ActivityRefusal::ContentUnavailable);
            }
        }
        let cell = |p: crate::content::LogicalCell| {
            crate::durability::character_last_position::LastPositionCell {
                x: p.x,
                y: p.y,
                floor: p.z as i16,
            }
        };
        if template.boss_key() != definition.boss_key
            || template.creature_key() != definition.boss_creature_key
            || template.content_revision() != definition.content_revision
            || template.minimum_players() as usize != definition.policy.minimum_players
            || template.maximum_players() != definition.policy.maximum_players
            || template.minimum_level() != definition.policy.minimum_level
            || u64::from(template.cooldown_seconds()) * 1000 != definition.policy.cooldown_ms
            || u64::from(template.time_limit_seconds()) * 1000 != definition.policy.time_limit_ms
            || u64::from(template.after_kill_seconds()) * 1000 != definition.after_kill_ms
            || template
                .lever_positions()
                .iter()
                .copied()
                .map(cell)
                .collect::<Vec<_>>()
                != definition.lever_positions
            || template
                .entry_positions()
                .iter()
                .copied()
                .map(cell)
                .collect::<Vec<_>>()
                != definition.entry_positions
            || cell(template.exit_position()) != definition.exit_position
            || cell(template.boss_position()) != definition.boss_position
        {
            return Err(ActivityRefusal::ContentChanged);
        }
        let active = self
            .active_generation
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        let native = active
            .native_gameplay()
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        let source_scope = {
            let runtime = self.runtime.lock().await;
            if runtime.content_pin().frame_binding_digest()
                != template.room().frame_binding().digest()
                || runtime.content_pin().map_revision_digest()
                    != template.room().map_revision_digest()
                || runtime.content_pin().server_artifact_digest()
                    != active.identity().server_artifact_digest()
                || runtime.content_pin().client_artifact_digest()
                    != active.identity().client_artifact_digest()
            {
                return Err(ActivityRefusal::ContentChanged);
            }
            runtime.binding().runtime_scope()
        };
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = source_scope
        else {
            return Err(ActivityRefusal::AuthorityRejected);
        };
        let issued = self
            .root
            .read_current_content_activation(world_id, channel_id)
            .await
            .map_err(|_| ActivityRefusal::AuthorityRejected)?
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        if issued.activation_sequence != active.activation_sequence()
            || issued.server_artifact_digest != active.identity().server_artifact_digest()
            || issued.client_artifact_digest != active.identity().client_artifact_digest()
            || issued.frame_binding_digest != template.activation_frame_binding_digest()
        {
            return Err(ActivityRefusal::ContentChanged);
        }
        let owner = ActivityHandoffOwner::new(self.root.clone());
        let mut after = None;
        loop {
            let candidates = owner
                .ready_allocations(self.character, self.holder, world_id, after)
                .await
                .map_err(|_| ActivityRefusal::AuthorityRejected)?;
            for allocation in &candidates {
                if let Some(existing) = self
                    .activity_runtimes
                    .get(allocation.scope, allocation.generation.get())
                {
                    if existing.template.as_ref().is_some_and(|key| {
                        key.key == definition.room_template_key
                            && key.revision == definition.template_revision
                    }) && super::native_activity::boss_room_native_ready(
                        &*existing.runtime.lock().await,
                        definition,
                    ) {
                        return Ok(());
                    }
                    continue;
                }
                let Some(assignment) = self
                    .root
                    .read_runtime_scope_assignment(allocation.scope)
                    .await
                    .map_err(|_| ActivityRefusal::AuthorityRejected)?
                else {
                    continue;
                };
                if assignment.ownership_generation != allocation.generation.get()
                    || assignment.source_revision != allocation.source_revision
                {
                    continue;
                }
                let pin = template
                    .issue_runtime_pin(active)
                    .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
                    allocation.scope,
                    self.holder.fact().node_id(),
                    self.holder.fact().registration_revision(),
                    assignment.ownership_generation,
                    assignment.source_revision,
                    &assignment.decision_identity,
                    capacity::for_template(template)?,
                    pin,
                )
                .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                runtime
                    .install_companion_policies(native.policies().clone())
                    .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                runtime
                    .realize_activity_boss(template, active)
                    .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                let RuntimeScopeRefV1::Instance { instance_id, .. } = allocation.scope else {
                    continue;
                };
                let room = std::sync::Arc::new(
                    template
                        .room()
                        .for_activity_instance(instance_id)
                        .ok_or(ActivityRefusal::ContentUnavailable)?,
                );
                let reference =
                    crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
                        .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                let fence = crate::world_runtime::ScopeContentGenerationFence::for_activation(
                    allocation.scope,
                    allocation.generation,
                    reference,
                );
                let door = crate::world_runtime::bind_native_entry_door(
                    room.door(),
                    &fence,
                    allocation.scope,
                    allocation.generation,
                )
                .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                self.register_activity_runtime(
                    ActivityTemplateKey {
                        key: template.template_key().into(),
                        revision: template.template_revision().into(),
                    },
                    &assignment,
                    runtime,
                    door,
                    ActivityRuntimeAssets {
                        movement_cells: std::sync::Arc::new(room.movement_cells().clone()).into(),
                        // Keep the origin qualification gate: a private Instance
                        // cannot open source-room mechanics for a bundle origin.
                        rooms: SeamRooms::owned(room, self.rooms.gate_source_world().is_some()),
                    },
                )
                .await
                .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                return Ok(());
            }
            if candidates.len() < 15 {
                return Err(ActivityRefusal::NoCapacity);
            }
            let Some(RuntimeScopeRefV1::Instance { instance_id, .. }) =
                candidates.last().map(|candidate| candidate.scope)
            else {
                return Err(ActivityRefusal::NoCapacity);
            };
            if after.is_some_and(|old| old >= instance_id) {
                return Err(ActivityRefusal::AuthorityRejected);
            }
            after = Some(instance_id);
        }
    }
}
