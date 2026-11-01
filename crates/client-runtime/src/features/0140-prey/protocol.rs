//! Typed Prey commands sent through the admitted session.
pub use oteryn_session::{CommandStatus, prey};
use oteryn_session::{Session, SessionError, SessionStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreyCommand {
    pub request: prey::Request,
}

impl PreyCommand {
    #[must_use]
    pub fn new(request: prey::Request) -> Self {
        Self { request }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreyResponse {
    pub status: CommandStatus,
    pub reply: Option<prey::Reply>,
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    command: &PreyCommand,
) -> Result<PreyResponse, SessionError> {
    let payload =
        prey::encode_request(&command.request).map_err(|_| SessionError::SessionUnusable)?;
    let result = session
        .gated_command(prey::CAPABILITY_PREY_V1, prey::COMMAND_TYPE_PREY, &payload)
        .await?;
    let reply = matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    )
    .then(|| prey::decode_reply(&result.payload))
    .transpose()
    .map_err(|_| SessionError::SessionUnusable)?;
    Ok(PreyResponse {
        status: result.status,
        reply,
    })
}
