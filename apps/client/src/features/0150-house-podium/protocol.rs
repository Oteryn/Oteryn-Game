use crate::play::feature_0115_mail::protocol::{accepted, uuid};
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
use oteryn_client_ui::house_podium::{Intent, Panel};
use oteryn_session::property_services::house_podium as wire;

pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    if panel.display_selected {
        use oteryn_session::property_services::house_podium_display::Request;
        let handle = panel.fields[0]
            .trim()
            .parse::<u64>()
            .ok()
            .filter(|h| *h != 0);
        let item = if handle.is_some() {
            [0; 16]
        } else {
            uuid(&panel.fields[0])?
        };
        let request = match intent {
            Intent::Refresh | Intent::Projection => {
                handle.map_or(Request::Query(item), Request::QueryHandle)
            }
            Intent::Set | Intent::Preview => {
                if panel.loaded_item != panel.fields[0] {
                    return Err("Refresh this podium before editing".into());
                }
                let value = wire::SetRequest {
                    item,
                    expected_revision: panel
                        .revision
                        .ok_or("Refresh this podium before editing")?,
                    display_kind: panel.fields[1]
                        .trim()
                        .parse()
                        .map_err(|_| "Choose Outfit, Mount or Creature")?,
                    direction: panel.fields[2]
                        .trim()
                        .parse()
                        .map_err(|_| "Choose direction 0 to 3")?,
                    key: panel.fields[3].trim().into(),
                };
                match (intent, handle) {
                    (Intent::Set, Some(handle)) => Request::SetHandle {
                        handle,
                        value,
                        visibility: panel.visibility,
                    },
                    (Intent::Preview, Some(handle)) => Request::PreviewHandle {
                        handle,
                        value,
                        visibility: panel.visibility,
                    },
                    (Intent::Set, None) => Request::Set {
                        value,
                        visibility: panel.visibility,
                    },
                    _ => Request::Preview {
                        value,
                        visibility: panel.visibility,
                    },
                }
            }
        };
        return oteryn_client_ui::house_podium_display::request(&request)
            .ok_or_else(|| "Invalid podium display settings".into());
    }
    let item = uuid(&panel.fields[0])?;
    let (kind, payload) = match intent {
        Intent::Refresh => (wire::COMMAND_QUERY, item.to_vec()),
        Intent::Projection => (wire::COMMAND_PROJECTION, item.to_vec()),
        Intent::Preview => return Err("The server does not support podium preview metadata".into()),
        Intent::Set => {
            if panel.loaded_item != panel.fields[0] {
                return Err("Refresh this podium before editing".into());
            }
            let request = wire::SetRequest {
                item,
                expected_revision: panel.revision.ok_or("Refresh this podium before editing")?,
                display_kind: panel.fields[1]
                    .trim()
                    .parse()
                    .map_err(|_| "Choose display kind 1, 2 or 3")?,
                direction: panel.fields[2]
                    .trim()
                    .parse()
                    .map_err(|_| "Choose direction 0, 1, 2 or 3")?,
                key: panel.fields[3].trim().into(),
            };
            let payload = request
                .encode()
                .ok_or("Enter a display content key of at most 160 bytes")?;
            wire::SetRequest::decode(&payload).ok_or("Invalid display kind or direction")?;
            (wire::COMMAND_SET, payload)
        }
    };
    Ok(FeatureCommand {
        capability: 150,
        kind,
        payload,
    })
}
pub(in crate::play) fn apply(
    panel: &mut Panel,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) {
    if !panel.pending || panel.pending_kind != command.kind {
        return;
    }
    panel.pending = false;
    if command.capability == 449 {
        apply_display(panel, result);
        return;
    }
    if !accepted(result) {
        panel.revision = None;
        panel.feedback = "The server refused the podium request".into();
        return;
    }
    if command.kind == wire::COMMAND_SET {
        match wire::SetResult::decode(&result.payload) {
            Some(wire::SetResult::Accepted { revision }) => {
                panel.revision = Some(revision);
                panel.summary.clear();
                panel.projection.clear();
                panel.feedback =
                    "Display saved. Refresh the projection to view the committed display".into();
            }
            Some(wire::SetResult::Refused(reason)) => {
                panel.revision = None;
                panel.feedback =
                    format!("Podium edit refused: {reason:?}. Refresh before retrying");
            }
            None => {
                panel.revision = None;
                panel.feedback = "Invalid podium reply".into();
            }
        }
        return;
    }
    match wire::Projection::decode(&result.payload) {
        Some(projection) => {
            panel.loaded_item = panel.fields[0].clone();
            panel.revision = Some(projection.as_ref().map_or(0, |p| p.revision));
            if let Some(p) = &projection {
                panel.fields[1] = p.display_kind.to_string();
                panel.fields[2] = p.direction.to_string();
                panel.fields[3] = p.key.clone();
            }
            panel.projection = projection.map_or_else(
                || "Empty podium".into(),
                |p| {
                    format!(
                        "Display: {} | kind {} | facing {} | revision {}",
                        p.key, p.display_kind, p.direction, p.revision
                    )
                },
            );
            panel.summary = panel.projection.clone();
            panel.feedback = "Projection confirmed by the server".into();
        }
        None => {
            panel.revision = None;
            panel.feedback = "Invalid podium projection".into();
        }
    }
}

