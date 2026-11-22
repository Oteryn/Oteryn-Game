use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn open_npc_session(&self, session: GameSessionId) {
        // A client without chat still has a conversation state, held with the chat session.
        if !self.states.chat.knows(session) {
            self.chat_open(session).await;
        }
        if let Ok(Some(fence)) = self
            .root
            .observe_current_gameplay_fence(self.character, session, true)
            .await
            && let Ok(sex) = self
                .root
                .read_character_sex(self.character, fence.character_id)
                .await
        {
            self.states.chat.set_female_npc_speaker(
                session,
                matches!(sex, crate::domain::appearance::Sex::Female),
            );
        }
        self.states.npc_conversations.cancel_bank(session);
        self.states.chat.set_npc_service(session);
    }

    pub(in crate::gameplay_transport) async fn follow_npc_session(
        &self,
        _actor: ExactActorRef,
        session: GameSessionId,
        shown: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
    ) -> Option<npc_command::NpcUpdate> {
        let runtime = self.runtime.lock().await;
        self.states
            .npc_conversations
            .follow(&self.states.chat, &runtime, session, shown)
    }
}
