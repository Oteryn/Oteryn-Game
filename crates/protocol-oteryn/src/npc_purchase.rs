//! Capability-gated NPC bulk purchases and Obtain container settings.
use crate::charm_wire::{
    push_message_field, push_varint_field, read_bytes, read_uint32, read_varint, set_once,
};
use crate::npc_service::{
    self, NpcServiceWireError, NpcTradeIntent, NpcTradeSide, malformed, read_fields,
};
use crate::quick_loot::QuickLootCategory;

pub const CAPABILITY_NPC_PURCHASE_V1: u32 = 514;
pub const CAPABILITY_OBTAIN_CONTAINERS_V1: u32 = 515;
pub const COMMAND_TYPE_NPC_PURCHASE: u32 = 6249;
pub const COMMAND_TYPE_OBTAIN_CONTAINER: u32 = 6250;
pub const MAX_PURCHASE_BYTES: usize = 128;
pub const MAX_OBTAIN_BYTES: usize = 32;
type Result<T> = std::result::Result<T, NpcServiceWireError>;

/// The existing offer identity plus a bulk amount. The embedded offer quantity is one;
/// keeping it separate preserves the exact legacy command and its singleton bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcPurchaseIntent {
    pub offer: NpcTradeIntent,
    pub amount: u32,
    pub ignore_cap: bool,
    pub in_backpacks: bool,
}

