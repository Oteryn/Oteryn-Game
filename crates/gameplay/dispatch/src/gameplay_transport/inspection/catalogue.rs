//! Public Item templates and inverse loot sources from the admitted content pin.
use super::*;
use crate::content::{item_ref::ItemDefinitionIndex, native_gameplay::NativeGameplayState};
use oteryn_protocol_oteryn::inspection::catalogue::{self as wire, Query};
use std::collections::BTreeSet;

pub(super) async fn inspect(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    query: Query,
) -> Option<Reply> {
    let runtime = owner.runtime.lock().await;
    if !super::super::battle_instinct_status::actor_is_current(&runtime, admitted) {
        return None;
    }
    let index = pinned_index(&runtime, owner.active_generation)?;
    drop(runtime);
    project(owner.active_generation?.native_gameplay()?, index, query)
}
fn native_category(record: &ItemProfileRecord) -> &str {
    record
        .authoring
        .taxonomy
        .as_ref()
        .map_or("Other", |taxonomy| taxonomy.primary.as_str())
}
fn name(record: &ItemProfileRecord) -> Option<&str> {
    let Known(presentation) = &record.semantics.presentation else {
        return None;
    };
    let Known(name) = &presentation.name else {
        return None;
    };
    Some(name)
}
fn records<'a>(
    native: &'a NativeGameplayState,
    index: &ItemDefinitionIndex,
) -> Vec<(u32, &'a ItemProfileRecord)> {
    let mut records: Vec<_> = native
        .item_profiles()
        .filter(|r| {
            r.admission.materializable && name(r).is_some_and(|name| !name.trim().is_empty())
        })
        .filter_map(|r| {
            let definition = &r.production_definition;
            Some((
                index
                    .definition_ref(
                        &definition.family,
                        &definition.production_key,
                        &definition.revision_ref,
                    )?
                    .get(),
                r,
            ))
        })
        .collect();
    records.sort_by_key(|(reference, _)| *reference);
    records
}
fn browse_category<'a>(record: &'a ItemProfileRecord, categories: &[&str]) -> &'a str {
    let category = native_category(record);
    if categories.binary_search(&category).is_ok() { category } else { "Other" }
}
fn template(record: &ItemProfileRecord, reference: u32, category: &str) -> Option<InspectedItem> {
    Some(InspectedItem {
        slot: 0,
        definition_ref: reference,
        count: 1,
        tier: 0,
        name: name(record)?.into(),
        attributes: vec![("Category".into(), category.into())],
    })
}
fn project(
    native: &NativeGameplayState,
    index: &ItemDefinitionIndex,
    query: Query,
) -> Option<Reply> {
    let records = records(native, index);
    let mut items = Vec::new();
    match query {
        Query::Items {
            after,
            category: selected,
            ..
        } => {
            let mut categories: Vec<_> = records
                .iter()
                .map(|(_, r)| native_category(r))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            if categories.iter().any(|name| name.contains('|')) { return None; }
            if categories.len() > wire::MAX_CATEGORIES {
                // The legacy numeric filter has 64 choices. Keep every item
                // browsable by folding excess taxonomy classes into Other.
                categories.retain(|name| *name != "Other");
                categories.truncate(wire::MAX_CATEGORIES - 1);
                categories.push("Other");
                categories.sort_unstable();
            }
            let category_text = categories.join("|");
            if category_text.len() > 1024
                || usize::from(selected) > categories.len()
            {
                return None;
            }
            let selected = selected
                .checked_sub(1)
                .and_then(|i| categories.get(usize::from(i)));
            let search = wire::search(query).ok()?.to_lowercase();
            let mut matches: Vec<_> = records
                .iter()
                .filter(|(reference, record)| {
                    *reference > after
                        && selected.is_none_or(|selected| *selected == browse_category(record, &categories))
                        && name(record).is_some_and(|name| name.to_lowercase().contains(&search))
                })
                .take(oteryn_protocol_oteryn::inspection::MAX_ITEMS + 1)
                .collect();
            let more = matches.len() > oteryn_protocol_oteryn::inspection::MAX_ITEMS;
            matches.truncate(oteryn_protocol_oteryn::inspection::MAX_ITEMS);
            let next = more
                .then(|| matches.last().map(|(reference, _)| *reference))
                .flatten();
            for (reference, record) in matches {
                items.push(template(record, *reference, browse_category(record, &categories))?);
            }
            if let Some(first) = items.first_mut() {
                first
                    .attributes
                    .push(("Catalogue categories".into(), category_text));
                if let Some(next) = next {
                    first
                        .attributes
                        .push(("Catalogue next".into(), next.to_string()));
                }
            }
        }
        Query::Drops { reference, after } => {
            let (_, record) = records.iter().find(|(id, _)| *id == reference)?;
            let mut item = template(record, reference, native_category(record))?;
            let races = super::super::charm_progression::canonical_races()?;
            let mut sources = Vec::new();
            for (ordinal, race) in races.iter().enumerate().skip(usize::from(after)) {
                let Ok(row) = native.reward_table().row(race.key()) else {
                    continue;
                };
                if !drops(
                    &row.loot_table.entries,
                    &record.production_definition.production_key,
                ) {
                    continue;
                }
                let Some(policy) = native
                    .policies()
                    .policies()
                    .iter()
                    .find(|p| p.definition_key == race.key())
                else {
                    continue;
                };
                sources.push(((ordinal + 1) as u16, policy.display_name.as_str()));
                if sources.len() == 7 {
                    break;
                }
            }
            let more = sources.len() > 6;
            sources.truncate(6);
            if more && let Some((ordinal, _)) = sources.last() {
                item.attributes
                    .push(("Catalogue drop next".into(), ordinal.to_string()));
            }
            let sources = if sources.is_empty() {
                "No admitted creature drop sources".into()
            } else {
                sources
                    .iter()
                    .map(|(race, name)| format!("Race {race}: {name}"))
                    .collect::<Vec<_>>()
                    .join("\r\n")
            };
            item.attributes.push(("Drop sources".into(), sources));
            items.push(item);
        }
    }
    let reply = Reply {
        catalogue: None,
        status: 1,
        items,
        requests: Vec::new(),
    };
    oteryn_protocol_oteryn::inspection::encode_reply(&reply).ok()?;
    Some(reply)
}
fn drops(entries: &[crate::domain::loot_data::LootTableEntry], key: &str) -> bool {
    entries.iter().any(|entry| {
        entry.probability_ppm != Some(0)
            && (entry.item.production_key == key || drops(&entry.details.children, key))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn item_catalogue_projects_real_search_categories_and_inverse_loot()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                let mut id = [1; 16];
                id[6] = 0x70;
                id[8] = 0x80;
                let world =
                    crate::foundation::WorldId::decode(&id).map_err(|e| format!("{e:?}"))?;
                let (controller, _) =
                    crate::content::native_gameplay::test_support::activated_with_loot_tables(
                        world, None,
                    );
                let native = controller
                    .active()
                    .and_then(|a| a.native_gameplay())
                    .ok_or("native fixture")?;
                let index = native.item_index().ok_or("item index")?;
                let page = project(
                    native,
                    index,
                    wire::items(0, 0, "").map_err(|e| format!("{e:?}"))?,
                )
                .ok_or("item page")?;
                assert!(!page.items.is_empty());
                let first = &page.items[0];
                let categories: Vec<_> = first.attributes.iter()
                    .find(|(key, _)| key == "Catalogue categories")
                    .ok_or("categories")?.1.split('|').collect();
                assert_eq!(categories.len(), wire::MAX_CATEGORIES);
                let (overflow_ref, _) = records(native, index).into_iter()
                    .find(|(_, record)| !categories.contains(&native_category(record)))
                    .ok_or("fixture must exercise category overflow")?;
                let other = u8::try_from(categories.iter().position(|name| *name == "Other")
                    .ok_or("Other category")? + 1)?;
                let overflow = project(native, index,
                    wire::items(overflow_ref - 1, other, "").map_err(|e| format!("{e:?}"))?)
                    .ok_or("overflow category page")?;
                assert_eq!(overflow.items.first().ok_or("overflow item")?.definition_ref, overflow_ref);
                assert!(overflow.items.iter().all(|item| item.attributes.iter()
                    .any(|(key, value)| key == "Category" && value == "Other")));
                let searched = project(
                    native,
                    index,
                    wire::items(0, 0, &first.name).map_err(|e| format!("{e:?}"))?,
                )
                .ok_or("search")?;
                assert!(searched.items.iter().all(|item| {
                    item.name
                        .to_lowercase()
                        .contains(&first.name.to_lowercase())
                }));
                let gold = native
                    .item_policy_for_source_id(3031)
                    .ok_or("gold")?
                    .record();
                let reference = index
                    .definition_ref(
                        "Item",
                        &gold.production_definition.production_key,
                        &gold.production_definition.revision_ref,
                    )
                    .ok_or("gold reference")?;
                let reply = project(
                    native,
                    index,
                    Query::Drops {
                        reference: reference.get(),
                        after: 0,
                    },
                )
                .ok_or("gold drop sources")?;
                assert!(
                    reply.items[0]
                        .attributes
                        .iter()
                        .any(|(key, value)| key == "Drop sources" && value.contains("Race "))
                );
                Ok(())
            })?
            .join()
            .map_err(|_| "item catalogue test thread panicked")?
    }
}

#[path = "page_catalogue.rs"]
mod page_catalogue;
#[cfg(test)]
pub(super) use page_catalogue::project_page;
pub(super) use page_catalogue::{category, query};
