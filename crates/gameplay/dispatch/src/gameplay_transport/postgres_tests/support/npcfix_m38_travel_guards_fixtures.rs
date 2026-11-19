use crate::durability::npc_travel::NpcTravelGate;

pub(super) struct NoPremium;

impl NpcTravelGate for NoPremium {
    async fn travel_permitted(
        &self,
        _tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
        _account: [u8; 16],
    ) -> Result<bool, crate::durability::DurabilityError> {
        Ok(false)
    }
}
