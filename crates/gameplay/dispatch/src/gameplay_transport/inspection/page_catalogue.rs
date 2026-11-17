//! Public templates only; the runtime lock keeps projection on one admitted content pin.
use super::*;
use oteryn_protocol_oteryn::inspection::catalogue::{MAX_CATEGORIES, Page, PageQuery, SCAN_LIMIT};
use std::collections::BTreeSet;

// Native inspection category names, including empty classes and equipment slots.
const FILTER_CATEGORIES: &[&str] = &[
    "Bed",
    "Carpet",
    "Container",
    "Depot",
    "Door",
    "Dummy",
    "Key",
    "Ladder",
    "MagicField",
    "Mailbox",
    "RewardChest",
    "Rune",
    "Teleport",
    "TrashHolder",
    "Ammunition",
    "Axe",
    "Club",
    "Distance",
    "Fist",
    "Shield",
    "Spellbook",
    "Sword",
    "Wand",
    "head",
    "body",
    "legs",
    "feet",
    "weapon hand",
    "shield hand",
    "neck",
    "finger",
    "container",
    "extra slot",
    "both hands",
    "wand/rod",
];

pub(in crate::gameplay_transport::inspection) fn category(
    record: &ItemProfileRecord,
) -> Option<String> {
    let semantics = &record.semantics;
    if let Known(classification) = &semantics.classification
        && let Known(kind) = classification.item_type
    {
        return Some(format!("{kind:?}"));
    }
    if let Known(weapon) = &semantics.weapon
        && let Known(kind) = weapon.weapon_type
    {
        return Some(format!("{kind:?}"));
    }
    None
}

pub(in crate::gameplay_transport::inspection) async fn query(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    query: PageQuery,
) -> Option<Reply> {
    let runtime = owner.runtime.lock().await;
    runtime.owner_fence().ok()?;
    let index = pinned_index(&runtime, owner.active_generation)?;
    let native = owner.active_generation?.native_gameplay()?;
    let digest = runtime.content_pin().server_artifact_digest();
    Some(project_page(query, |reference| {
        let (key, revision) = index.definition_at(std::num::NonZeroU32::new(reference)?)?;
        Some(
            native
                .item_policy(key, revision)
                .filter(|policy| {
                    policy.source_digest() == digest
                        && policy.record().production_definition.family
                            == crate::content::item_ref::ITEM_FAMILY
                })
                .and_then(|policy| details(policy.record(), 0, reference, 1, None, None)),
        )
    }))
}

pub(in crate::gameplay_transport::inspection) fn project_page(
    query: PageQuery,
    mut lookup: impl FnMut(u32) -> Option<Option<InspectedItem>>,
) -> Reply {
    let mut items = Vec::new();
    let mut categories: BTreeSet<String> = FILTER_CATEGORIES
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    let mut cursor = query.after();
    let mut next = None;
    for _ in 0..SCAN_LIMIT {
        let Some(reference) = cursor.checked_add(1) else {
            next = None;
            break;
        };
        let Some(item) = lookup(reference) else {
            next = None;
            break;
        };
        cursor = reference;
        next = Some(cursor);
        if let Some(item) = item {
            for category in oteryn_protocol_oteryn::inspection::catalogue::categories(&item) {
                if category.len()
                    <= oteryn_protocol_oteryn::inspection::catalogue::MAX_CATEGORY_BYTES
                    && categories.len() < MAX_CATEGORIES
                {
                    categories.insert(category.to_owned());
                }
            }
            if query.matches(&item) {
                items.push(item);
            }
            if items.len() == oteryn_protocol_oteryn::inspection::MAX_ITEMS {
                break;
            }
        }
    }
    Reply {
        status: 1,
        requests: Vec::new(),
        items,
        catalogue: Some(Page {
            next,
            categories: categories.into_iter().collect(),
        }),
    }
}
