use super::VocationPanel;
// The server-owned Newhaven offer and its five submission buttons.
use oteryn_client_runtime::{
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
    tutorial_vocation as wire,
};
pub fn request(panel: &mut VocationPanel, choice: Option<u8>) -> Option<FeatureCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    if choice
        .is_some_and(|id| !(1..=5).contains(&id) || !panel.observation.is_some_and(|o| o.offered))
    {
        return None;
    }
    panel.pending = true;
    panel.feedback = None;
    Some(FeatureCommand {
        capability: wire::CAPABILITY_TUTORIAL_VOCATION_V1,
        kind: wire::COMMAND_TYPE_TUTORIAL_VOCATION,
        payload: choice.into_iter().collect(),
    })
}
pub fn apply_result(panel: &mut VocationPanel, result: &GatedCommandResult) {
    if !panel.pending {
        return;
    }
    panel.pending = false;
    if matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) && let Some(observation) = wire::Observation::decode(&result.payload)
    {
        panel.observation = Some(observation);
        panel.feedback = None;
    } else {
        // An old offer cannot authorize another click after a refused submission.
        panel.observation = None;
        panel.feedback = Some("The vocation choice could not be completed. Refreshing the offer.");
    }
}
