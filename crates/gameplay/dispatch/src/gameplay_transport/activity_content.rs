//! Instantiate only a qualified template in a fresh independently ready scope.
use super::activity_runtime::{ActivityRuntimeAssets, ActivityTemplateKey};
use super::{ComposedFreshAdmission, SeamRooms};
use crate::durability::activity_handoff::boss_room::BossRoomDefinition;
use crate::durability::activity_handoff::{ActivityHandoffOwner, ActivityRefusal};
use crate::foundation::{ChannelRuntimeV1, RuntimeScopeRefV1};

impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    pub(super) async fn ensure_activity_template_runtime(
        &self,
        definition: &BossRoomDefinition,
    ) -> Result<(), ActivityRefusal> {
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
            || issued.frame_binding_digest != template.room().frame_binding().digest()
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
                    template.maximum_players() + 1,
                    pin,
                )
                .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                runtime
                    .install_companion_policies(native.policies().clone())
                    .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                runtime
                    .realize_activity_boss(template, active)
                    .map_err(|_| ActivityRefusal::ContentUnavailable)?;
                let room = template.room();
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
                        movement_cells: room.movement_cells(),
                        // Keep the origin qualification gate: a private Instance
                        // cannot open source-room mechanics for a bundle origin.
                        rooms: SeamRooms::new(
                            self.rooms.gate_source_world().map(|_| room),
                            Some(room),
                        ),
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
