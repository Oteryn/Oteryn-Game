//! Authenticated activity-session routing and authoritative scope projection.
//! Historical links and authored cells never restore control or reset live state.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::activity_handoff::ActivityHandoffOwner;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{GameSessionState, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::activity_scope::{
    ActivityScope, ActivityScopeSnapshot, encode_activity_scope,
};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn note_activity_input(&self, admitted: AdmittedSession) -> bool {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1)
        {
            return true;
        }
        let (Some(fence), Some(controller)) = (admitted.item_fence, admitted.controller) else {
            return false;
        };
        ActivityHandoffOwner::new(self.root.clone())
            .note_transfer_input(self.holder, fence, controller.transport)
            .await
            .is_ok()
    }

    /// After a real ClientResume this is the committed binding projected in the
    /// initial full snapshot. The same domain is a source-stream barrier during
    /// a live handoff; initial projection never resets its new stream twice.
    pub(super) async fn project_activity_scope(
        &self,
        admitted: AdmittedSession,
    ) -> Result<Option<Vec<u8>>, ()> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1)
        {
            return Ok(None);
        }
        let (Some(fence), Some(controller), Some(actor)) = (
            admitted.item_fence,
            admitted.controller,
            admitted.runtime_actor,
        ) else {
            return Ok(None);
        };
        // House's older virtual custody remains its own accepted protocol path.
        if actor.runtime_scope() != fence.runtime_scope {
            return Ok(None);
        }
        let handoffs = ActivityHandoffOwner::new(self.root.clone());
        let Some(candidate) = handoffs
            .transfer_recovery(admitted.game_session_id)
            .await
            .map_err(|_| ())?
        else {
            return Ok(None);
        };
        if candidate.current_session != admitted.game_session_id
            || candidate.runtime_scope != fence.runtime_scope
            || candidate.scope_generation != fence.scope_ownership_generation
            || candidate.connection_generation != admitted.continuity.connection_generation
            || candidate.lease_generation != fence.character_lease_generation
            || candidate.account != controller.account_id
            || candidate.current.character_id.as_bytes() != fence.character_id.as_bytes()
            || candidate.selected_capabilities.as_slice()
                != admitted.continuity.selected_capabilities.as_slice()
        {
            return Err(());
        }
        let (current, _) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(admitted.game_session_id)
            .await
            .map_err(|_| ())?;
        if super::item_fence_of(current) != Some(fence)
            || current.session_state() != GameSessionState::Active
            || current.current_transport() != Some(controller.transport)
        {
            return Err(());
        }
        let scoped = self.resolve_runtime_view(admitted).await?;
        let owner = scoped.as_ref().unwrap_or(self);
        if !owner.required_capabilities.matches_current(
            fence.runtime_scope,
            fence.scope_ownership_generation.get(),
            &candidate.current.world_policy_revision,
        ) {
            return Err(());
        }
        let runtime = owner.runtime.lock().await;
        runtime.owner_fence().map_err(|_| ())?;
        let control = runtime
            .player_control_facts(actor, admitted.game_session_id)
            .map_err(|_| ())?;
        let lease = runtime
            .bound_attacker_lease(actor, admitted.game_session_id)
            .map_err(|_| ())?;
        if control.control_loss.is_some()
            || lease.generation() != fence.character_lease_generation
            || lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || runtime.content_pin().server_artifact_digest()
                != candidate.destination_content.server_artifact_digest
            || runtime.content_pin().frame_binding_digest()
                != candidate.destination_content.coordinate_frame_digest
            || runtime.map_revision_digest() != candidate.destination_content.map_revision_digest
        {
            return Err(());
        }
        let scope = match fence.runtime_scope {
            RuntimeScopeRefV1::Channel {
                world_id,
                channel_id,
            } => ActivityScope::Channel {
                world: *world_id.as_bytes(),
                channel: *channel_id.as_bytes(),
            },
            RuntimeScopeRefV1::Instance {
                world_id,
                instance_id,
            } => ActivityScope::Instance {
                world: *world_id.as_bytes(),
                instance: instance_id,
            },
        };
        encode_activity_scope(&ActivityScopeSnapshot {
            handoff: candidate.handoff_id,
            source_session: *candidate.source_session.as_bytes(),
            session: *admitted.game_session_id.as_bytes(),
            scope,
            scope_generation: fence.scope_ownership_generation.get(),
            connection_generation: admitted.continuity.connection_generation,
            lease_generation: fence.character_lease_generation,
            selection: candidate.selected_capabilities,
        })
        .map(Some)
        .map_err(|_| ())
    }
}
