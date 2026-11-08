//! NPC-TRADE-3: source subtypes are Item state, never transaction quantity.
use super::service::Record;
use crate::content::{
    ProjectV2CandidateValue, ProjectV2Declaration, ProjectV2DefinitionRef, ProjectV2ServiceOffer,
};

pub fn fluid_container(key: &str) -> bool {
    matches!(
        key,
        "oteryn:item.tibia.i2874"
            | "oteryn:item.tibia.i2875"
            | "oteryn:item.tibia.i2877"
            | "oteryn:item.tibia.i2880"
            | "oteryn:item.tibia.i2901"
            | "oteryn:item.tibia.i5552"
            | "oteryn:item.tibia.i9232"
    )
}

/// Names in the existing Item fluid subtype namespace, for the shop LOOK result.
pub fn fluid_name(subtype: u16) -> Option<&'static str> {
    [
        "empty",
        "water",
        "wine",
        "beer",
        "mud",
        "blood",
        "slime",
        "oil",
        "urine",
        "milk",
        "mana",
        "life",
        "lemonade",
        "rum",
        "fruit juice",
        "coconut milk",
        "mead",
        "tea",
        "ink",
        "candy",
        "chocolate",
    ]
    .get(usize::from(subtype))
    .copied()
}

/// The donor enum is the same Item subtype namespace used by I10a. Only proven
/// fluid and exact full-charge wrap holds are restored; access callbacks stay held.
pub(super) fn declarations(
    records: &[Record<'_>],
    facts: &dyn super::service::NpcItemFacts,
) -> Vec<(ProjectV2DefinitionRef, ProjectV2Declaration)> {
    records
        .iter()
        .filter_map(|(reference, declaration)| {
            let ProjectV2Declaration::Service { offers, fields, .. } = declaration else {
                return None;
            };
            let mut restored = offers.clone();
            for field in fields {
                if !field
                    .field_path
                    .starts_with("oteryn:source.npc.trade.held_tuple_")
                    && !field
                        .field_path
                        .starts_with("oteryn:source.npc.held_offer_")
                {
                    continue;
                }
                let ProjectV2CandidateValue::Text(text) = &field.value else {
                    continue;
                };
                let Ok(hold) = serde_json::from_str::<serde_json::Value>(text) else {
                    continue;
                };
                if !matches!(
                    hold["reason"].as_str(),
                    Some(
                        "FLUID_SOURCE_ENUM_IS_NOT_NATIVE_TRANSACTION_QUANTITY"
                            | "FLUID_WATER_VARIANT_NATIVE_BINDING_UNQUALIFIED"
                            | "SOURCE_FLUID_SUBTYPE_NATIVE_BINDING_UNQUALIFIED"
                            | "VARIANT_SAME_TUPLE_TWO_WIKI_NOT_PROVEN"
                    )
                ) {
                    continue;
                }
                let Ok(mut offer) = serde_json::from_value::<ProjectV2ServiceOffer>(
                    hold["old_native_offer"].clone(),
                ) else {
                    continue;
                };
                if !fluid_container(&offer.item.key) {
                    let proven_wrap = matches!(
                        (offer.item.key.as_str(), offer.sub_type, offer.unit_price),
                        ("oteryn:item.tibia.i50293", Some(500), 347222)
                            | ("oteryn:item.tibia.i50294", Some(1800), 1250000)
                            | ("oteryn:item.tibia.i50295", Some(14400), 10000000)
                    );
                    if proven_wrap
                        && facts.default_charges(&offer.item.key) == offer.sub_type.map(u32::from)
                    {
                        offer.count = Some(1);
                        if !restored.contains(&offer) {
                            restored.push(offer);
                        }
                    }
                    continue;
                }
                let subtype = offer.sub_type.map(u32::from).or(offer.count).unwrap_or(0);
                // Pinned donor enum FLUID_NONE through FLUID_CHOCOLATE, not arbitrary bytes.
                if subtype > 20 {
                    continue;
                }
                offer.sub_type = u16::try_from(subtype).ok();
                offer.count = Some(1);
                if !restored.contains(&offer) {
                    restored.push(offer);
                }
            }
            for offer in &mut restored {
                if fluid_container(&offer.item.key) {
                    offer.sub_type = offer
                        .sub_type
                        .or_else(|| offer.count.and_then(|n| u16::try_from(n).ok()))
                        .or(Some(0));
                    offer.count = Some(1);
                } else if let Some(charges) = facts.default_charges(&offer.item.key)
                    && offer
                        .sub_type
                        .map(u32::from)
                        .or(offer.count)
                        .is_none_or(|source| source == charges)
                {
                    offer.sub_type = u16::try_from(charges).ok();
                    offer.count = Some(1);
                }
            }
            if &restored == offers {
                return None;
            }
            let mut declaration = (*declaration).clone();
            if let ProjectV2Declaration::Service { offers, .. } = &mut declaration {
                *offers = restored;
            }
            Some(((*reference).clone(), declaration))
        })
        .collect()
}