fn apply_display(panel: &mut Panel, result: &GatedCommandResult) {
    use oteryn_session::property_services::house_podium_display::Reply;
    if !accepted(result) {
        panel.preview = None;
        panel.revision = None;
        panel.feedback = "The server refused podium display settings".into();
        return;
    }
    match Reply::decode(&result.payload) {
        Some(Reply::State(state)) => {
            if state.committed {
                panel.loaded_item = panel.fields[0].clone();
                panel.revision = Some(state.projection.as_ref().map_or(0, |p| p.revision));
                panel.visibility = state.visibility;
                if let Some(p) = &state.projection {
                    panel.fields[1] = p.display_kind.to_string();
                    panel.fields[2] = p.direction.to_string();
                    panel.fields[3] = p.key.clone();
                    panel.projection = format!(
                        "Committed {} facing {} · revision {}",
                        p.key, p.direction, p.revision
                    );
                } else {
                    panel.projection = "Empty podium".into();
                }
                panel.summary = panel.projection.clone();
            }
            panel.feedback = if state.committed {
                "Committed display loaded"
            } else {
                "Draft preview validated; Save applies it"
            }
            .into();
            panel.preview = Some(state);
        }
        Some(Reply::Refused(reason)) => {
            panel.preview = None;
            panel.revision = None;
            panel.feedback = format!("Display refused: {reason:?}. Refresh this podium");
        }
        None => {
            panel.preview = None;
            panel.revision = None;
            panel.feedback = "Invalid podium display reply".into();
        }
    }
}

pub(in crate::play) fn initialize(panel: &mut Panel, session: &impl crate::play::Stepper) {
    *panel = Panel {
        selected: session.feature_selected(150),
        display_selected: session.feature_selected(449),
        ..Panel::default()
    };
}

pub(in crate::play) fn after_house_transfer(
    panel: &mut Panel,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) {
    use oteryn_client_runtime::house_transfer as transfer;
    if command.capability != 86
        || command.kind != transfer::COMMAND_TYPE_HOUSE_TRANSFER
        || !accepted(result)
    {
        return;
    }
    let (Ok(request), Ok(reply)) = (
        transfer::decode_house_transfer_intent(&command.payload),
        transfer::decode_house_transfer_ack(&result.payload),
    ) else {
        return;
    };
    if reply.house.as_deref()
        != if request.exit {
            None
        } else {
            Some(request.house.as_str())
        }
    {
        return;
    }
    panel.reset_scope();
    panel.selected = reply.selection.contains(&150);
    panel.display_selected = panel.selected && reply.selection.contains(&449);
}
