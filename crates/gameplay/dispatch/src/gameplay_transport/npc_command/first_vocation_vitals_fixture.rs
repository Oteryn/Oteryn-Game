//! Test-only access to the existing private topic for lost-acknowledgement replay.
use super::*;
use crate::gameplay_transport::vocation_choice::NpcVocationConversation;

impl NpcConversations {
    pub(in crate::gameplay_transport) fn vocation_topic_for_test(
        &self,
        session: GameSessionId,
    ) -> Option<NpcVocationConversation> {
        self.sessions.lock().ok()?.get(&session)?.vocation.clone()
    }

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
