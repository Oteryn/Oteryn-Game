//! NPC-TRADE-3: source subtypes are Item state, never transaction quantity.
use super::service::{NpcItemFacts, Record};
use crate::content::{
    ProjectV2CandidateValue, ProjectV2Declaration, ProjectV2DefinitionRef, ProjectV2ServiceOffer,
};

pub const REGIONAL_DATA: &str = include_str!("../../../../../../content/npcs/regional-trade.json");

#[derive(serde::Deserialize)]
struct RegionalTrades {
    services: std::collections::BTreeMap<String, RegionalShop>,
}

#[derive(serde::Deserialize)]
struct RegionalShop {
    offers: Vec<ProjectV2ServiceOffer>,
}

fn regional_offers(service: &str) -> impl Iterator<Item = &'static ProjectV2ServiceOffer> {
    static TRADES: std::sync::OnceLock<Option<RegionalTrades>> = std::sync::OnceLock::new();
    TRADES
        .get_or_init(|| serde_json::from_str(REGIONAL_DATA).ok())
        .as_ref()
        .and_then(|trades| trades.services.get(service))
        .into_iter()
        .flat_map(|shop| &shop.offers)
}

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

fn normalize(offer: &mut ProjectV2ServiceOffer, facts: &dyn NpcItemFacts) {
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

fn same_variant(left: &ProjectV2ServiceOffer, right: &ProjectV2ServiceOffer) -> bool {
    left.item == right.item
        && left.direction == right.direction
        && left.currency == right.currency
        && left.count == right.count
        && left.sub_type == right.sub_type
}

/// The donor enum is the same Item subtype namespace used by I10a. Only proven
/// fluid and exact full-charge wrap holds are restored; access callbacks stay held.
pub(super) fn declarations(
    records: &[Record<'_>],
    facts: &dyn NpcItemFacts,
) -> Vec<(ProjectV2DefinitionRef, ProjectV2Declaration)> {
    records
        .iter()
        .filter_map(|(reference, declaration)| {
            let ProjectV2Declaration::Service { offers, fields, .. } = declaration else {
                return None;
            };
            let mut restored: Vec<(ProjectV2ServiceOffer, bool)> = Vec::with_capacity(offers.len());
            // Explicit subtype rows own the price when an older count encoding
            // normalizes to the same offer. Keep the order of distinct variants.
            for source in offers {
                let mut offer = source.clone();
                normalize(&mut offer, facts);
                if let Some((prior, exact)) = restored
                    .iter_mut()
                    .find(|(prior, _)| same_variant(prior, &offer))
                {
                    if source.sub_type.is_some() && !*exact {
                        *prior = offer;
                        *exact = true;
                    }
                } else {
                    restored.push((offer, source.sub_type.is_some()));
                }
            }
            let mut restored: Vec<_> = restored.into_iter().map(|(offer, _)| offer).collect();
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
                        if !restored.iter().any(|row| same_variant(row, &offer)) {
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
                if !restored.iter().any(|row| same_variant(row, &offer)) {
                    restored.push(offer);
                }
            }
            for offer in regional_offers(&reference.key) {
                restored.retain(|existing| {
                    existing.item != offer.item
                        || existing.direction != offer.direction
                        || existing.currency != offer.currency
                        || existing.count.unwrap_or(1) != offer.count.unwrap_or(1)
                        || existing.sub_type != offer.sub_type
                });
                restored.push(offer.clone());
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
