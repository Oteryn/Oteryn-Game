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
        let Some(Request::CataloguePage(previous)) = panel.last_request else {
            return None;
        };
        if previous.search() != panel.search || previous.category() != panel.category {
            panel.feedback = "Refresh the catalogue after changing filters".into();
            return None;
        }
        panel.catalogue_page.as_ref()?.next?
    } else {
        0
    };
    let Ok(query) = catalogue::Query::new(after, &panel.search, &panel.category) else {
        panel.feedback = "Use a name of at most 48 bytes and category of at most 32 bytes".into();
        return None;
    };
    request(panel, Request::CataloguePage(query))
}
pub fn apply_result(panel: &mut Panel, response: &ServerMechanicResponse) {
    if !panel.pending {
        return;
    }
    panel.pending = false;
    if matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) && let Ok(reply) = decode_reply(&response.payload)
    {
        if reply.catalogue.is_some()
            && !matches!(panel.last_request, Some(Request::CataloguePage(_)))
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
                || reply
                    .items
                    .iter()
                    .any(|item| item.definition_ref <= query.after())
            {
                panel.feedback = "Invalid catalogue continuation".into();
                return;
            }
            panel.catalogue.clear();
            for item in &reply.items {
                panel.catalogue.insert(item.definition_ref, item.clone());
            }
            for category in &page.categories {
                if panel.catalogue_categories.len() < catalogue::MAX_CATEGORIES {
                    panel.catalogue_categories.insert(category.clone());
                }
            }
            panel.catalogue_page = Some(page.clone());
        }
        if reply.status == 1 && matches!(panel.last_request, Some(Request::Catalogue(_))) {
            panel.catalogue_page = None;
            for item in &reply.items {
                if panel.catalogue.contains_key(&item.definition_ref) || panel.catalogue.len() < 512
                {
                    panel.catalogue.insert(item.definition_ref, item.clone());
                }
            }
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
