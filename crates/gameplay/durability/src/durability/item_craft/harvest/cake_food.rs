//! Existing map-harvest entry and closed Cake-food composition share one transaction owner.
use super::*;

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn harvest_map_item<C: ItemLoopContent>(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: MapHarvestRequest,
        native: &NativeGameplayState,
        content: &C,
    ) -> Result<Option<MapHarvestResult>, DurabilityError> {
        self.harvest_map_item_with_food(
            permit, recovery, node, fence, request, native, content, None,
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "closed native food successor shares the existing harvested map source"
    )]
    pub async fn harvest_cake_food<C: ItemLoopContent>(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: MapHarvestRequest,
        native: &NativeGameplayState,
        content: &C,
        food: crate::durability::seasonal_cake::food::CakeFoodContext,
    ) -> Result<Option<MapHarvestResult>, DurabilityError> {
        self.harvest_map_item_with_food(
            permit,
            recovery,
            node,
            fence,
            request,
            native,
            content,
            Some(food),
        )
        .await
    }
}
