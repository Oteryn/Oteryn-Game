//! Native browse-field controls and platform-independent result presentation.
use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
use oteryn_client_ui::browse_field::{BrowseFieldPanel, open, panel_text, seek};

pub(super) fn enabled(panel: &BrowseFieldPanel) -> bool {
    panel.selected && !panel.pending
}

pub(super) fn submit(
    panel: &mut BrowseFieldPanel,
    seeking: bool,
    fields: [&str; 5],
) -> Option<ServerMechanicCommand> {
    if !enabled(panel) {
        return None;
    }
    if seeking {
        let (Ok(view), Ok(index)) = (
            fields[3].trim().parse::<u8>(),
            fields[4].trim().parse::<u16>(),
        ) else {
            panel.feedback = Some("Enter a valid view id and page index".into());
            return None;
        };
        panel.view_id = view;
        panel.index = index;
        let command = seek(panel);
        if command.is_none() {
            panel.feedback = Some("Enter a view id from 0 to 15".into());
        }
        command
    } else {
        let (Ok(x), Ok(y), Ok(floor)) = (
            fields[0].trim().parse::<i32>(),
            fields[1].trim().parse::<i32>(),
            fields[2].trim().parse::<i16>(),
        ) else {
            panel.feedback = Some("Enter numeric tile coordinates and floor".into());
            return None;
        };
        panel.x = x;
        panel.y = y;
        panel.floor = floor;
        open(panel)
    }
}

pub(super) fn result_text(panel: &BrowseFieldPanel) -> String {
    let mut text = panel_text(panel);
    if panel.pending {
        text.push_str("\nWaiting for the server");
    }
    if !panel.response.is_empty() {
        match oteryn_session::container_tree::decode_container_view_result(&panel.response) {
            Ok(outcome) => text.push_str(&format!(
                "\nResult: {}",
                oteryn_client_runtime::container_tree::feedback(Some(outcome))
            )),
            Err(_) => text.push_str("\nThe server returned an invalid field result"),
        }
    }
    if let Some(view) = panel
        .views
        .views
        .iter()
        .find(|view| view.view_id == panel.view_id)
    {
        text.push_str("\nField/container rows:");
        for (row, item) in view.entries.iter().enumerate() {
            text.push_str(&format!(
                "\n{row}: Item {} × {}",
                item.item_definition_ref, item.count
            ));
        }
    }
    text.push_str("\nBackpack rows:");
    for (row, item) in panel.inventory.entries.iter().enumerate() {
        text.push_str(&format!(
            "\n{row}: Item {} × {}",
            item.item_definition_ref, item.count
        ));
    }
    text
}

// Uses displayed rows, never a user-supplied item handle. CL1's session lane owns moves.
pub(super) fn item_command(
    panel: &mut BrowseFieldPanel,
    action: usize,
    field_row: &str,
    bag_row: &str,
) -> Option<crate::combat_input::PlayCommand> {
    use crate::combat_input::PlayCommand;
    use oteryn_session::{
        ItemMoveDestination,
        container_tree::{ContainerViewIntent, encode_container_view_intent},
    };
    if !enabled(panel) {
        return None;
    }
    let view = panel
        .views
        .views
        .iter()
        .find(|view| view.view_id == panel.view_id)?;
    let command = match action {
        105 => PlayCommand::BrowseFieldMove(oteryn_session::ItemMoveIntent {
            source: view
                .entries
                .get(field_row.trim().parse::<usize>().ok()?)?
                .handle,
            destination: ItemMoveDestination::MainBackpack,
        }),
        106 => PlayCommand::BrowseFieldMove(oteryn_session::ItemMoveIntent {
            source: panel
                .inventory
                .entries
                .get(bag_row.trim().parse::<usize>().ok()?)?
                .handle,
            destination: ItemMoveDestination::Ground(view.browse?.position),
        }),
        109 | 110 => {
            let source = if action == 109 {
                view.entries.get(field_row.trim().parse::<usize>().ok()?)?
            } else {
                panel
                    .inventory
                    .entries
                    .get(bag_row.trim().parse::<usize>().ok()?)?
            };
            PlayCommand::WorldContext(oteryn_client_runtime::world_interaction::Command {
                target: oteryn_client_runtime::world_interaction::Target::Item(source.handle),
                action: oteryn_client_runtime::world_interaction::Action::Use,
                selection: None,
            })
        }
        107 | 108 => {
            let intent = if action == 108 {
                ContainerViewIntent::Up {
                    view_id: view.view_id,
                }
            } else {
                ContainerViewIntent::Open {
                    handle: view
                        .entries
                        .get(field_row.trim().parse::<usize>().ok()?)?
                        .handle,
                    replace_view: Some(view.view_id),
                }
            };
            PlayCommand::ServerMechanic(ServerMechanicCommand {
                capability: 14,
                command_type: 21,
                payload: encode_container_view_intent(&intent).ok()?,
            })
        }
        _ => return None,
    };
    panel.pending = !matches!(&command, PlayCommand::WorldContext(_));
    panel.feedback = None;
    Some(command)
}

#[cfg(windows)]
pub(in crate::play) use platform::open_window;

#[cfg(windows)]
// Win32 controls are owned and accessed exclusively by the panel thread.
#[allow(unsafe_code)]
#[path = "native_platform.rs"]
mod platform;
