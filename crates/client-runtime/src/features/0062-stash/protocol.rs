//! Stash commands sent through the admitted session.
pub use oteryn_session::{CommandStatus, ItemHandle, stash};
use oteryn_session::{Session, SessionError, SessionStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashCommand {
    pub intent: stash::StashIntent,
    pub payload: Vec<u8>,
}

impl StashCommand {
    pub fn new(intent: stash::StashIntent) -> Option<Self> {
        Some(Self {
            payload: stash::encode_stash_intent(intent).ok()?,
            intent,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashResponse {
    pub status: CommandStatus,
    pub reply: stash::StashReply,
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    command: &StashCommand,
) -> Result<StashResponse, SessionError> {
    if StashCommand::new(command.intent).as_ref() != Some(command) {
        return Err(SessionError::CapabilityNotSelected {
            capability: stash::CAPABILITY_STASH_V1,
        });
    }
    let result = session
        .gated_command(
            stash::CAPABILITY_STASH_V1,
            stash::COMMAND_TYPE_STASH_INTENT,
            &command.payload,
        )
        .await?;
    let reply =
        stash::decode_stash_reply(&result.payload).map_err(|_| SessionError::SessionUnusable)?;
    Ok(StashResponse {
        status: result.status,
        reply,
    })
}
