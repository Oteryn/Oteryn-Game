use super::ArenaPanel;
use oteryn_client_runtime::{
    arena::{self, ArenaAction},
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Catalogue,
    Action(ArenaAction),
}

pub fn request(panel: &mut ArenaPanel, intent: Intent) -> Option<FeatureCommand> {
    if !panel.selected {
        panel.feedback = Some("Arena unavailable in this session");
        return None;
    }
    if panel.pending.is_some() {
        return None;
    }
    let (kind, payload) = match intent {
        Intent::Catalogue => (arena::COMMAND_TYPE_ARENA_CATALOGUE, Vec::new()),
        Intent::Action(action) => {
            let Ok(payload) = arena::encode_match_request(action) else {
                panel.feedback = Some("Invalid reward selection");
                return None;
            };
            (arena::COMMAND_TYPE_ARENA_MATCH, payload)
        }
    };
    panel.pending = Some(kind);
    panel.feedback = None;
    Some(FeatureCommand {
        capability: arena::CAPABILITY_ARENA_V1,
        kind,
        payload,
    })
}

pub fn apply_result(panel: &mut ArenaPanel, result: &GatedCommandResult) -> bool {
    let Some(kind) = panel.pending.take() else {
        return false;
    };
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some("The server refused the arena request");
        return false;
    }
    let valid = match kind {
        arena::COMMAND_TYPE_ARENA_CATALOGUE => arena::decode_catalogue(&result.payload)
            .map(|rows| panel.challenges = rows)
            .is_ok(),
        arena::COMMAND_TYPE_ARENA_MATCH => arena::decode_match_result(&result.payload)
            .map(|reply| panel.result = Some(reply))
            .is_ok(),
        _ => false,
    };
    panel.feedback = if valid {
        None
    } else {
        Some("Invalid arena reply; refresh to retry")
    };
    valid
}
