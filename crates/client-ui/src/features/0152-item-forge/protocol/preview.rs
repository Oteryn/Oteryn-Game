//! Optional Forge metadata is consumed by the existing 0152 panel.
use crate::item_forge::ItemForgePanel;
use oteryn_client_runtime::{
    forge_preview as wire,
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};
pub fn request(panel: &mut ItemForgePanel, selection: wire::Selection) -> Option<FeatureCommand> {
    if !panel.editing() || !panel.preview.selected || panel.preview.pending.is_some() {
        return None;
    }
    panel.preview.nonce = panel.preview.nonce.checked_add(1)?;
    let request = wire::Request {
        nonce: panel.preview.nonce,
        selection,
    };
    let payload = request.encode()?;
    panel.preview.pending = Some(request);
    panel.preview.feedback = None;
    panel.preview.refresh = false;
    Some(FeatureCommand {
        capability: wire::CAPABILITY_FORGE_PREVIEW_V1,
        kind: wire::COMMAND_TYPE_FORGE_PREVIEW,
        payload,
    })
}
pub fn apply(panel: &mut ItemForgePanel, command: &FeatureCommand, result: &GatedCommandResult) {
    if command.capability != wire::CAPABILITY_FORGE_PREVIEW_V1
        || command.kind != wire::COMMAND_TYPE_FORGE_PREVIEW
    {
        return;
    }
    let Some(request) = wire::Request::decode(&command.payload) else {
        return;
    };
    if panel.preview.pending != Some(request) {
        return;
    }
    panel.preview.pending = None;
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.preview.quote = None;
        panel.preview.feedback = Some("Forge preview refused; refresh current items".into());
        return;
    }
    let Some(reply) =
        wire::Reply::decode(&result.payload).filter(|reply| reply.nonce == request.nonce)
    else {
        panel.preview.quote = None;
        panel.preview.feedback = Some("Invalid Forge preview".into());
        return;
    };
    match (request.selection, reply.payload) {
        (wire::Selection::Browse { after }, wire::Payload::Page(page)) if page.after == after => {
            for item in &page.items {
                if !panel.preview.remember(item.clone()) {
                    panel.preview.feedback = Some("Refresh the bounded item list".into());
                    return;
                }
            }
            panel.preview.page = Some(page);
            panel.preview.feedback = None;
        }
        (selection, wire::Payload::Quote(quote))
            if selection == quote.selection && panel.selection() == Some(selection) =>
        {
            panel.preview.remember(quote.first.clone());
            panel.preview.remember(quote.second.clone());
            panel.preview.quote = Some(quote);
            panel.preview.feedback = None;
        }
        (_, wire::Payload::Refused) => {
            panel.preview.quote = None;
            panel.preview.feedback =
                Some("This pair is not eligible for the selected Forge operation".into());
        }
        _ => {
            panel.preview.quote = None;
            panel.preview.feedback = Some("Preview no longer matches this selection".into());
        }
    }
}
