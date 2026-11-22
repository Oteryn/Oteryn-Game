//! QUICKLOOT-1: manual commands resolve session handles before the durable item loop.
use super::{
    ComposedFreshAdmission, FreshAdmissionStore, GameSessionState,
    connection::AdmittedSession,
    item_fence_of,
    item_view::{ItemKey, SessionItemView},
};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, CommandStatus, MovementLocalPosition,
};
use oteryn_protocol_oteryn::quick_loot::{QuickLootRequest, QuickLootResult, decode_target_handle};

macro_rules! authority_methods {
    () => {
        fn quick_loot(
            &self,
            _admitted: AdmittedSession,
            _command_id: u64,
            _request: oteryn_protocol_oteryn::quick_loot::QuickLootRequest,
            _item_view: Option<&crate::gameplay_transport::item_view::SessionItemView>,
        ) -> impl std::future::Future<
            Output = Option<(
                crate::foundation::CommandStatus,
                oteryn_protocol_oteryn::quick_loot::QuickLootResult,
            )>,
        > {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn quick_loot(
            &self,
            admitted: connection::AdmittedSession,
            command_id: u64,
            request: oteryn_protocol_oteryn::quick_loot::QuickLootRequest,
            item_view: Option<&crate::gameplay_transport::item_view::SessionItemView>,
        ) -> Option<(
            crate::foundation::CommandStatus,
            oteryn_protocol_oteryn::quick_loot::QuickLootResult,
        )> {
            self.execute_quick_loot(admitted, command_id, request, item_view)
                .await
        }
    };
}
pub(super) use composed_methods;

fn resolve_targets(
    runtime: &ChannelRuntimeV1,
    position: MovementLocalPosition,
    view: Option<&SessionItemView>,
    mut request: QuickLootRequest,
) -> Option<QuickLootRequest> {
    if let Some(handle) = request.corpse {
        let ItemKey::Entity(identity) = view?.resolve(decode_target_handle(handle)?)? else {
            return None;
        };
        let corpse = runtime.bound_corpse(identity)?;
        if corpse.position.floor != position.floor
            || corpse.position.x.abs_diff(position.x) > 1
            || corpse.position.y.abs_diff(position.y) > 1
        {
            return None;
        }
        request.corpse = Some(corpse.item.item_instance_id);
    }
    if let Some(handle) = request.recipient_id {
        let ItemKey::Instance(instance) = view?.resolve(decode_target_handle(handle)?)? else {
            return None;
        };
        // The item owner checks current carried custody and container policy in its transaction.
        request.recipient_id = Some(instance);
    }
    Some(request)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn execute_quick_loot(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
        request: QuickLootRequest,
        item_view: Option<&SessionItemView>,
    ) -> Option<(CommandStatus, QuickLootResult)> {
        let rejected = || {
            Some((
                CommandStatus::Rejected,
                QuickLootResult { items: Vec::new() },
            ))
        };
        let (Some(fence), Some(actor), Some(controller)) = (
            admitted.item_fence,
            admitted.runtime_actor,
            admitted.controller,
        ) else {
            return rejected();
        };
        let session = admitted.game_session_id;
        let permit = self.spell_lane_permit().await?;
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let (current, _) = super::AdmissionRetryBudget::for_command(self.root)
            .retry_durability("quick_loot_session", async || {
                store.current_session_at(session).await
            })
            .await
            .ok()?;
        let (content, position, request) = {
            let runtime = self.runtime.lock().await;
            if current.session_state() != GameSessionState::Active
                || current.current_transport() != Some(controller.transport)
                || item_fence_of(current) != Some(fence)
                || fence.game_session_id != session
                || fence.connection_generation.get() != admitted.continuity.connection_generation
                || fence.runtime_scope != actor.runtime_scope()
                || fence.scope_ownership_generation != actor.scope_generation()
                || !runtime
                    .player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return rejected();
            }
            let states = self.states.spell_states.lock().await;
            let content = super::equipment_move::native_for_move(
                &runtime,
                self.active_generation,
                states.is_dead(actor),
                states.has_pending_spell_commit(actor, session),
            )?;
            let position = runtime.read_actor_position(actor).ok()?.position();
            let Some(request) = resolve_targets(&runtime, position, item_view, request) else {
                return rejected();
            };
            (content, position, request)
        };
        let cell = [
            position.x.to_be_bytes().as_slice(),
            position.y.to_be_bytes().as_slice(),
            position.floor.to_be_bytes().as_slice(),
        ]
        .concat()
        .try_into()
        .ok()?;
        let premium = self.session_premium_gate(session).await.require().is_ok();
        let result = self
            .root
            .commit_quick_loot(
                &permit,
                self.character,
                self.holder,
                fence,
                CommandRef::new(session, CommandId::new(command_id).ok()?),
                &super::item_ref_admission::NativeItemWeights(content)
                    .with_wheel(self.premium_coordinator),
                cell,
                premium,
                request,
            )
            .await
            .ok()?;
        match result {
            Ok(result) => Some((CommandStatus::Accepted, result)),
            Err(_) => rejected(),
        }
    }
}
