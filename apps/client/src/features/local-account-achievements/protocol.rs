use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::account_achievements::Response;
impl Outbox {
    pub(in crate::play) fn achievements_initialize(&self) {
        lock(&self.0).achievements.selected = true;
    }
    pub(in crate::play) fn achievements_result(&self, response: &Response) {
        oteryn_client_ui::account_achievements::apply_result(
            &mut lock(&self.0).achievements,
            response,
        );
    }
}

// Keep the feature's session vocabulary beside its adapter while preserving Stepper's
// default implementation for synthetic peers that do not exercise achievements.
macro_rules! session_trait {
    ($($methods:tt)*) => {
        /// The session calls the task needs; implemented by `Session`.
        pub trait Stepper: Send {
            $($methods)*

            fn account_achievements(
                &mut self,
                _page: u32,
            ) -> impl std::future::Future<
                Output = Result<oteryn_client_runtime::account_achievements::Response, oteryn_session::SessionError>,
            > + Send {
                async { Err(oteryn_session::SessionError::SessionUnusable) }
            }
        }
    };
}
pub(in crate::play) use session_trait;

// A match expression keeps the feature-owned arm local; the caller supplies its
// remaining arms and publication function without changing queue ordering.
macro_rules! dispatch {
    ($command:expr, $session:ident, $outbox:ident, $held:ident, $publish:ident; $($arms:tt)*) => {
        match $command {
            Ok(crate::combat_input::PlayCommand::AccountAchievements(page)) => {
                match $session.account_achievements(page).await {
                    Ok(response) => {
                        $outbox.achievements_result(&response);
                        $publish(&mut $session, &$outbox, &mut $held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            $($arms)*
        }
    };
}
pub(in crate::play) use dispatch;
