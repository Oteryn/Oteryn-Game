//! Public templates only; the runtime lock keeps projection on one admitted content pin.
use super::*;
use oteryn_protocol_oteryn::inspection::catalogue::{MAX_CATEGORIES, Page, Query, SCAN_LIMIT};
use std::collections::BTreeSet;

pub(super) async fn query(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    query: Query,
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

pub(super) fn project_page(
    query: Query,
    mut lookup: impl FnMut(u32) -> Option<Option<InspectedItem>>,
) -> Reply {
    let mut items = Vec::new();
    let mut categories = BTreeSet::new();
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
