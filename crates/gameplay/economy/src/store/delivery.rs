use oteryn_gameplay_domain::domain::{AccountId, CharacterId, WorldId};

use super::{Catalogue, CharacterStoreView, DeliveryKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(pub [u8; 16]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryIntent {
    pub operation_id: OperationId,
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub world_id: WorldId,
    pub fulfilment_unit_id: String,
    pub offer_key: String,
    pub catalog_revision: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryEffect {
    MintStoreInbox { item_key: String, count: u32 },
    GrantUnlock { unlock_key: String },
    NoGameEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryRejection {
    UnknownCatalogueRevision,
    UnknownOffer,
    ZeroQuantity,
    QuantityOverflow,
    AlreadyOwned,
}

pub fn plan_delivery(
    catalogue: &Catalogue,
    intent: &DeliveryIntent,
    character: &CharacterStoreView,
) -> Result<Vec<DeliveryEffect>, DeliveryRejection> {
    if catalogue.revision() != intent.catalog_revision {
        return Err(DeliveryRejection::UnknownCatalogueRevision);
    }
    if intent.quantity == 0 {
        return Err(DeliveryRejection::ZeroQuantity);
    }
    let offer = catalogue
        .offer(&intent.offer_key)
        .ok_or(DeliveryRejection::UnknownOffer)?;
    let effect = match &offer.delivery {
        DeliveryKind::Item { item_key, count } => DeliveryEffect::MintStoreInbox {
            item_key: item_key.clone(),
            count: count
                .checked_mul(intent.quantity)
                .ok_or(DeliveryRejection::QuantityOverflow)?,
        },
        DeliveryKind::Unlock { unlock_key } => {
            if character.owned_unlocks.contains(unlock_key) {
                return Err(DeliveryRejection::AlreadyOwned);
            }
            DeliveryEffect::GrantUnlock {
                unlock_key: unlock_key.clone(),
            }
        }
        DeliveryKind::Service | DeliveryKind::CoinPackage => DeliveryEffect::NoGameEffect,
    };
    Ok(vec![effect])
}

#[cfg(test)]
#[path = "tests/delivery_tests.rs"]
mod tests;
