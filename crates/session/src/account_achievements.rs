//! Core command 10 uses the session's sequencing, replay and liveness owner.
use super::*;
pub use oteryn_protocol_oteryn::account_achievements::{
    AccountAchievementRow, AccountAchievementsResult,
};
use oteryn_protocol_oteryn::account_achievements::{
    AccountAchievementsQuery, COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY,
    decode_account_achievements_result, encode_account_achievements_query,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    Page(AccountAchievementsResult),
    Refused,
}

impl<S: SessionStream> Session<S> {
    pub async fn account_achievements(&mut self, page: u32) -> Result<Response, SessionError> {
        self.ensure_usable()?;
        let payload = encode_account_achievements_query(AccountAchievementsQuery { page });
        let outcome = self
            .send_and_read_result(COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY, &payload)
            .await;
        let result = self.poison_on_error(outcome)?;
        if !matches!(
            result.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) {
            return Ok(Response::Refused);
        }
        let decoded = decode_account_achievements_result(&result.payload)
            .map_err(|_| SessionError::Protocol(FoundationProtocolError::MalformedFrame))
            .and_then(|result| {
                if result.page == page {
                    Ok(Response::Page(result))
                } else {
                    Err(SessionError::Protocol(
                        FoundationProtocolError::MalformedFrame,
                    ))
                }
            });
        self.poison_on_error(decoded)
    }
}
