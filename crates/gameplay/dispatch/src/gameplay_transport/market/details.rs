//! Public catalogue facets and world-scoped settled-price statistics.
use super::*;
use crate::content::ReferenceItemField::Known;
use oteryn_protocol_oteryn::market_details::{self as wire_details, Query, Reply, Ware};
use std::collections::{BTreeMap, BTreeSet};
pub(super) async fn project(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    native: &crate::content::native_gameplay::NativeGameplayState,
    town: u16,
    query: Query,
    definition: Option<crate::durability::item_mint::TypedDefinitionRef>,
) -> Option<Vec<u8>> {
    let key = match &query {
        Query::Statistics { .. } | Query::Book { .. } => Some(definition?.production_key),
        _ => None,
    };
    let projection = owner
        .root
        .market_projection(
            owner.character,
            owner.holder,
            fence,
            key,
            town,
            match &query {
                Query::Book { page, .. } => Some(*page),
                _ => None,
            },
        )
        .await;
    let projection = match projection {
        Ok(value) => value,
        Err(_) => return wire_details::encode_reply(&Reply::Refused { code: 9 }).ok(),
    };
    let reply = match query {
        Query::Book { definition, page } => {
            let index = native.item_index()?;
            let mut rows = Vec::new();
            for offer in projection.book {
                let d = &offer.definition;
                if index
                    .definition_ref(d.family(), d.key(), d.revision())
                    .map(|reference| reference.get())
                    != Some(definition)
                {
                    continue;
                }
                rows.push(wire::MarketRow {
                    offer: *offer.id.as_bytes(),
                    definition,
                    side: if offer.side == crate::durability::market::Side::Buy {
                        1
                    } else {
                        2
                    },
                    amount: offer.remaining,
                    price: offer.piece_price,
                    state: 1,
                    anonymous: offer.anonymous,
                });
            }
            let mut statistics = projection.statistics;
            statistics.definition = definition;
            Reply::Book {
                definition,
                page,
                rows,
                statistics,
                has_next: projection.has_next,
            }
        }
        Query::Statistics { definition } => {
            let mut statistics = projection.statistics;
            statistics.definition = definition;
            Reply::Statistics(statistics)
        }
        Query::Catalogue { filter, page } => {
            if !native.market_metadata_ready() {
                return wire_details::encode_reply(&Reply::Refused { code: 9 }).ok();
            }
            let index = native.item_index()?;
            let profiles: BTreeMap<_, _> = native
                .item_profiles()
                .map(|p| {
                    (
                        (
                            p.production_definition.production_key.as_str(),
                            p.production_definition.revision_ref.as_str(),
                        ),
                        p,
                    )
                })
                .collect();
            let mut categories = BTreeSet::new();
            let mut items = Vec::new();
            let mut matched = 0u32;
            for reference in native.market_references() {
                let number = reference.get();
                let (key, revision) = index.definition_at(reference)?;
                if !native.marketable_definition(key, revision) {
                    continue;
                }
                let metadata = native.market_metadata(key, revision);
                let mut item = Ware {
                    definition: number,
                    name: format!("Item {number}"),
                    categories: vec![metadata.map_or_else(
                        || "Unclassified".into(),
                        |m| wire_details::category_label(m.category).to_owned(),
                    )],
                    vocations: None,
                    level: None,
                    forge_class: metadata.and_then(|m| m.forge_class),
                    depot_units: projection.depot.get(key).copied().unwrap_or(0),
                };
                if let Some(record) = profiles.get(&(key, revision)) {
                    if let Known(p) = &record.semantics.presentation
                        && let Known(n) = &p.name
                    {
                        item.name = n.clone();
                    }
                    if let Known(equipment) = &record.semantics.equipment
                        && let Known(patterns) = &equipment.patterns
                    {
                        let mut all = Vec::new();
                        let mut known = true;
                        for pattern in patterns {
                            if let Known(vs) = &pattern.vocations {
                                all.extend(vs.iter().map(|v| *v as u8));
                            } else {
                                known = false;
                            }
                            if let Known(n) = pattern.level {
                                item.level = Some(item.level.map_or(n, |old| old.min(n)));
                            }
                        }
                        if known {
                            all.sort_unstable();
                            all.dedup();
                            item.vocations = Some(all);
                        }
                    }
                    if let Known(trade) = &record.semantics.trade_restrictions
                        && let Known(vs) = &trade.vocations
                    {
                        item.vocations = Some(vs.iter().map(|v| *v as u8).collect());
                    }
                    if let Known(requirements) = &record.semantics.use_requirements {
                        if let Known(n) = requirements.min_level {
                            item.level = Some(n);
                        }
                        if let Known(vs) = &requirements.vocations {
                            item.vocations = Some(vs.iter().map(|v| *v as u8).collect());
                        }
                    }
                    if let Some(forge) = record.authoring.forge {
                        item.forge_class = Some(forge.classification);
                    }
                }
                categories.extend(item.categories.iter().cloned());
                if filter.matches(&item) {
                    if matched >= page * 32 && items.len() < 33 {
                        items.push(item);
                    }
                    matched = matched.saturating_add(1);
                }
            }
            if categories.len() > 256 {
                return None;
            }
            let has_next = items.len() > 32;
            items.truncate(32);
            Reply::Catalogue {
                items,
                categories: categories.into_iter().collect(),
                page,
                has_next,
            }
        }
    };
    wire_details::encode_reply(&reply).ok()
}
