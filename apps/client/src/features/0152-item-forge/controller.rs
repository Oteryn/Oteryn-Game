//! Borrowed native callbacks. No temporary owning PlayLink is constructed.
use crate::combat_input::PlayCommand;
use crate::play::mailbox::Mailbox;
use oteryn_client_runtime::{cui3::item_forge::Operation, forge_preview as wire};
use tokio::sync::mpsc::UnboundedSender;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(windows),
    allow(
        dead_code,
        reason = "The Windows Forge form constructs its native actions"
    )
)]
pub(in crate::play) enum Action {
    CurrentDust,
    Effects,
    ConfirmResource(u64),
    DismissResource(u64),
    Legacy(Operation),
    Refresh,
    Next,
    Operation(Operation),
    First(u64),
    Second(u64),
    Cores { boost: bool, protect: bool },
    Quote,
    Forge,
    Retry,
    OpenBag(u64),
}
pub(in crate::play) fn occurrence() -> Option<[u8; 16]> {
    let mut id = [0; 16];
    oteryn_platform_client::native_login::secure_random(&mut id).ok()?;
    let now = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis(),
    )
    .ok()?;
    if now >= 1_u64 << 48 {
        return None;
    }
    id[..6].copy_from_slice(&now.to_be_bytes()[2..]);
    id[6] = 0x70 | (id[6] & 15);
    id[8] = 0x80 | (id[8] & 63);
    Some(id)
}
pub(in crate::play) fn activate(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    action: Action,
) {
    let panel = &mut mailbox.item_forge;
    if !panel.selected || panel.pending {
        return;
    }
    match action {
        Action::Effects => {
            let Some(handle) =
                oteryn_client_runtime::cui3::item_view::ItemHandle::new(panel.first_item)
            else {
                return;
            };
            if let Some(command) = oteryn_client_ui::item_forge::preview_request(
                panel,
                wire::Selection::Effects(handle),
            ) && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.preview.pending = None;
                panel.feedback = Some("The session has ended".into());
            }
        }
        Action::CurrentDust => {
            let command = oteryn_client_runtime::server_mechanic::ServerMechanicCommand {
                capability: 152,
                command_type: 668,
                payload: vec![],
            };
            if let Some(command) = oteryn_client_ui::item_forge::request(panel, command)
                && commands.send(PlayCommand::ServerMechanic(command)).is_err()
            {
                panel.pending = false;
                panel.feedback = Some("The session has ended".into());
            }
        }
        Action::DismissResource(serial) => {
            panel.resource_dialog.dismiss(serial);
        }
        Action::ConfirmResource(serial) => {
            let Some(id) = occurrence() else {
                panel.feedback = Some("Cannot generate an attempt identity".into());
                return;
            };
            let Some(request) = panel.confirm_resource(serial, id) else {
                return;
            };
            let Some(bytes) = request.encode() else {
                return;
            };
            if let Some(command) = oteryn_client_ui::item_forge::request(
                panel,
                oteryn_client_ui::item_forge::command(bytes.to_vec()),
            ) && commands.send(PlayCommand::ServerMechanic(command)).is_err()
            {
                panel.pending = false;
                panel.resource_dialog.message("The session ended before the result could be confirmed. Check Forge history after reconnecting.");
            }
        }
        Action::Legacy(operation) => {
            if operation.requires_pair() || panel.preview.pending.is_some() {
                return;
            }
            if wire::resource_operation(operation) {
                if !panel.editing() {
                    return;
                }
                panel.resource_dialog.reset();
                if !panel.preview.selected {
                    panel.resource_dialog.offer(operation, None);
                    return;
                }
                if let Some(command) = oteryn_client_ui::item_forge::preview_request(
                    panel,
                    wire::Selection::Resource(operation),
                ) && commands.send(PlayCommand::Feature(command)).is_err()
                {
                    panel.preview.pending = None;
                    panel.resource_dialog.message("The session has ended");
                }
                return;
            }
            let occurrence = if operation == Operation::QueryHistory {
                [0; 16]
            } else {
                let Some(id) = occurrence() else {
                    panel.feedback = Some("Cannot generate an attempt identity".into());
                    return;
                };
                id
            };
            let request = oteryn_client_runtime::cui3::item_forge::Request {
                operation,
                occurrence,
                first_item: None,
                second_item: None,
                boost: false,
                protect: false,
                history_limit: if operation == Operation::QueryHistory {
                    50
                } else {
                    0
                },
            };
            let Some(bytes) = request.encode() else {
                return;
            };
            if let Some(command) = oteryn_client_ui::item_forge::request(
                panel,
                oteryn_client_ui::item_forge::command(bytes.to_vec()),
            ) && commands.send(PlayCommand::ServerMechanic(command)).is_err()
            {
                panel.pending = false;
                panel.feedback =
                    Some("The session ended; retry the exact unresolved attempt".into());
            }
        }
        Action::Operation(operation) => {
            panel.choose_operation(operation);
        }
        Action::First(handle) => {
            panel.choose_item(true, handle);
        }
        Action::Second(handle) => {
            panel.choose_item(false, handle);
        }
        Action::Cores { boost, protect } => {
            panel.choose_cores(boost, protect);
        }
        Action::OpenBag(handle) => {
            if !panel.editing() || panel.preview.navigation.is_some() {
                return;
            }
            let valid = if panel.preview.selected {
                panel
                    .preview
                    .item(handle)
                    .is_some_and(|item| item.container)
            } else {
                panel
                    .preview
                    .live
                    .iter()
                    .any(|item| item.handle.get() == handle)
            };
            if !valid {
                return;
            }
            let Some(handle) = oteryn_client_runtime::depot::ItemHandle::new(handle) else {
                return;
            };
            if commands.send(PlayCommand::DepotContainer(handle)).is_err() {
                panel.feedback = Some("The session has ended".into());
                return;
            }
            panel.preview.navigation = Some(handle.get());
            panel.preview.quote = None;
        }
        Action::Refresh | Action::Next | Action::Quote => {
            if !panel.preview.selected {
                panel.preview.refresh = false;
                panel.preview.feedback=Some("This server does not provide Forge preview metadata; legacy submission remains available".into());
                return;
            }
            let selection = match action {
                Action::Refresh => {
                    panel.preview.invalidate();
                    panel.preview.items.clear();
                    panel.preview.page = None;
                    wire::Selection::Browse { after: 0 }
                }
                Action::Next => {
                    let Some(after) = panel.preview.page.as_ref().and_then(|page| page.next) else {
                        return;
                    };
                    wire::Selection::Browse { after }
                }
                _ => {
                    let Some(selection) = panel
                        .selection()
                        .filter(|selection| selection.canonical([0; 16]).is_some())
                    else {
                        panel.feedback = Some("Choose two different current items".into());
                        return;
                    };
                    selection
                }
            };
            if let Some(command) = oteryn_client_ui::item_forge::preview_request(panel, selection)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.preview.pending = None;
                panel.preview.feedback = Some("The session has ended".into());
            }
        }
        Action::Forge | Action::Retry => {
            let request = if action == Action::Retry {
                if panel.preview.pending.is_some() {
                    return;
                }
                panel.retry
            } else {
                let Some(id) = occurrence() else {
                    panel.feedback = Some("Cannot generate an attempt identity".into());
                    return;
                };
                panel.paid_request(id)
            };
            let Some(request) = request else {
                return;
            };
            let Some(bytes) = request.encode() else {
                return;
            };
            if let Some(command) = oteryn_client_ui::item_forge::request(
                panel,
                oteryn_client_ui::item_forge::command(bytes.to_vec()),
            ) && commands.send(PlayCommand::ServerMechanic(command)).is_err()
            {
                panel.pending = false;
                panel.feedback = Some(
                    "The session ended; this exact attempt remains available for retry".into(),
                );
            }
        }
    }
}
