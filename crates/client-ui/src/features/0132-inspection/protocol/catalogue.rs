use super::{InspectedItem, Panel, Reply, Request};
use oteryn_client_runtime::discovery::inspection::catalogue::Query;
fn value<'a>(item: Option<&'a InspectedItem>, key: &str) -> Result<Option<&'a str>, &'static str> {
    let mut values = item
        .into_iter()
        .flat_map(|item| &item.attributes)
        .filter(|(name, _)| name == key);
    let result = values.next().map(|(_, value)| value.as_str());
    if values.next().is_some() {
        return Err("Duplicate catalogue metadata");
    }
    Ok(result)
}
pub(super) fn apply(panel: &mut Panel, reply: &mut Reply) -> Result<(), &'static str> {
    match panel.last_request {
        Some(Request::CatalogueQuery(Query::Items {
            after, category, ..
        })) => {
            let next: Option<u32> = value(reply.items.first(), "Catalogue next")?
                .map(str::parse)
                .transpose()
                .map_err(|_| "Invalid catalogue cursor")?;
            let categories: Vec<String> = value(reply.items.first(), "Catalogue categories")?
                .map(|names| names.split('|').map(str::to_owned).collect())
                .unwrap_or_else(|| panel.catalogue_categories.clone());
            if categories.len() > 64
                || categories
                    .iter()
                    .any(|name| name.is_empty() || name.len() > 128)
                || categories.windows(2).any(|w| w[0] >= w[1])
                || usize::from(category) > categories.len()
            {
                return Err("Invalid catalogue categories");
            }
            if reply.items.iter().any(|item| item.definition_ref <= after)
                || reply
                    .items
                    .windows(2)
                    .any(|w| w[0].definition_ref >= w[1].definition_ref)
                || next.is_some_and(|n| {
                    reply
                        .items
                        .last()
                        .is_none_or(|item| item.definition_ref != n)
                })
            {
                return Err("Invalid catalogue page order");
            }
            panel.catalogue_loaded = true;
            panel.catalogue.clear();
            panel.catalogue_categories = categories;
            panel.catalogue_next = next;
            panel.catalogue_drop_next = None;
            panel.catalogue_market = None;
            panel.catalogue_definition = reply
                .items
                .first()
                .map_or_else(String::new, |item| item.definition_ref.to_string());
        }
        Some(Request::CatalogueQuery(Query::Drops { reference, after })) => {
            if reply.items.len() != 1 || reply.items[0].definition_ref != reference {
                return Err("Mismatched drop sources");
            }
            let next: Option<u16> = value(reply.items.first(), "Catalogue drop next")?
                .map(str::parse)
                .transpose()
                .map_err(|_| "Invalid drop source cursor")?;
            if next.is_some_and(|n| n <= after || n > 1024) {
                return Err("Invalid drop source page order");
            }
            panel.catalogue_drop_next = next;
            panel.catalogue_definition = reference.to_string();
        }
        Some(Request::Catalogue(reference)) => {
            panel.catalogue_definition = reference.to_string();
            panel.catalogue_drop_next = None;
        }
        _ => return Ok(()),
    }
    for item in &mut reply.items {
        if !item
            .attributes
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case("category"))
            && let Some(category) = panel
                .catalogue
                .get(&item.definition_ref)
                .and_then(|cached| {
                    cached
                        .attributes
                        .iter()
                        .find(|(key, _)| key.eq_ignore_ascii_case("category"))
                })
        {
            item.attributes.push(category.clone());
        }

        item.attributes.retain(|(key, _)| {
            !matches!(
                key.as_str(),
                "Catalogue next" | "Catalogue categories" | "Catalogue drop next"
            )
        });
        if panel.catalogue.contains_key(&item.definition_ref) || panel.catalogue.len() < 512 {
            panel.catalogue.insert(item.definition_ref, item.clone());
        }
    }
    Ok(())
}
