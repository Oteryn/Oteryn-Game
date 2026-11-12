use super::*;
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};
pub fn request(panel: &mut Panel, listing: Option<LeaderListing>) -> Option<ServerMechanicCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    let (command_type, payload) = match listing {
        Some(value) => (390, encode_upsert(value).ok()?),
        None => (391, Vec::new()),
    };
    panel.submitted = Some(listing);
    panel.pending = true;
    panel.feedback = tr!("Waiting for the server").into();
    Some(ServerMechanicCommand {
        capability: 200,
        command_type,
        payload,
    })
}
pub fn apply_result(panel: &mut Panel, response: &ServerMechanicResponse) {
    if !panel.pending {
        return;
    }
    panel.pending = false;
    let submitted = panel.submitted.take();
    if matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) && response.payload.is_empty()
    {
        panel.confirmed = submitted.flatten();
        panel.feedback = tr!("Advertisement confirmed").into();
    } else {
        panel.feedback = tr!("Advertisement refused or invalid reply").into();
    }
}
