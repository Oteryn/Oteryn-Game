//! Prey commands share the session's capability, sequence and generation gates.
pub use oteryn_session::{CommandStatus, prey};
use oteryn_session::{Session, SessionError, SessionStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreyResponse {
    pub request: prey::Request,
    pub status: CommandStatus,
    pub payload: Vec<u8>,
    /// A receipt may contain old state; mutations are followed by a current Query.
    pub current: Option<Vec<u8>>,
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    request: &prey::Request,
) -> Result<PreyResponse, SessionError> {
    let payload =
        prey::encode_request(request).map_err(|_| SessionError::CapabilityNotSelected {
            capability: prey::CAPABILITY_PREY_V1,
        })?;
    let result = session
        .gated_command(prey::CAPABILITY_PREY_V1, prey::COMMAND_TYPE_PREY, &payload)
        .await?;
    let current = if matches!(
        request,
        prey::Request::Query | prey::Request::OfferedToday { .. }
    ) {
        None
    } else {
        let query = session
            .gated_command(prey::CAPABILITY_PREY_V1, prey::COMMAND_TYPE_PREY, &[8, 0])
            .await?;
        matches!(
            query.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        )
        .then_some(query.payload)
    };
    Ok(PreyResponse {
        request: request.clone(),
        status: result.status,
        payload: result.payload,
        current,
    })
}
