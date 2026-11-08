use crate::play::feature_0115_mail::protocol::{accepted, uuid};
use oteryn_client_runtime::{
    character_info_panel as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::character_info::{Intent, Panel};

fn operation(panel: &Panel, intent: Intent) -> Result<wire::CharacterInfoOperation, String> {
    use wire::CharacterInfoOperation as Op;
    let page = || {
        let offset = if panel.fields[0].trim().is_empty() {
            0
        } else {
            panel.fields[0]
                .trim()
                .parse()
                .map_err(|_| "Enter a valid page offset")?
        };
        let limit = if panel.fields[1].trim().is_empty() {
            50
        } else {
            panel.fields[1]
                .trim()
                .parse()
                .map_err(|_| "Enter a valid page size")?
        };
        wire::PageRequest::new(offset, limit).ok_or_else(|| "Page size must be 1-50".to_owned())
    };
    let title = || {
        let id = panel.fields[3]
            .trim()
            .parse::<u8>()
            .map_err(|_| "Enter a valid title ID")?;
        if id == 0 {
            Ok(None)
        } else {
            wire::CharacterTitle::from_id(id)
                .map(Some)
                .ok_or_else(|| "Unknown title ID".to_owned())
        }
    };
    Ok(match intent {
        Intent::Query => Op::QuerySelf(page()?),
        Intent::Visible => Op::QueryVisible {
            character: uuid(&panel.fields[2])?,
            page: page()?,
        },
        Intent::Look => Op::LookVisible {
            character: uuid(&panel.fields[2])?,
        },
        Intent::Unlock => Op::Unlock(title()?.ok_or("Choose a title to unlock")?),
        Intent::Select => Op::Select(title()?),
    })
}
pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_CHARACTER_INFO_V1,
        kind: wire::COMMAND_TYPE_CHARACTER_INFO,
        payload: wire::encode_operation(operation(panel, intent)?),
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
        panel.feedback = "The server refused the request".into();
        return;
    }
    // Draft fields cannot change while pending. Match the sent operation tag.
    let intent = match command.payload.first() {
        Some(0) => Intent::Query,
        Some(1) => Intent::Visible,
        Some(2) => Intent::Unlock,
        Some(3) => Intent::Select,
        Some(4) => Intent::Look,
        _ => {
            panel.feedback = "Invalid request".into();
            return;
        }
    };
    let operation = operation(panel, intent).ok();
    let text = operation.and_then(|op| wire::result_text(op, &result.payload));
    if let Some(text) = text {
        if let Some(operation) = operation {
            oteryn_client_ui::character_info::apply_title_reply(panel, operation, &result.payload);
        }
        panel.summary = text;
        panel.feedback = "Character information updated".into();
    } else {
        panel.feedback = "Invalid character information reply".into();
    }
}
