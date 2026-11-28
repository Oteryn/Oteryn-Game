use super::{StashPanel, stash};
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};
use std::num::NonZeroU32;

pub fn row_action(panel: &mut StashPanel, intent: stash::StashRowIntent) -> Option<FeatureCommand> {
    if !panel.open
        || !panel.selected
        || !panel.actions_selected
        || panel.pending
        || !panel
            .rows
            .iter()
            .any(|row| row.handle == intent.row && row.quantity > 0)
    {
        return None;
    }
    let payload = stash::encode_stash_row_intent(intent).ok()?;
    panel.pending = true;
    panel.feedback = None;
    Some(FeatureCommand {
        capability: stash::CAPABILITY_STASH_ACTIONS_V1,
        kind: stash::COMMAND_TYPE_STASH_ROW_ACTION,
        payload,
    })
}

pub fn apply_row_action(panel: &mut StashPanel, result: &GatedCommandResult) -> Option<NonZeroU32> {
    panel.pending = false;
    let reference = (result.status == CommandStatus::Accepted)
        .then(|| stash::decode_stash_row_result(&result.payload).ok())
        .flatten();
    panel.feedback = reference
        .is_none()
        .then(|| "The stash action was refused. Select a current row and try again.".into());
    reference
}
