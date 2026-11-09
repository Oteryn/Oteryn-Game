use super::*;
use oteryn_client_runtime::{CommandStatus, TypingUpdate, WorldSpatialEntity};

pub fn request(panel: &mut PlayerTypingPanel, typing: bool) -> Option<PlayerTypingCommand> {
    if !panel.selected {
        panel.feedback = Some(tr!("Player typing is unavailable in this session"));
        return None;
    }
    if panel.pending {
        panel.feedback = Some(tr!("Waiting for the server"));
        return None;
    }
    panel.pending = true;
    panel.feedback = None;
    Some(PlayerTypingCommand {
        typing,
        origin: PlayerTypingOrigin::Manual,
    })
}

pub fn apply_result(
    panel: &mut PlayerTypingPanel,
    command: PlayerTypingCommand,
    response: PlayerTypingResponse,
) {
    let accepted = matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    );
    match command.origin {
        PlayerTypingOrigin::Composer => {
            if accepted {
                panel.own_typing = command.typing;
            }
        }
        PlayerTypingOrigin::Manual => {
            panel.pending = false;
            if accepted {
                panel.own_typing = command.typing;
                panel.feedback = None;
            } else {
                panel.feedback = Some(tr!("The server refused the typing state"));
            }
        }
    }
}

pub fn reduce(panel: &mut PlayerTypingPanel, update: TypingUpdate) {
    panel
        .indicators
        .retain(|entry| entry.identity != update.speaker.identity);
    if update.typing {
        panel.indicators.push(TypingIndicator {
            identity: update.speaker.identity,
            generation: update.speaker.generation,
        });
    }
}

pub fn retain_visible(panel: &mut PlayerTypingPanel, entities: &[WorldSpatialEntity]) {
    panel.indicators.retain(|indicator| {
        entities.iter().any(|entity| {
            entity.entity.identity == indicator.identity
                && entity.entity.generation == indicator.generation.get()
        })
    });
}
