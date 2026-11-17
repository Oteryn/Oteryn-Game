use crate::task_board::{FullIntent, TaskBoardPanel, full_request, task_board};
use oteryn_client_runtime::raw_feature::FeatureCommand;

/// The separate result query is negotiated so original Task Board clients keep their codec.
pub fn request(panel: &mut TaskBoardPanel) -> Option<FeatureCommand> {
    if !panel.weekly_results_selected {
        return None;
    }
    let mut request = full_request(panel, FullIntent::Refresh)?;
    request.capability = task_board::full::CAPABILITY_WEEKLY_RESULTS_V1;
    request.kind = task_board::full::COMMAND_WEEKLY_RESULTS;
    Some(request)
}

/// Queue the explicit Elite preference; existing offers and the active task stay frozen.
pub fn set_elite(panel: &mut TaskBoardPanel, enabled: bool) -> Option<FeatureCommand> {
    full_request(panel, FullIntent::Elite(enabled))
}
