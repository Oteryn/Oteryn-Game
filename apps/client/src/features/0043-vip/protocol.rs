use crate::play::{Outbox, mailbox::lock};

impl Outbox {
    pub(in crate::play) fn vip_initialize(&self, selected: bool) {
        lock(&self.0).vip.selected = selected;
    }
    pub(in crate::play) fn vip_result(&self, response: &oteryn_client_runtime::vip::VipResponse) {
        let mut mailbox = lock(&self.0);
        let changed = mailbox.vip.revision != response.result.revision;
        oteryn_client_ui::vip::apply_result(&mut mailbox.vip, response);
        if response.result.code == 1
            && let Ok(page) = serde_json::from_str::<serde_json::Value>(&response.result.view)
            && let Some(entries) = page.get("entries").and_then(serde_json::Value::as_array) {
            let names = &mut mailbox.chat.console.communication.vip_names;
            if changed { names.clear(); }
            names.extend(entries.iter().filter_map(|entry| entry.get("name")?.as_str()).map(str::to_lowercase));
            mailbox.chat_private.communication = mailbox.chat.console.communication.clone();
            mailbox.chat_party_room.communication = mailbox.chat.console.communication.clone();
        }
    }
}

/// A successful management action is followed by a fresh first page.
pub(in crate::play) fn refresh_after(
    command: &oteryn_client_runtime::vip::VipCommand,
    response: &oteryn_client_runtime::vip::VipResponse,
) -> Option<oteryn_client_runtime::vip::VipCommand> {
    use oteryn_client_runtime::vip::{VipCommand, vip};
    if command.kind == vip::COMMAND_TYPE_VIP_QUERY || response.result.code != 1 {
        return None;
    }
    let operation = vip::decode_request(command.kind, &command.payload)
        .ok()?
        .operation;
    if operation == vip::Operation::FriendsQuery {
        return None;
    }
    VipCommand::new(&vip::Request {
        operation: if operation.is_account() {
            vip::Operation::FriendsQuery
        } else {
            vip::Operation::Query
        },
        number: 0,
        text: String::new(),
        icon: 0,
        notify: false,
        groups: vec![],
    })
}

/// Send one VIP action and obtain the authoritative view after successful changes.
pub(in crate::play) async fn send_vip<T: crate::play::Stepper>(
    session: &mut T,
    command: &oteryn_client_runtime::vip::VipCommand,
) -> Result<oteryn_client_runtime::vip::VipResponse, oteryn_session::SessionError> {
    let response = session.vip(command).await?;
    if let Some(refresh) = refresh_after(command, &response) {
        session.vip(&refresh).await
    } else {
        Ok(response)
    }
}
