//! Test-only access to the existing private topic for lost-acknowledgement replay.
use super::*;
#[cfg(any(
    feature = "pg-tests-g1",
    feature = "pg-tests-g2",
    feature = "pg-tests-g3",
    feature = "pg-tests-g4"
))]
#[cfg(any(
    not(feature = "pg-tests-g1"),
    feature = "pg-tests-g2",
    feature = "pg-tests-g3",
    feature = "pg-tests-g4"
))]
#[cfg(any(
    not(feature = "pg-tests-g2"),
    feature = "pg-tests-g1",
    feature = "pg-tests-g3",
    feature = "pg-tests-g4"
))]
#[cfg(any(
    not(feature = "pg-tests-g3"),
    feature = "pg-tests-g1",
    feature = "pg-tests-g2",
    feature = "pg-tests-g4"
))]
use crate::gameplay_transport::vocation_choice::NpcVocationConversation;

impl NpcConversations {
    #[cfg(any(
        feature = "pg-tests-g1",
        feature = "pg-tests-g2",
        feature = "pg-tests-g3",
        feature = "pg-tests-g4"
    ))]
    #[cfg(any(
        not(feature = "pg-tests-g1"),
        feature = "pg-tests-g2",
        feature = "pg-tests-g3",
        feature = "pg-tests-g4"
    ))]
    #[cfg(any(
        not(feature = "pg-tests-g2"),
        feature = "pg-tests-g1",
        feature = "pg-tests-g3",
        feature = "pg-tests-g4"
    ))]
    #[cfg(any(
        not(feature = "pg-tests-g3"),
        feature = "pg-tests-g1",
        feature = "pg-tests-g2",
        feature = "pg-tests-g4"
    ))]
    pub(in crate::gameplay_transport) fn vocation_topic_for_test(
        &self,
        session: GameSessionId,
    ) -> Option<NpcVocationConversation> {
        self.sessions.lock().ok()?.get(&session)?.vocation.clone()
    }

    #[cfg(any(
        feature = "pg-tests-g1",
        feature = "pg-tests-g2",
        feature = "pg-tests-g3",
        feature = "pg-tests-g4"
    ))]
    #[cfg(any(
        not(feature = "pg-tests-g1"),
        feature = "pg-tests-g2",
        feature = "pg-tests-g3",
        feature = "pg-tests-g4"
    ))]
    #[cfg(any(
        not(feature = "pg-tests-g2"),
        feature = "pg-tests-g1",
        feature = "pg-tests-g3",
        feature = "pg-tests-g4"
    ))]
    #[cfg(any(
        not(feature = "pg-tests-g3"),
        feature = "pg-tests-g1",
        feature = "pg-tests-g2",
        feature = "pg-tests-g4"
    ))]
    pub(in crate::gameplay_transport) fn restore_vocation_topic_for_test(
        &self,
        session: GameSessionId,
        topic: NpcVocationConversation,
    ) -> bool {
        let Ok(mut sessions) = self.sessions.lock() else {
            return false;
        };
        let Some(state) = sessions.get_mut(&session) else {
            return false;
        };
        state.vocation = Some(topic);
        true
    }
}
