use crate::play::{Outbox, mailbox::lock};

impl Outbox {
    pub(in crate::play) fn vip_initialize(&self, selected: bool) {
        lock(&self.0).vip.selected = selected;
    }
    pub(in crate::play) fn vip_result(&self, response: &oteryn_client_runtime::vip::VipResponse) {
        let mut mailbox = lock(&self.0);
        let changed = mailbox.vip.revision != response.result.revision;
        oteryn_client_ui::vip::apply_result(&mut mailbox.vip, response);
        if mailbox.vip_membership_revision != Some(response.result.revision)
            && mailbox.vip_presence_confirmed_at.is_none()
            && response.result.code == 1
            && let Ok(page) = serde_json::from_str::<serde_json::Value>(&response.result.view)
            && let Some(entries) = page.get("entries").and_then(serde_json::Value::as_array)
        {
            let names = &mut mailbox.chat.console.communication.vip_names;
            if changed {
                names.clear();
            }
            names.extend(
                entries
                    .iter()
                    .filter_map(|entry| entry.get("name")?.as_str())
                    .map(str::to_lowercase),
            );
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
    outbox: &Outbox,
    command: &oteryn_client_runtime::vip::VipCommand,
) -> Result<oteryn_client_runtime::vip::VipResponse, oteryn_session::SessionError> {
    let response = session.vip(command).await?;
    let refresh = refresh_after(command, &response);
    let (page_command, response) = if let Some(refresh) = &refresh {
        (refresh, session.vip(refresh).await?)
    } else {
        (command, response)
    };
    sync_membership(session, outbox, page_command, &response).await?;
    Ok(response)
}

/// VIP v1 returns 25 contacts per query. Sessions with presence receive full replacements.
async fn sync_membership<T: crate::play::Stepper>(
    session: &mut T,
    outbox: &Outbox,
    command: &oteryn_client_runtime::vip::VipCommand,
    response: &oteryn_client_runtime::vip::VipResponse,
) -> Result<(), oteryn_session::SessionError> {
    use oteryn_client_runtime::vip::{VipCommand, vip};
    #[derive(serde::Deserialize)]
    struct Page {
        entries: Vec<Contact>,
        next_entry: u64,
    }
    #[derive(serde::Deserialize)]
    struct Contact {
        name: String,
    }
    {
        let mailbox = lock(&outbox.0);
        if mailbox.vip_presence.selected
            || !mailbox.chat.console.communication.allow_vips
            || mailbox.vip_membership_revision == Some(response.result.revision)
        {
            return Ok(());
        }
    }
    let Ok(mut query) = vip::decode_request(command.kind, &command.payload) else {
        return Ok(());
    };
    if query.operation != vip::Operation::Query || response.result.code != 1 {
        return Ok(());
    }
    let mut initial = (query.number == 0).then_some(&response.result);
    query.number = 0;
    let mut names = std::collections::BTreeSet::new();
    // The account/world list is bounded at 100. Never follow an unbounded or repeated cursor.
    for _ in 0..4 {
        let extra;
        let result = if let Some(first) = initial.take() {
            first
        } else {
            let Some(request) = VipCommand::new(&query) else {
                break;
            };
            extra = session.vip(&request).await?;
            &extra.result
        };
        if result.code != 1 || result.revision != response.result.revision {
            break;
        }
        let Ok(page) = serde_json::from_str::<Page>(&result.view) else {
            break;
        };
        names.extend(
            page.entries
                .into_iter()
                .map(|entry| entry.name.to_lowercase()),
        );
        if page.next_entry == 0 {
            let mut mailbox = lock(&outbox.0);
            if mailbox.vip_presence_confirmed_at.is_none() {
                mailbox.vip_membership_revision = Some(result.revision);
                mailbox.chat.console.communication.vip_names = names;
                mailbox.chat_private.communication = mailbox.chat.console.communication.clone();
                mailbox.chat_party_room.communication = mailbox.chat.console.communication.clone();
            }
            break;
        }
        if page.next_entry <= query.number {
            break;
        }
        query.number = page.next_entry;
    }
    Ok(())
}
