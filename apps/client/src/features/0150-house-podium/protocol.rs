use crate::play::feature_0115_mail::protocol::{accepted, uuid};
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
use oteryn_client_ui::house_podium::{Intent, Panel};
use oteryn_session::property_services::house_podium as wire;

pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    let item = uuid(&panel.fields[0])?;
    let (kind, payload) = match intent {
        Intent::Refresh => (wire::COMMAND_QUERY, item.to_vec()),
        Intent::Projection => (wire::COMMAND_PROJECTION, item.to_vec()),
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
