use oteryn_session::{CommandStatus, Session, SessionError, SessionStream, player_typing};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerTypingOrigin {
    Manual,
    Composer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTypingCommand {
    pub typing: bool,
    pub origin: PlayerTypingOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTypingResponse {
    pub status: CommandStatus,
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    command: &PlayerTypingCommand,
) -> Result<PlayerTypingResponse, SessionError> {
    let result = session
        .gated_command(
            player_typing::CAPABILITY_PLAYER_TYPING_V1,
            player_typing::COMMAND_TYPE_PLAYER_TYPING,
            &player_typing::encode_intent(command.typing),
        )
        .await?;
    if !result.payload.is_empty() {
        return Err(SessionError::SessionUnusable);
    }
    Ok(PlayerTypingResponse {
        status: result.status,
    })
}
pub use oteryn_session::player_typing::{Error, decode_update};
