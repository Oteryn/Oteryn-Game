//! INVCAP-1: the own character's `Cap`, read through the session's current item fence with the
//! capacity and weights item moves check against.
use super::item_ref_admission::NativeItemWeights;
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::inventory_capacity::{
    CAPABILITY_INVENTORY_CAPACITY_V1, InventoryCapacity,
};

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The destination item fence identifies the House instance. Supply its origin
    /// Channel only for the actor that the live House currently holds.
    pub(super) async fn resident_origin(&self, admitted: AdmittedSession) -> Option<[u8; 16]> {
        let fence = admitted.item_fence?;
        if !matches!(
            fence.runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Instance { .. }
        ) {
            return None;
        }
        let state = self
            .runtime
            .lock()
            .await
            .house_actor_state(admitted.runtime_actor?, admitted.game_session_id)
            .ok()?;
        (state.scope == fence.runtime_scope
            && state.generation == fence.scope_ownership_generation
            && state.lease.character_id().as_bytes() == fence.character_id.as_bytes()
            && state.lease.generation() == fence.character_lease_generation)
            .then_some(*admitted.channel_id.as_bytes())
    }

    pub(super) async fn observe_own_capacity(
        &self,
        admitted: AdmittedSession,
    ) -> Option<InventoryCapacity> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_INVENTORY_CAPACITY_V1)
        {
            return None;
        }
        let fence = admitted.item_fence?;
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return None;
        }
        let native = self.active_generation?.native_gameplay()?;
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        let (capacity, carried) = self
            .root
            .read_inventory_capacity(
                self.character,
                self.holder,
                fence,
                self.resident_origin(admitted).await,
                &weights,
            )
            .await
            .ok()??;
        InventoryCapacity::from_carried(capacity, carried)
    }
}