pub fn encode_npc_purchase(value: &NpcPurchaseIntent) -> Result<Vec<u8>> {
    if value.offer.side != NpcTradeSide::Buy
        || value.offer.quantity != 1
        || !(1..=10_000).contains(&value.amount)
    {
        return malformed();
    }
    let mut out = Vec::new();
    push_message_field(
        &mut out,
        1,
        &npc_service::encode_npc_trade_intent(&value.offer)?,
    );
    push_varint_field(&mut out, 2, u64::from(value.amount));
    push_varint_field(&mut out, 3, u64::from(value.ignore_cap));
    push_varint_field(&mut out, 4, u64::from(value.in_backpacks));
    Ok(out)
}
pub fn decode_npc_purchase(input: &[u8]) -> Result<NpcPurchaseIntent> {
    npc_service::check_size(input, MAX_PURCHASE_BYTES)?;
    let (mut offer, mut amount, mut ignore, mut bags) = (None, None, None, None);
    read_fields(input, |number, wire, input, cursor| match (number, wire) {
        (1, 2) => set_once(
            &mut offer,
            npc_service::decode_npc_trade_intent(read_bytes(input, cursor)?)?,
        ),
        (2, 0) => set_once(&mut amount, read_uint32(input, cursor)?),
        (3, 0) => set_once(&mut ignore, boolean(read_varint(input, cursor)?)?),
        (4, 0) => set_once(&mut bags, boolean(read_varint(input, cursor)?)?),
        _ => malformed(),
    })?;
    let value = NpcPurchaseIntent {
        offer: offer.ok_or(NpcServiceWireError::Malformed)?,
        amount: amount.unwrap_or(0),
        ignore_cap: ignore.unwrap_or(false),
        in_backpacks: bags.unwrap_or(false),
    };
    encode_npc_purchase(&value)?;
    Ok(value)
}
fn boolean(value: u64) -> Result<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => malformed(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObtainAction {
    Set = 4,
    Clear = 5,
    Open = 6,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObtainContainerIntent {
    pub action: ObtainAction,
    /// None selects the default category; otherwise the existing quick-loot taxonomy.
    pub category: Option<QuickLootCategory>,
    pub container: Option<[u8; 16]>,
}
pub fn encode_obtain_container(value: &ObtainContainerIntent) -> Result<Vec<u8>> {
    if (value.action == ObtainAction::Set) != value.container.is_some() {
        return malformed();
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, value.action as u64);
    if let Some(category) = value.category {
        push_varint_field(&mut out, 2, category as u64);
    }
    if let Some(container) = value.container {
        push_message_field(&mut out, 3, &container);
    }
    Ok(out)
}
pub fn decode_obtain_container(input: &[u8]) -> Result<ObtainContainerIntent> {
    npc_service::check_size(input, MAX_OBTAIN_BYTES)?;
    let (mut action, mut category, mut container) = (None, None, None);
    read_fields(input, |number, wire, input, cursor| match (number, wire) {
        (1, 0) => set_once(
            &mut action,
            match read_uint32(input, cursor)? {
                4 => ObtainAction::Set,
                5 => ObtainAction::Clear,
                6 => ObtainAction::Open,
                _ => return malformed(),
            },
        ),
        (2, 0) => set_once(
            &mut category,
            match read_uint32(input, cursor)? {
                1 => QuickLootCategory::Gold,
                2 => QuickLootCategory::CreatureProduct,
                3 => QuickLootCategory::Equipment,
                4 => QuickLootCategory::Consumable,
                5 => QuickLootCategory::Other,
                _ => return malformed(),
            },
        ),
        (3, 2) => set_once(
            &mut container,
            read_bytes(input, cursor)?
                .try_into()
                .map_err(|_| NpcServiceWireError::Malformed)?,
        ),
        _ => malformed(),
    })?;
    let value = ObtainContainerIntent {
        action: action.ok_or(NpcServiceWireError::Malformed)?,
        category,
        container,
    };
    encode_obtain_container(&value)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_spatial_entities::EntityRef;
    fn offer() -> NpcTradeIntent {
        NpcTradeIntent {
            npc_actor: EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            catalogue_revision: 2,
            offer_index: 3,
            side: NpcTradeSide::Buy,
            quantity: 1,
            expected_unit_price: 4,
        }
    }
    #[test]
    fn purchase_flags_round_trip_and_legacy_rejects_extension() -> Result<()> {
        let legacy = npc_service::encode_npc_trade_intent(&offer())?;
        assert_eq!(
            legacy,
            vec![
                10, 20, 10, 16, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 16, 1, 16, 2, 24,
                3, 32, 1, 40, 1, 48, 4
            ]
        );
        for ignore_cap in [false, true] {
            for in_backpacks in [false, true] {
                let value = NpcPurchaseIntent {
                    offer: offer(),
                    amount: 250,
                    ignore_cap,
                    in_backpacks,
                };
                let bytes = encode_npc_purchase(&value)?;
                assert_eq!(decode_npc_purchase(&bytes)?, value);
                assert!(npc_service::decode_npc_trade_intent(&bytes).is_err());
            }
        }
        assert_eq!(npc_service::encode_npc_trade_intent(&offer())?, legacy);
        assert_eq!(npc_service::decode_npc_trade_intent(&legacy)?, offer());
        Ok(())
    }
    #[test]
    fn obtain_actions_round_trip() -> Result<()> {
        for action in [ObtainAction::Set, ObtainAction::Clear, ObtainAction::Open] {
            for category in [
                None,
                Some(QuickLootCategory::Gold),
                Some(QuickLootCategory::CreatureProduct),
                Some(QuickLootCategory::Equipment),
                Some(QuickLootCategory::Consumable),
                Some(QuickLootCategory::Other),
            ] {
                let value = ObtainContainerIntent {
                    action,
                    category,
                    container: (action == ObtainAction::Set).then_some([2; 16]),
                };
                assert_eq!(
                    decode_obtain_container(&encode_obtain_container(&value)?)?,
                    value
                );
            }
        }
        assert!(
            encode_obtain_container(&ObtainContainerIntent {
                action: ObtainAction::Set,
                category: None,
                container: None
            })
            .is_err()
        );
        Ok(())
    }
    #[test]
    fn malformed_flags_and_actions_refuse() -> Result<()> {
        let mut bytes = encode_npc_purchase(&NpcPurchaseIntent {
            offer: offer(),
            amount: 1,
            ignore_cap: false,
            in_backpacks: false,
        })?;
        let end = bytes.len() - 1;
        bytes[end] = 2;
        assert!(decode_npc_purchase(&bytes).is_err());
        assert!(decode_obtain_container(&[8, 3]).is_err());
        assert!(decode_obtain_container(&[8, 5, 8, 5]).is_err());
        Ok(())
    }
}
