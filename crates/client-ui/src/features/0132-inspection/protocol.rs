#[path = "protocol/catalogue.rs"]
mod catalogue;
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
        if reply.status == 1
            && let Err(message) = catalogue::apply(panel, &mut reply)
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
