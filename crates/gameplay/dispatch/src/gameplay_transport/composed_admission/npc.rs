use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn open_npc_session(&self, session: GameSessionId) {
        // A client without chat still has a conversation state, held with the chat session.
        if !self.states.chat.knows(session) {
            self.chat_open(session).await;
        }
        self.states.npc_conversations.cancel_bank(session);
        self.states.chat.set_npc_service(session);
    }
}
