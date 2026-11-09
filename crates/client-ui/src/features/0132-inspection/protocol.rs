#[path = "protocol/catalogue.rs"]
mod catalogue_queries;
use super::*;
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};
pub fn request(panel: &mut Panel, request: Request) -> Option<ServerMechanicCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    let payload = encode_request(request).ok()?;
    panel.last_request = Some(request);
    panel.reply = None;
    panel.pending = true;
    panel.feedback = "Waiting for the server".into();
    Some(ServerMechanicCommand {
        capability: 132,
        command_type: 153,
        payload,
    })
}
pub fn request_page(panel: &mut Panel, next: bool) -> Option<ServerMechanicCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    let after = if next {
        let previous = panel.catalogue_query?;
        if previous.search() != panel.search || previous.category() != panel.category {
            panel.feedback = "Refresh the catalogue after changing filters".into();
            return None;
        }
        panel.catalogue_page.as_ref()?.next?
    } else {
        0
    };
    let Ok(query) = catalogue::PageQuery::new(after, &panel.search, &panel.category) else {
        panel.feedback = "Use a name of at most 48 bytes and category of at most 32 bytes".into();
        return None;
    };
    request(panel, Request::CataloguePage(query))
}
pub fn request_sources(panel: &mut Panel, next: bool) -> Option<ServerMechanicCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    let Some(definition) = panel
        .catalogue_definition
        .parse::<u32>()
        .ok()
        .filter(|id| panel.catalogue.contains_key(id))
    else {
        panel.feedback = "Select a catalogue item".into();
        return None;
    };
    let after = if next {
        if panel.catalogue_source_definition != definition {
            return None;
        }
        panel.catalogue_source_next?
    } else {
        0
    };
    request(panel, Request::CatalogueSources { definition, after })
}
fn accept_sources(panel: &mut Panel, definition: u32, after: u32, reply: &Reply) -> bool {
    let Some(page) = &reply.catalogue else {
        panel.feedback = "Invalid item source reply".into();
        return false;
    };
    let [item] = reply.items.as_slice() else {
        panel.feedback = "Invalid item source reply".into();
        return false;
    };
    if item.definition_ref != definition
        || !page.categories.is_empty()
        || page.next.is_some_and(|next| next <= after)
        || item.attributes.len() > catalogue::SOURCES_PER_PAGE
        || item
            .attributes
            .iter()
            .any(|(label, _)| label != "Dropped by")
    {
        panel.feedback = "Invalid item source reply".into();
        return false;
    }
    panel.catalogue_sources = item
        .attributes
        .iter()
        .map(|(_, name)| name.clone())
        .collect();
    panel.catalogue_source_next = page.next;
    panel.catalogue_source_definition = definition;
    true
}
pub fn apply_result(panel: &mut Panel, response: &ServerMechanicResponse) {
    if !panel.pending {
        return;
    }
    panel.pending = false;
    if matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) && let Ok(mut reply) = decode_reply(&response.payload)
    {
        if reply.catalogue.is_some()
            && !matches!(
                panel.last_request,
                Some(Request::CataloguePage(_) | Request::CatalogueSources { .. })
            )
        {
            panel.feedback = "Unexpected catalogue reply".into();
            return;
        }
        if reply.status == 1
            && let Some(Request::CataloguePage(query)) = panel.last_request
        {
            let Some(page) = &reply.catalogue else {
                panel.feedback = "Catalogue browsing unavailable on this server".into();
                return;
            };
            if page.next.is_some_and(|next| next <= query.after())
                || reply.items.iter().any(|item| {
                    item.definition_ref <= query.after()
                        || page.next.is_some_and(|next| item.definition_ref > next)
                })
            {
                panel.feedback = "Invalid catalogue continuation".into();
                return;
            }
            panel.catalogue.clear();
            panel.catalogue_query_categories.clear();
            panel.catalogue_next = None;
            panel.catalogue_drop_next = None;
            panel.catalogue_market = None;
            panel.catalogue_loaded = true;
            panel.clear_sources();
            panel.catalogue_query = Some(query);
            for item in &reply.items {
                panel.catalogue.insert(item.definition_ref, item.clone());
            }
            for category in &page.categories {
                if panel.catalogue_categories.len() < catalogue::MAX_CATEGORIES {
                    panel.catalogue_categories.insert(category.clone());
                }
            }
            if !panel
                .catalogue_definition
                .parse::<u32>()
                .ok()
                .is_some_and(|id| panel.catalogue.contains_key(&id))
            {
                panel.catalogue_definition = reply
                    .items
                    .first()
                    .map_or_else(String::new, |item| item.definition_ref.to_string());
            }
            panel.catalogue_page = Some(page.clone());
        }
        if reply.status == 1
            && let Some(Request::CatalogueSources { definition, after }) = panel.last_request
            && !accept_sources(panel, definition, after, &reply)
        {
            return;
        }
        if reply.status == 1 && matches!(panel.last_request, Some(Request::Catalogue(_))) {
            panel.catalogue_page = None;
            panel.catalogue_query = None;
            panel.clear_sources();
        }
        if reply.status == 1
            && let Err(message) = catalogue_queries::apply(panel, &mut reply)
        {
            panel.feedback = message.into();
            return;
        }
        panel.feedback = match reply.status {
            1 => "Inspection received",
            2 => "Unavailable",
            3 => "Not visible",
            4 => "Too far away",
            5 => "Please wait before inspecting again",
            6 => "Permission denied",
            7 => "Permission requested",
            8 => "Permission updated",
            _ => "Unknown inspection status",
        }
        .into();
        panel.reply = Some(reply);
        return;
    }
    panel.feedback = "Inspection refused or invalid reply; try again".into();
}
