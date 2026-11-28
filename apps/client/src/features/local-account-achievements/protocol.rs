use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::account_achievements::{AchievementCompletion, Response};
impl Outbox {
    #[cfg(test)]
    pub(in crate::play) fn achievements_initialize(&self) {
        self.achievements_initialize_with_completion(false);
    }

    pub(in crate::play) fn achievements_initialize_with_completion(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.achievements.selected = true;
        mailbox.achievements.completion = selected.then_some(AchievementCompletion::Completed);
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

            fn account_achievements_filtered(
                &mut self,
                _page: u32,
                _completion: oteryn_client_runtime::account_achievements::AchievementCompletion,
            ) -> impl std::future::Future<
                Output = Result<oteryn_client_runtime::account_achievements::Response, oteryn_session::SessionError>,
            > + Send {
                async {
                    Err(oteryn_session::SessionError::CapabilityNotSelected {
                        capability: oteryn_client_runtime::account_achievements::CAPABILITY_ACHIEVEMENT_COMPLETION_FILTER_V1,
                    })
                }
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
            Ok(command @ (crate::combat_input::PlayCommand::AccountAchievements(page)
                | crate::combat_input::PlayCommand::AccountAchievementsFiltered(page, _))) => {
                let result = if let crate::combat_input::PlayCommand::AccountAchievementsFiltered(_, mode) = command {
                    $session.account_achievements_filtered(page, mode).await
                } else {
                    $session.account_achievements(page).await
                };
                match result {
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
