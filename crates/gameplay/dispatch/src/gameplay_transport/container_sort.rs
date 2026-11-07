//! Container reorder composition: the item loop remains the only order writer.
use super::item_ref_admission::NativeItemWeights;
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::item_loop_move::container_sort::ContainerSortContent;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::container_sort::Request;

impl ContainerSortContent for NativeItemWeights<'_> {
    fn sort_name(&self, definition: &TypedDefinitionRef) -> Option<String> {
        use crate::content::ReferenceItemField::Known;
        let policy = self
            .0
            .item_policy(&definition.production_key, &definition.revision_ref)?;
        let Known(presentation) = &policy.record().semantics.presentation else {
            return None;
        };
        let Known(name) = &presentation.name else {
            return None;
        };
        Some(name.clone())
    }
}
macro_rules! authority_methods {
    () => {
        fn container_sort_replay(
            &self,
            _admitted: AdmittedSession,
            _id: u64,
            _request: oteryn_protocol_oteryn::container_sort::Request,
        ) -> impl std::future::Future<Output = Option<Option<bool>>> {
            async { Some(None) }
        }
        fn container_sort(
            &self,
            _admitted: AdmittedSession,
            _id: u64,
            _target: [u8; 16],
            _request: oteryn_protocol_oteryn::container_sort::Request,
        ) -> impl std::future::Future<Output = Option<bool>> {
            async { Some(false) }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn container_sort_replay(
            &self,
            admitted: AdmittedSession,
            id: u64,
            request: oteryn_protocol_oteryn::container_sort::Request,
        ) -> Option<Option<bool>> {
            let fence = admitted.item_fence?;
            self.root
                .read_item_loop_container_sort_receipt(
                    self.character,
                    fence.character_id,
                    crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(id).ok()?,
                    ),
                    None,
                    request,
                )
                .await
                .ok()
        }
        async fn container_sort(
            &self,
            admitted: AdmittedSession,
            id: u64,
            target: [u8; 16],
            request: oteryn_protocol_oteryn::container_sort::Request,
        ) -> Option<bool> {
            container_sort::execute(self, admitted, id, target, request).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    id: u64,
    target: [u8; 16],
    request: Request,
) -> Option<bool> {
    let Some(fence) = admitted.item_fence else {
        return Some(false);
    };
    let Some(actor) = admitted.runtime_actor else {
        return Some(false);
    };
    let command = CommandRef::new(admitted.game_session_id, CommandId::new(id).ok()?);
    if let Some(found) = owner
        .root
        .read_item_loop_container_sort_receipt(
            owner.character,
            fence.character_id,
            command,
            Some(target),
            request,
        )
        .await
        .ok()?
    {
        return Some(found);
    }
    let permit = owner.spell_lane_permit().await?;
    // Sorting a containing bag also changes the binding of any offered descendant.
    // Service exclusion rejects either participant while a physical offer is active.
    if !owner
        .states
        .player_trades
        .physical_service(admitted.game_session_id)
    {
        return Some(false);
    }
    let runtime = owner.runtime.lock().await;
    if runtime
        .player_control_facts(actor, admitted.game_session_id)
        .ok()?
        .control_loss
        .is_some()
    {
        return Some(false);
    }
    let states = owner.states.spell_states.lock().await;
    let native = super::equipment_move::native_for_move(
        &runtime,
        owner.active_generation,
        states.is_dead(actor),
        states.has_pending_spell_commit(actor, admitted.game_session_id),
    );
    drop(states);
    let Some(native) = native else {
        return Some(false);
    };
    owner
        .root
        .sort_item_loop_container(
            &permit,
            owner.character,
            owner.holder,
            &fence,
            command,
            &NativeItemWeights(native),
            target,
            request,
        )
        .await
        .ok()
}
