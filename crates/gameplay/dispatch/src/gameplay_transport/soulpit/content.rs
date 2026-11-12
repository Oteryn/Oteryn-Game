//! Qualify a private Soulpit child of the activated full World bundle.
use super::super::activity_runtime::{ActivityRuntimeAssets, ActivityTemplateKey, SharedOwner};
use super::super::{AdmissionRetryBudget, ComposedFreshAdmission, SeamRooms};
use super::handoff_owner::read_with_budget;
use crate::content::soulpit_activity::{
    SOULPIT_BOUNDS, SOULPIT_TEMPLATE_KEY, SOULPIT_TEMPLATE_REVISION,
};
use crate::durability::activity_handoff::{ActivityHandoffOwner, ActivityRefusal};
use crate::foundation::{ChannelRuntimeV1, RuntimeScopeRefV1};
use crate::map::boot::BundleCollisionIndexExt;
use std::sync::Arc;

impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    pub(super) async fn ensure_soulpit_template_runtime(
        &self,
        retry: &AdmissionRetryBudget<'_>,
    ) -> Result<(), ActivityRefusal> {
        let _transition = self.activity_runtimes.transitions.lock().await;
        let template = self
            .activity_runtimes
            .soulpit_template()
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        let active = self
            .active_generation
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        let native = active
            .native_gameplay()
            .ok_or(ActivityRefusal::ContentUnavailable)?;
        let source_scope = {
            let runtime = self.runtime.lock().await;
            let pin = runtime.content_pin();
            if runtime.owner_fence().is_err()
                || pin.frame_binding_digest() != template.room().frame_binding().digest()
                || pin.map_revision_digest() != template.room().map_revision_digest()
                || pin.server_artifact_digest() != active.identity().server_artifact_digest()
                || pin.client_artifact_digest() != active.identity().client_artifact_digest()
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
        let issued = read_with_budget(retry, async || {
            self.root
                .read_current_content_activation(world_id, channel_id)
                .await
        })
        .await
        .map_err(|_| ActivityRefusal::AuthorityRejected)?
        .ok_or(ActivityRefusal::ContentUnavailable)?;
        if issued.activation_sequence != active.activation_sequence()
            || issued.server_artifact_digest != active.identity().server_artifact_digest()
            || issued.client_artifact_digest != active.identity().client_artifact_digest()
        {
            return Err(ActivityRefusal::ContentChanged);
        }
        let owner = ActivityHandoffOwner::new(self.root.clone());
        let mut after = None;
        loop {
            let candidates = read_with_budget(retry, async || {
                owner
                    .ready_allocations(self.character, self.holder, world_id, after)
                    .await
            })
            .await
            .map_err(|_| ActivityRefusal::AuthorityRejected)?;
            for allocation in &candidates {
                if let Some(existing) = self
                    .activity_runtimes
                    .get(allocation.scope, allocation.generation.get())
                {
                    if existing.template.as_ref().is_some_and(|key| {
                        key.key == SOULPIT_TEMPLATE_KEY && key.revision == SOULPIT_TEMPLATE_REVISION
                    }) && existing.runtime.lock().await.owner_fence().is_ok()
                    {
                        return Ok(());
                    }
                    continue;
                }
                let Some(assignment) = read_with_budget(retry, async || {
                    self.root
                        .read_runtime_scope_assignment(allocation.scope)
                        .await
                        .map_err(|error| match error {
                            crate::durability::runtime_scope_assignment::AssignmentError::Unavailable(error) => error,
                            // Only the nested RootUnavailable is a busy root. All typed
                            // assignment refusals remain fail-closed, without retry.
                            _ => crate::durability::DurabilityError::Unavailable,
                        })
                })
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
                // Crystal soulpit.lua:31-84: five entrants, seven actors per wave.
                // Reuse the Channel's accepted capacity so donor summons have the
                // same bounded pool rather than a second, unbounded actor owner.
                let capacity = self.runtime.lock().await.actor_capacity();
                if capacity
                    < oteryn_gameplay_domain::domain::soulpit::MAX_PARTICIPANTS
                        + oteryn_gameplay_domain::domain::soulpit::MONSTERS_PER_WAVE
                {
                    // Refuse before debit rather than let resource exhaustion
                    // turn all seven donor spawn attempts into empty waves.
                    return Err(ActivityRefusal::NoCapacity);
                }
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
                let room = template.room();
                let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
                    room.movement_cells().index()
                else {
                    return Err(ActivityRefusal::ContentUnavailable);
                };
                let private_index = index
                    .fork_activity(allocation.scope, SOULPIT_BOUNDS)
                    .ok_or(ActivityRefusal::ContentUnavailable)?;
                let private_room = Arc::new(
                    room.with_activity_bundle_index(private_index, SOULPIT_BOUNDS)
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
                        key: SOULPIT_TEMPLATE_KEY.into(),
                        revision: SOULPIT_TEMPLATE_REVISION.into(),
                    },
                    &assignment,
                    runtime,
                    door,
                    ActivityRuntimeAssets {
                        movement_cells: SharedOwner::Owned(Arc::new(
                            private_room.movement_cells().clone(),
                        )),
                        rooms: SeamRooms::owned(private_room, false),
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
