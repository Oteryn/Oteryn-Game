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
        if matches!(request.selection, wire::Selection::Resource(_)) {
            panel.resource_dialog.message("The server could not provide a current resource review. Refresh before trying again.");
        }
        return;
    }
    let Some(reply) =
        wire::Reply::decode(&result.payload).filter(|reply| reply.nonce == request.nonce)
    else {
        panel.preview.quote = None;
        panel.preview.feedback = Some("Invalid Forge preview".into());
        if matches!(request.selection, wire::Selection::Resource(_)) {
            panel
                .resource_dialog
                .message("The resource review could not be decoded. Nothing was submitted.");
        }
        return;
    };
    match (request.selection, reply.payload) {
        (wire::Selection::Effects(handle), wire::Payload::Effects(e))
            if e.item.handle == handle
                && panel.first_item == handle.get()
                && panel
                    .preview
                    .item(handle.get())
                    .is_some_and(|i| i.definition_ref == e.item.definition_ref) =>
        {
            panel.tier_effects.retain(|old| old.item.handle != handle);
            if panel.tier_effects.len() >= 2 {
                panel.tier_effects.remove(0);
            }
            panel.tier_effects.push(e.clone());
            panel.resource_dialog.reset();
            panel.resource_dialog.content =
                Some(crate::item_forge::resource_dialog::Content::Effects(e));
        }
        (wire::Selection::Resource(operation), wire::Payload::Resource(quote))
            if quote.operation == operation =>
        {
            panel.resource_dialog.offer(operation, Some(quote));
            panel.preview.feedback = None;
        }
        (wire::Selection::Resource(_), wire::Payload::Refused) => {
            panel.resource_dialog.message("This resource conversion is unavailable with your current resources or dust limit.");
        }
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
