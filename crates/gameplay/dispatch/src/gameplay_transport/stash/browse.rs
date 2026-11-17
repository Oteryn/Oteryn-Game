//! Resolve view metadata exclusively from the active item pin and admitted trader book.
use crate::content::{ReferenceItemField::Known, native_gameplay::NativeGameplayState};
use crate::durability::character_stash::StashCatalogueEntry;
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::stash::StashFilter;
use std::collections::BTreeSet;

pub(in crate::gameplay_transport) fn catalogue(
    native: &NativeGameplayState,
    traders: Option<&NpcTalkBook>,
    filter: &StashFilter,
) -> Vec<StashCatalogueEntry> {
    let trader = filter.trader.trim().to_lowercase();
    let traded: BTreeSet<_> = traders
        .into_iter()
        .flat_map(|book| book.entries())
        .filter(|entry| entry.name.to_lowercase().contains(&trader))
        .flat_map(|entry| entry.offers())
        .filter(|offer| {
            offer.direction == crate::content::ProjectV2ServiceOfferDirection::BuyFromPlayer
        })
        .map(|offer| (offer.item.key.as_str(), offer.item.revision.as_str()))
        .collect();
    let search = filter.search.trim().to_lowercase();
    let category = filter.category.trim().to_lowercase();
    native
        .item_profiles()
        .map(|record| {
            let key = &record.production_definition.production_key;
            let revision = &record.production_definition.revision_ref;
            let name = match &record.semantics.presentation {
                Known(presentation) => match &presentation.name {
                    Known(name) => name.clone(),
                    _ => key.clone(),
                },
                _ => key.clone(),
            };
            let mut categories = vec!["other".to_owned()];
            if let Known(classification) = &record.semantics.classification
                && let Known(kind) = &classification.item_type
            {
                categories.push(format!("{kind:?}").to_lowercase());
            }
            if let Known(weapon) = &record.semantics.weapon
                && let Known(kind) = &weapon.weapon_type
            {
                categories.push(format!("{kind:?}").to_lowercase());
            }
            if let Known(equipment) = &record.semantics.equipment
                && let Known(patterns) = &equipment.patterns
            {
                for pattern in patterns {
                    if let Known(slot) = &pattern.primary_slot {
                        categories.push(format!("{slot:?}").to_lowercase());
                    }
                }
            }
            if matches!(
                &record.semantics.consumption,
                Known(crate::content::ReferenceItemConsumption::Potion(_))
            ) {
                categories.push("potions".into());
            }
            if matches!(
                &record.semantics.consumption,
                Known(crate::content::ReferenceItemConsumption::Food(_))
            ) {
                categories.push("food".into());
            }
            if categories.len() > 1 {
                categories.remove(0);
            }
            let matched = (name.to_lowercase().contains(&search)
                || key.to_lowercase().contains(&search))
                && (category.is_empty()
                    || categories.iter().any(|value| value.contains(&category)))
                && (trader.is_empty()
                    || traded.contains(&(
                        record.authoring.item.key.as_str(),
                        record.authoring.item.revision.as_str(),
                    ))
                    || traded.contains(&(key.as_str(), revision.as_str())));
            StashCatalogueEntry {
                key: key.clone(),
                revision: revision.clone(),
                name,
                matched,
            }
        })
        .collect()
}
