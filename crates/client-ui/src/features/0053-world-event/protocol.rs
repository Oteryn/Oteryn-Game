use super::WorldEventPanel;
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};
use oteryn_client_runtime::world_event::{
    CAPABILITY_WORLD_EVENT_V1, COMMAND_TYPE_WORLD_EVENT_QUERY, decode_world_event_state,
};

pub fn request(panel: &mut WorldEventPanel) -> Option<FeatureCommand> {
    if !panel.selected {
        panel.feedback = Some(tr!("World events unavailable in this session"));
        return None;
    }
    if panel.pending {
        return None;
    }
    panel.pending = true;
    panel.state = None;
    panel.feedback = None;
    Some(FeatureCommand {
        capability: CAPABILITY_WORLD_EVENT_V1,
        kind: COMMAND_TYPE_WORLD_EVENT_QUERY,
        payload: Vec::new(),
    })
}

pub fn apply_result(panel: &mut WorldEventPanel, result: &GatedCommandResult) {
    panel.pending = false;
    panel.state = None;
    panel.feedback = Some(tr!("The server refused the world-event query"));
    if matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.state = decode_world_event_state(&result.payload).ok();
        panel.feedback = panel
            .state
            .is_none()
            .then_some(tr!("The server returned an invalid world event"));
    }
}
