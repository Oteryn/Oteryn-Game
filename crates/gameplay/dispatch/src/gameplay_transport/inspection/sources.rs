//! Public acquisition sources from the admitted generation's already-qualified loot tables.
use super::*;
use crate::domain::loot_data::LootTableEntry;
use oteryn_protocol_oteryn::inspection::catalogue::{Page, SCAN_LIMIT, SOURCES_PER_PAGE};

pub(super) fn contains(entries: &[LootTableEntry], key: &str, revision: &str) -> bool {
    entries.iter().any(|entry| {
        entry.probability_ppm != Some(0)
            && ((entry.item.family == crate::content::item_ref::ITEM_FAMILY
                && entry.item.production_key == key
                && entry.item.revision_ref == revision)
                || contains(&entry.details.children, key, revision))
    })
}

pub(super) async fn query(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    reference: u32,
    after: u32,
) -> Option<Reply> {
    let runtime = owner.runtime.lock().await;
    runtime.owner_fence().ok()?;
    let index = pinned_index(&runtime, owner.active_generation)?;
    let (key, revision) = index.definition_at(std::num::NonZeroU32::new(reference)?)?;
    let native = owner.active_generation?.native_gameplay()?;
    let policy = native.item_policy(key, revision)?;
    if policy.source_digest() != runtime.content_pin().server_artifact_digest()
        || policy.record().production_definition.family != crate::content::item_ref::ITEM_FAMILY
    {
        return None;
    }
    let template = details(policy.record(), 0, reference, 1, None, None)?;
    Some(project_sources(
        reference,
        &template.name,
        after,
        |cursor| {
            let profile = native
                .creature_profiles()
                .records
                .get(usize::try_from(cursor.checked_sub(1)?).ok()?)?;
            let creature = &profile.profile.target.key;
            Some(
                native
                    .reward_table()
                    .row(creature)
                    .ok()
                    .filter(|row| contains(&row.loot_table.entries, key, revision))
                    .and_then(|_| native.reward_table().encounter_facts(creature))
                    .map(|facts| facts.display_name.as_str())
                    .filter(|name| {
                        !name.is_empty()
                            && name.len() <= oteryn_protocol_oteryn::inspection::MAX_TEXT_BYTES
                            && !name.contains('\0')
                    }),
            )
        },
    ))
}

pub(super) fn project_sources<'a>(
    definition: u32,
    name: &str,
    after: u32,
    mut lookup: impl FnMut(u32) -> Option<Option<&'a str>>,
) -> Reply {
    let mut cursor = after;
    let mut next = None;
    let mut attributes = Vec::new();
    for _ in 0..SCAN_LIMIT {
        let Some(reference) = cursor.checked_add(1) else {
            next = None;
            break;
        };
        let Some(name) = lookup(reference) else {
            next = None;
            break;
        };
        cursor = reference;
        next = Some(cursor);
        if let Some(name) = name {
            attributes.push(("Dropped by".into(), name.to_owned()));
            if attributes.len() == SOURCES_PER_PAGE {
                break;
            }
        }
    }
    Reply {
        status: 1,
        requests: vec![],
        items: vec![InspectedItem {
            slot: 0,
            definition_ref: definition,
            count: 1,
            tier: 0,
            name: name.to_owned(),
            attributes,
        }],
        catalogue: Some(Page {
            next,
            categories: vec![],
        }),
    }
}
