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
        if intent.text.eq_ignore_ascii_case("trade") {
            let greeting = self.npc_shop_greet(session, intent.npc_actor).await?;
            if greeting != oteryn_session::NpcIntentDisposition::Ok {
                lock(&self.0).dialogue.result(greeting);
                return Ok(());
            }
        }
        let result = session.npc_talk(intent).await?;
        lock(&self.0).dialogue.result(result);
        if intent.text.eq_ignore_ascii_case("trade")
            && result == oteryn_session::NpcIntentDisposition::Ok
        {
            self.npc_shop_balances(session).await?;
        }
        Ok(())
    }
}
impl Mailbox {
    pub(in crate::play) fn dialogue_events(&mut self, events: &[SessionEvent]) {
        for event in events {
            if let SessionEvent::NpcConversation(delta) = event {
                self.dialogue.apply(delta.value.clone());
                self.chat.console.sync_npc(&self.dialogue.lines);
            }
        }
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl Outbox {
    pub(crate) fn bevy_npc_dialogue_supported(&self) {
        lock(&self.0).dialogue.selected = true;
    }
}
