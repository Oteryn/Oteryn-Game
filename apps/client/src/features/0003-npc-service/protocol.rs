use crate::play::{
    Outbox, Stepper,
    mailbox::{Mailbox, lock},
};
use oteryn_session::{SessionEvent, npc_service};
impl Outbox {
    pub(in crate::play) fn dialogue_initialize<T: Stepper>(&self, session: &T) {
        lock(&self.0).dialogue.selected = session.feature_selected(3);
    }
    pub(in crate::play) async fn dialogue_run<T: Stepper>(
        &self,
        session: &mut T,
        intent: &npc_service::NpcTalkIntent,
    ) -> Result<(), oteryn_session::SessionError> {
        let result = session.npc_talk(intent).await?;
        lock(&self.0).dialogue.result(result);
        Ok(())
    }
}
impl Mailbox {
    pub(in crate::play) fn dialogue_events(&mut self, events: &[SessionEvent]) {
        for event in events {
            if let SessionEvent::NpcConversation(delta) = event {
                self.dialogue.apply(delta.value.clone());
            }
        }
    }
}
