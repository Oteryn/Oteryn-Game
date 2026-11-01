//! Typed imbuement requests and replies over the selected session capability.
pub use oteryn_session::CommandStatus;
pub use oteryn_session::ItemHandle;
pub use oteryn_session::imbuement;
use oteryn_session::{Session, SessionError, SessionStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImbueCommand(pub imbuement::ImbuementRequest);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImbueResponse {
    pub status: CommandStatus,
    pub reply: Option<imbuement::ImbuementReply>,
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    command: &ImbueCommand,
) -> Result<ImbueResponse, SessionError> {
    let payload = imbuement::encode_imbuement_request(&command.0).map_err(|_| {
        SessionError::CapabilityNotSelected {
            capability: imbuement::CAPABILITY_IMBUE_V1,
        }
    })?;
    let result = session
        .gated_command(
            imbuement::CAPABILITY_IMBUE_V1,
            imbuement::COMMAND_TYPE_IMBUEMENT,
            &payload,
        )
        .await?;
    let reply = matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    )
    .then(|| imbuement::decode_imbuement_reply(&result.payload))
    .transpose()
    .map_err(|_| SessionError::SessionUnusable)?;
    Ok(ImbueResponse {
        status: result.status,
        reply,
    })
}
