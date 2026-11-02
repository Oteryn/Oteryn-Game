use super::DailyBoostPanel;
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intent {
    pub command: u32,
    pub payload: Vec<u8>,
}

pub fn request(panel: &mut DailyBoostPanel, intent: Intent) -> Option<FeatureCommand> {
    if !panel.selected {
        panel.feedback = Some("Feature unavailable in this session");
        return None;
    }
    if panel.pending {
        panel.feedback = Some("Waiting for the server");
        return None;
    }
    if ![177].contains(&intent.command) {
        panel.feedback = Some("Unsupported feature action");
        return None;
    }
    panel.pending = true;
    panel.feedback = None;
    panel.last_command = Some(intent.command);
    Some(FeatureCommand {
        capability: 151,
        kind: intent.command,
        payload: intent.payload,
    })
}

pub fn apply_result(panel: &mut DailyBoostPanel, result: &GatedCommandResult) {
    panel.pending = false;
    if matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.result.clone_from(&result.payload);
        panel.feedback = None;
    } else {
        panel.feedback = Some("The server refused the action");
    }
}
