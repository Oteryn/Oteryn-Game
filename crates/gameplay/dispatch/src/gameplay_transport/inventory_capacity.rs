//! INVCAP-1: the own character's `Cap`, read through the session's current item fence with the
//! capacity and weights item moves check against.
use super::item_ref_admission::NativeItemWeights;
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::inventory_capacity::{
    CAPABILITY_INVENTORY_CAPACITY_V1, InventoryCapacity,
};

impl ComposedFreshAdmission<'_, '_, '_> {
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
            .read_inventory_capacity(self.character, self.holder, fence, &weights)
            .await
            .ok()??;
        InventoryCapacity::from_carried(capacity, carried)
    }
}
