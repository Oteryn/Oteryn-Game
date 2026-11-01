//! Market command execution through the admitted session.
pub use oteryn_session::CommandStatus;
pub use oteryn_session::market;
use oteryn_session::{Session, SessionError, SessionStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketCommand(pub market::MarketRequest);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketResponse {
    pub status: CommandStatus,
    pub reply: Option<market::MarketReply>,
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    command: &MarketCommand,
) -> Result<MarketResponse, SessionError> {
    let payload = market::encode_request(command.0).map_err(|_| SessionError::SessionUnusable)?;
    let result = session
        .gated_command(
            market::CAPABILITY_MARKET_V1,
            market::COMMAND_TYPE_MARKET,
            &payload,
        )
        .await?;
    let reply = matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    )
    .then(|| market::decode_reply(&result.payload))
    .transpose()
    .map_err(|_| SessionError::SessionUnusable)?;
    Ok(MarketResponse {
        status: result.status,
        reply,
    })
}
