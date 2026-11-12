use super::BossDifficultyPanel;
use oteryn_client_runtime::{
    CommandStatus,
    boss_difficulty::{self, Reply, Request},
    raw_feature::{FeatureCommand, GatedCommandResult},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Open,
    Preview,
    Start,
    Cancel,
}
pub fn request(panel: &mut BossDifficultyPanel, intent: Intent) -> Option<FeatureCommand> {
    if !panel.selected {
        panel.feedback = Some(tr!("Feature unavailable in this session"));
        return None;
    }
    if panel.pending {
        panel.feedback = Some(tr!("Waiting for the server"));
        return None;
    }
    let typed = match intent {
        Intent::Open => {
            // Opening another draft must never reuse a previous boss context.
            panel.context = None;
            panel.reply = None;
            Request::OpenWithProgress {
                boss_key: panel.boss_key.clone(),
            }
        }
        Intent::Preview => Request::Preview {
            context: panel.context?,
            difficulty: panel.difficulty,
        },
        Intent::Start => Request::Start {
            context: panel.context?,
            difficulty: panel.difficulty,
        },
        Intent::Cancel => Request::Cancel {
            context: panel.context?,
        },
    };
    let Ok(payload) = boss_difficulty::encode_request(&typed) else {
        panel.feedback = Some(tr!("Enter a valid boss id or open a server context first"));
        return None;
    };
    panel.pending = true;
    panel.feedback = None;
    Some(FeatureCommand {
        capability: 225,
        kind: 431,
        payload,
    })
}
pub fn apply_result(panel: &mut BossDifficultyPanel, result: &GatedCommandResult) -> bool {
    panel.pending = false;
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some(tr!("The server refused the difficulty action"));
        return false;
    }
    let Some(reply) = decode_reply(&result.payload) else {
        panel.feedback = Some(tr!("The server returned an invalid difficulty reply"));
        return false;
    };
    panel.context = match &reply {
        Reply::Window {
            context,
            boss_key,
            selected_difficulty,
            ..
        } => {
            panel.boss_key.clone_from(boss_key);
            panel.difficulty = *selected_difficulty;
            Some(*context)
        }
        Reply::Closed { .. } => None,
        Reply::Refused { .. } => None,
    };
    panel.feedback = if matches!(reply, Reply::Refused { .. }) {
        Some(tr!("The server refused the difficulty choice"))
    } else {
        None
    };
    panel.reply = Some(reply);
    true
}
fn decode_reply(bytes: &[u8]) -> Option<Reply> {
    boss_difficulty::decode_reply(bytes).ok()
}
