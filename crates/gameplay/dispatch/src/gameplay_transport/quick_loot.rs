//! QUICKLOOT: reconcile the original wire binding before consulting session handles.
use super::{
    ComposedFreshAdmission, FreshAdmissionStore, GameSessionState,
    connection::AdmittedSession,
    item_fence_of,
    item_view::{ItemKey, SessionItemView, View},
};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, CommandStatus, MovementLocalPosition,
};
use super::session_ingress::QuickLootWire;
use oteryn_protocol_oteryn::quick_loot::{
    COMMAND_TYPE_QUICK_LOOT_V2, QuickLootRequest, QuickLootResult, decode_target_handle,
    encode_quick_loot_result,
};

macro_rules! authority_methods {
    () => {
        fn quick_loot(
            &self,
            _admitted: AdmittedSession,
            _command_id: u64,
            _request: oteryn_protocol_oteryn::quick_loot::QuickLootRequest,
            _wire: crate::gameplay_transport::session_ingress::QuickLootWire,
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
            wire: crate::gameplay_transport::session_ingress::QuickLootWire,
            item_view: Option<&crate::gameplay_transport::item_view::SessionItemView>,
        ) -> Option<(
            crate::foundation::CommandStatus,
            oteryn_protocol_oteryn::quick_loot::QuickLootResult,
        )> {
            self.execute_quick_loot(admitted, command_id, request, wire, item_view)
                .await
        }
    };
}
pub(super) use composed_methods;

fn resolve_targets(
    runtime: &ChannelRuntimeV1,
    position: MovementLocalPosition,
    view: Option<&SessionItemView>,
    command_type: u32,
    mut request: QuickLootRequest,
) -> Option<QuickLootRequest> {
    if command_type != COMMAND_TYPE_QUICK_LOOT_V2 {
        return Some(request);
    }
    if let Some(handle) = request.corpse {
        let view = view?.table();
        let handle = decode_target_handle(handle)?;
        request.corpse = Some(match view.resolve(handle)? {
            ItemKey::Entity(identity) => {
                let corpse = runtime.bound_corpse(identity)?;
                if corpse.position.floor != position.floor
                    || corpse.position.x.abs_diff(position.x) > 1
                    || corpse.position.y.abs_diff(position.y) > 1
                {
                    return None;
                }
                corpse.item.item_instance_id
            }
            // A bundle publishes durable Ground even when the runtime no longer
            // retains a corpse projection. The item transaction verifies the exact
            // instance, current scope, corpse mint receipt and reach.
            ItemKey::Instance(instance) if view.resolve_in_view(handle, View::Map).is_some() => {
                instance
            }
            _ => return None,
        });
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
        wire: QuickLootWire,
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
        if fence.game_session_id != session
            || fence.connection_generation.get() != admitted.continuity.connection_generation
            || fence.runtime_scope != actor.runtime_scope()
            || fence.scope_ownership_generation != actor.scope_generation()
        {
            return rejected();
        }
        let command = CommandRef::new(session, CommandId::new(command_id).ok()?);
        let permit = self.spell_lane_permit().await?;
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let (current, _) = super::AdmissionRetryBudget::for_command(self.root)
            .retry_durability("quick_loot_session", async || {
                store.current_session_at(session).await
            })
            .await
            .ok()?;
        if current.session_state() != GameSessionState::Active
                || current.current_transport() != Some(controller.transport)
                || item_fence_of(current) != Some(fence)
        {
            return rejected();
        }
        self.states.command_ingress.retain_quick_loot(command, wire)?;
        if let Some(result) = self.root
            .read_quick_loot_result(fence.character_id, command, wire.command_type, wire.payload())
            .await.ok()?
        {
            return match result {
                Ok(result) => Some((CommandStatus::Accepted, result)),
                Err(_) => rejected(),
            };
        }
        let (content, position, request) = {
            let runtime = self.runtime.lock().await;
            if !runtime.player_control_facts(actor, session).is_ok_and(|facts| facts.control_loss.is_none()) {
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
            let Some(request) = resolve_targets(&runtime, position, item_view, wire.command_type, request) else {
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
                command,
                &super::item_ref_admission::NativeItemWeights(content)
                    .with_wheel(self.premium_coordinator),
                cell,
                premium,
                crate::durability::item_loop_move::QuickLootInput {
                    request,
                    command_type: wire.command_type,
                    payload: wire.payload().to_vec(),
                },
            )
            .await
            .ok()?;
        match result {
            Ok(result) => Some((CommandStatus::Accepted, result)),
            Err(_) => rejected(),
        }
    }

    pub(super) async fn recover_pending_quick_loot(&self, admitted: &AdmittedSession) -> bool {
        let ingress = &self.states.command_ingress;
        let Some((command, wire)) = ingress.pending_quick_loot(admitted.game_session_id) else {
            return true;
        };
        let Some(fence) = admitted.item_fence else {
            return false;
        };
        let Ok(result) = self.root
            .read_quick_loot_result(fence.character_id, command, wire.command_type, wire.payload())
            .await else {
            return false;
        };
        let accepted = matches!(result, Some(Ok(_)));
        let result = result.and_then(Result::ok).unwrap_or(QuickLootResult { items: Vec::new() });
        encode_quick_loot_result(&result).ok().is_some_and(|payload|
            ingress.complete_quick_loot(command, wire, !accepted, &payload).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};
    use oteryn_protocol_oteryn::quick_loot::QuickLootAction;

    #[test]
    fn quick_loot_v1_click_preserves_the_wire_corpse_uuid() -> Result<(), String> {
        let id = |last| [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, last];
        let world = WorldId::decode(&id(1)).map_err(|error| format!("{error:?}"))?;
        let runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).map_err(|error| format!("{error:?}"))?,
            NodeId::decode(&id(3)).map_err(|error| format!("{error:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            ChannelContentPin::test(world),
        )
        .map_err(|error| format!("{error:?}"))?;
        let corpse = id(4);
        let request = QuickLootRequest {
            action: QuickLootAction::Click,
            corpse: Some(corpse),
            category: None,
            recipient: None,
            recipient_id: None,
            fallback_to_main: false,
            filter_mode: None,
            filter_definition_ref: None,
        };
        let position = MovementLocalPosition { x: 1, y: 1, floor: 7 };
        assert_eq!(
            resolve_targets(
                &runtime, position, None,
                oteryn_protocol_oteryn::quick_loot::COMMAND_TYPE_QUICK_LOOT,
                request,
            ).and_then(|request| request.corpse),
            Some(corpse)
        );
        Ok(())
    }
}
