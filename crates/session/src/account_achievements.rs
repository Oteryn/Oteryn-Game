//! Account achievement queries use the session's sequencing, replay and liveness owner.
use super::*;
pub use oteryn_protocol_oteryn::account_achievements::{
    ACCOUNT_ACHIEVEMENTS_PAGE_ROWS, AccountAchievementRow, AccountAchievementsResult,
    AchievementCompletion, CAPABILITY_ACHIEVEMENT_COMPLETION_FILTER_V1,
};
use oteryn_protocol_oteryn::account_achievements::{
    AccountAchievementsQuery, COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_FILTER,
    COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY, decode_account_achievements_result,
    encode_account_achievements_filter, encode_account_achievements_query,
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
        self.query_account_achievements(page, COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY, &payload)
            .await
    }

    pub async fn account_achievements_filtered(
        &mut self,
        page: u32,
        completion: AchievementCompletion,
    ) -> Result<Response, SessionError> {
        self.ensure_usable()?;
        let capability = CAPABILITY_ACHIEVEMENT_COMPLETION_FILTER_V1;
        if !self.is_selected(capability) {
            return Err(SessionError::CapabilityNotSelected { capability });
        }
        let payload = encode_account_achievements_filter(page, completion);
        self.query_account_achievements(page, COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_FILTER, &payload)
            .await
    }

    async fn query_account_achievements(
        &mut self,
        page: u32,
        command_type: u32,
        payload: &[u8],
    ) -> Result<Response, SessionError> {
        let outcome = self.send_and_read_result(command_type, payload).await;
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
