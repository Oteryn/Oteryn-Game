//! Typed core-protocol account achievement adapter.
pub use oteryn_session::account_achievements::{
    AccountAchievementRow, AccountAchievementsResult, Response,
};
use oteryn_session::{Session, SessionError, SessionStream};

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    page: u32,
) -> Result<Response, SessionError> {
    session.account_achievements(page).await
}
