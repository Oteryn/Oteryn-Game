pub use oteryn_session::CommandStatus;
pub use oteryn_session::hireling;
use oteryn_session::{Session, SessionError, SessionStream};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HirelingCommand(pub hireling::Request);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HirelingResponse {
    pub status: CommandStatus,
    pub active: Option<bool>,
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    command: &HirelingCommand,
) -> Result<HirelingResponse, SessionError> {
    let result = session
        .gated_command(
            hireling::CAPABILITY_HIRELING_V1,
            hireling::COMMAND_TYPE_HIRELING,
            &command.0.encode(),
        )
        .await?;
    Ok(HirelingResponse {
        status: result.status,
        active: hireling::decode_reply(&result.payload),
    })
}
