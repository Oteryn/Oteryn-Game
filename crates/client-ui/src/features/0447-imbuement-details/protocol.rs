//! Optional recipe projection consumed by the existing shrine panel.
use crate::imbue::ImbuePanel;
use oteryn_client_runtime::{
    imbuement_details as wire,
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};
pub fn request(panel: &mut ImbuePanel) -> Option<FeatureCommand> {
    if !panel.selected
        || !panel.details_selected
        || panel.pending
        || panel.details_pending.is_some()
        || panel.details_attempted
        || panel.window.is_none()
    {
        return None;
    }
    let item = panel.item?;
    panel.details_attempted = true;
    panel.details_pending = Some(item);
    panel.details_feedback = None;
    Some(FeatureCommand {
        capability: wire::CAPABILITY_IMBUEMENT_DETAILS_V1,
        kind: wire::COMMAND_TYPE_IMBUEMENT_DETAILS,
        payload: wire::encode_query(item),
    })
}
pub fn apply(panel: &mut ImbuePanel, command: &FeatureCommand, result: &GatedCommandResult) {
    if command.capability != wire::CAPABILITY_IMBUEMENT_DETAILS_V1
        || command.kind != wire::COMMAND_TYPE_IMBUEMENT_DETAILS
    {
        return;
    }
    let Ok(item) = wire::decode_query(&command.payload) else {
        return;
    };
    if panel.details_pending != Some(item) || panel.item != Some(item) {
        return;
    }
    panel.details_pending = None;
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.details_feedback = Some(tr!("The server refused material details; refresh to retry"));
        return;
    }
    let Ok(reply) = wire::decode_reply(&result.payload) else {
        panel.details_feedback = Some(tr!("Invalid material details; refresh the shrine"));
        return;
    };
    let Some(window) = &panel.window else {
        return;
    };
    if reply.item != item
        || reply.quotes.len() != window.offers.len()
        || window.offers.iter().any(|offer| {
            !reply.quotes.iter().any(|quote| {
                quote.family == offer.family
                    && quote.tier == offer.tier
                    && quote.gold == offer.gold
                    && quote.requires_premium == offer.requires_premium
            })
        })
    {
        panel.details_feedback = Some(tr!(
            "Material details no longer match this item; refresh it"
        ));
        return;
    }
    panel.details = Some(reply);
    panel.details_feedback = None;
}
