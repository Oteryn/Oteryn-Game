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
use oteryn_protocol_oteryn::quick_loot::{
    CAPABILITY_QUICK_LOOT_TARGET_HANDLES_V1, QuickLootRequest, QuickLootResult, decode_target_handle,
};

macro_rules! authority_methods {
    () => {
        fn quick_loot(
            &self,
            _admitted: AdmittedSession,
            _command_id: u64,
            _request: oteryn_protocol_oteryn::quick_loot::QuickLootRequest,
            _item_view: Option<&crate::gameplay_transport::item_view::SessionItemView>,
            _target_handles: bool,
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
            target_handles: bool,
        ) -> Option<(
            crate::foundation::CommandStatus,
            oteryn_protocol_oteryn::quick_loot::QuickLootResult,
        )> {
            self.execute_quick_loot(admitted, command_id, request, item_view, target_handles)
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
    target_handles: bool,
) -> Option<QuickLootRequest> {
    if !target_handles {
        // Legacy clients can name a corpse by the UUID shown in the spatial view.
        // Preserve direct item UUIDs for Ground corpses without a runtime projection.
        if let Some(corpse) = request.corpse.and_then(|identity| runtime.bound_corpse(identity)) {
            request.corpse = Some(corpse.item.item_instance_id);
        }
        return Some(request);
    }
    if let Some(handle) = request.corpse {
        request.corpse = Some(match view?.resolve(decode_target_handle(handle)?)? {
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
            // A Ground corpse can outlive its runtime projection. The durable owner
            // checks current Ground custody, reach and loot rights for this instance.
            ItemKey::Instance(instance) => instance,
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
        item_view: Option<&SessionItemView>,
        target_handles: bool,
    ) -> Option<(CommandStatus, QuickLootResult)> {
        let rejected = || {
            Some((
                CommandStatus::Rejected,
                QuickLootResult { items: Vec::new() },
            ))
        };
        if target_handles && !admitted.continuity.selected_capabilities
            .contains(CAPABILITY_QUICK_LOOT_TARGET_HANDLES_V1)
        {
            return rejected();
        }
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
            let Some(request) = resolve_targets(&runtime, position, item_view, request, target_handles) else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay_transport::actor_spell::tests::runtime_with_player;
    use crate::gameplay_transport::item_view::{ItemViewContinuity, ItemViewError};
    use oteryn_protocol_oteryn::quick_loot::{QuickLootAction, encode_target_handle};

    #[test]
    fn quick_loot_handle_command_requires_separate_capability() -> Result<(), Box<dyn std::error::Error>> {
        use crate::gameplay_transport::capabilities::{
            BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
        };
        use oteryn_protocol_oteryn::quick_loot::{
            COMMAND_TYPE_QUICK_LOOT, COMMAND_TYPE_QUICK_LOOT_TARGET_HANDLES,
        };

        let legacy = SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[4, 6, 98])
            .ok_or("legacy Quick Loot selection")?;
        assert!(legacy.command_selected(COMMAND_TYPE_QUICK_LOOT));
        assert!(!legacy.command_selected(COMMAND_TYPE_QUICK_LOOT_TARGET_HANDLES));
        let extended = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 98, CAPABILITY_QUICK_LOOT_TARGET_HANDLES_V1],
        )
        .ok_or("Quick Loot handle selection")?;
        assert!(extended.command_selected(COMMAND_TYPE_QUICK_LOOT));
        assert!(extended.command_selected(COMMAND_TYPE_QUICK_LOOT_TARGET_HANDLES));
        Ok(())
    }

    #[test]
    fn quick_loot_v1_preserves_legacy_target_uuids() {
        use oteryn_protocol_oteryn::quick_loot::QuickLootRecipient;

        let (runtime, _, _) = runtime_with_player(1);
        let position = MovementLocalPosition { x: 1, y: 2, floor: 7 };
        let click = QuickLootRequest {
            action: QuickLootAction::Click,
            corpse: Some([7; 16]),
            category: None,
            recipient: None,
            recipient_id: None,
            fallback_to_main: false,
            filter_mode: None,
            filter_definition_ref: None,
        };
        for request in [
            click,
            QuickLootRequest {
                action: QuickLootAction::Assign,
                corpse: None,
                recipient: Some(QuickLootRecipient::Container),
                recipient_id: Some([8; 16]),
                ..click
            },
            QuickLootRequest {
                action: QuickLootAction::Assign,
                corpse: None,
                recipient: Some(QuickLootRecipient::GoldPouch),
                recipient_id: Some([9; 16]),
                ..click
            },
        ] {
            assert_eq!(resolve_targets(&runtime, position, None, request, false), Some(request));
        }
    }

    #[test]
    fn quick_loot_resolves_a_ground_handle_after_corpse_projection_eviction()
    -> Result<(), ItemViewError> {
        let (runtime, _, _) = runtime_with_player(1);
        assert!(runtime.visible_corpses().is_empty());
        let position = MovementLocalPosition { x: 1, y: 2, floor: 7 };
        let corpse = [7; 16];
        let key = ItemKey::Instance(corpse);
        let mut view = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
        let handle = view.map_view(&[key], |handles| Ok(handles.handle(&key)))?;
        let request = QuickLootRequest {
            action: QuickLootAction::Click,
            corpse: handle.map(encode_target_handle),
            category: None,
            recipient: None,
            recipient_id: None,
            fallback_to_main: false,
            filter_mode: None,
            filter_definition_ref: None,
        };

        assert_eq!(
            resolve_targets(&runtime, position, Some(&view), request, true).and_then(|r| r.corpse),
            Some(corpse),
        );
        assert_eq!(resolve_targets(&runtime, position, Some(&view), request, false), Some(request));
        view.map_view(&[], |_| Ok(()))?;
        assert_eq!(resolve_targets(&runtime, position, Some(&view), request, true), None);
        Ok(())
    }
}
