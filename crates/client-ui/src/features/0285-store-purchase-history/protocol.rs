use super::*;
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};
pub fn request(panel: &mut Panel, next: bool) -> Option<ServerMechanicCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    let cursor = if next {
        let cursor = &panel.page.as_ref()?.next_page_cursor;
        if cursor.is_empty() {
            return None;
        }
        cursor.clone()
    } else {
        String::new()
    };
    let payload = encode_history_request(&StoreHistoryRequest {
        page_cursor: cursor,
    })
    .ok()?;
    panel.pending = true;
    panel.feedback = tr!("Waiting for the server").into();
    Some(ServerMechanicCommand {
        capability: 285,
        command_type: 501,
        payload,
    })
}
pub fn apply_result(panel: &mut Panel, response: &ServerMechanicResponse) {
    if !panel.pending {
        return;
    }
    panel.pending = false;
    if matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) && let Ok(page) = decode_history_page(&response.payload)
    {
        panel.page = Some(page);
        panel.feedback = tr!("History received").into();
        return;
    }
    panel.feedback = tr!("Purchase history refused or invalid reply; refresh").into();
}
