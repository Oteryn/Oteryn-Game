use crate::play::PlayView;
use oteryn_session::LogoutResult;

/// The status line of a refused logout. `None` (the server refused the command before its logout
/// authority) and `BUSY` both ask the player to try again; `IN_FIGHT` names the whole seconds
/// left, rounded up.
#[must_use]
pub fn logout_refusal_text(result: Option<LogoutResult>) -> String {
    match result {
        Some(LogoutResult::InFight { retry_after_ms }) => format!(
            "You may not logout during a fight ({} s)",
            retry_after_ms.div_ceil(1_000)
        ),
        Some(LogoutResult::Busy | LogoutResult::Accepted) | None => {
            "You cannot logout now, try again".to_owned()
        }
    }
}

impl PlayView {
    /// Shows why the server refused a logout; the session keeps playing.
    pub fn apply_logout_refusal(&mut self, result: Option<LogoutResult>) {
        self.logout_line = Some(logout_refusal_text(result));
    }
}
