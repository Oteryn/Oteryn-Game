//! One source POFF on fresh Wrap; resource reservation precedes the item await.
use super::*;
use crate::foundation::{
    ExactActorRef, MovementLocalPosition, RuntimeScopeRefV1, ScopeOwnershipGeneration,
};
use crate::gameplay_transport::house_item_use::side_effects::cues::{Journal, Reservation};
struct Binding(
    RuntimeScopeRefV1,
    ScopeOwnershipGeneration,
    ([u8; 32], [u8; 32], u64),
);
pub(super) struct Prepared {
    held: Reservation,
    binding: Binding,
}
impl Prepared {
    pub(super) async fn reserve(
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        actor: ExactActorRef,
        command: UseCommand,
        position: [i32; 3],
    ) -> Option<Self> {
        let map = owner.bundle_map()?;
        let runtime = owner.runtime.lock().await;
        let fence = command.item_fence?;
        let context =
            HouseActionContext::from_native_owner(&runtime, actor, command.game_session_id, fence)
                .ok()?;
        if !runtime
            .house_player_control_facts(actor, command.game_session_id)
            .is_ok_and(|f| f.control_loss.is_none())
        {
            return None;
        }
        super::super::item_ref_admission::pinned_index(&runtime, owner.active_generation)?;
        let scope = context.house().runtime_scope().ok()?;
        let binding = (
            runtime.content_pin().client_artifact_digest(),
            map.overlay.base().digest(),
            owner.reset_epoch,
        );
        let at = MovementLocalPosition {
            x: position[0],
            y: position[1],
            floor: position[2].try_into().ok()?,
        };
        let held = Journal::reserve(
            &owner.states.house_item_effects,
            scope,
            fence.scope_ownership_generation,
            actor,
            at,
            vec![3],
            binding,
        )?;
        Some(Self {
            held,
            binding: Binding(scope, fence.scope_ownership_generation, binding),
        })
    }
    pub(super) async fn publish(
        self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        actor: ExactActorRef,
        command: UseCommand,
    ) {
        let Some(map) = owner.bundle_map() else {
            return;
        };
        let runtime = owner.runtime.lock().await;
        let Some(fence) = command.item_fence else {
            return;
        };
        let Ok(context) =
            HouseActionContext::from_native_owner(&runtime, actor, command.game_session_id, fence)
        else {
            return;
        };
        let Binding(scope, generation, binding) = self.binding;
        if context.house().runtime_scope().ok() != Some(scope)
            || fence.scope_ownership_generation != generation
            || (
                runtime.content_pin().client_artifact_digest(),
                map.overlay.base().digest(),
                owner.reset_epoch,
            ) != binding
        {
            return;
        }
        self.held.commit();
    }
}
