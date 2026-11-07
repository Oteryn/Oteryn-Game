//! Closed Cupcake texts share the current connection's existing private notices.
use super::*;
impl ServerMessageHub {
    pub(in crate::gameplay_transport) fn npc_source_advance(
        &self,
        session: GameSessionId,
        generation: u64,
        text: &str,
    ) {
        // Closed source EVENT_ADVANCE; this does not expose a general publisher.
        if !matches!(
            text,
            "Congratulations you received the Noxious Ripptor mount."
                | "Congratulations! You received the Hand of the Inquisition Outfit."
        ) {
            return;
        }
        self.cupcake_notice(session, generation, ServerMessage::Advance(text.into()));
    }

    pub(in crate::gameplay_transport) fn horse_rental_recipient_current(
        &self,
        session: GameSessionId,
        generation: u64,
    ) -> bool {
        self.state.lock().is_ok_and(|state| {
            state.recipients.get(&session).is_some_and(|r| {
                r.connection_generation == generation && r.queue.len() < MAX_ANNOUNCEMENTS
            })
        })
    }
    pub(in crate::gameplay_transport) fn horse_rental_expired(
        &self,
        session: GameSessionId,
        generation: u64,
    ) {
        self.cupcake_notice(
            session,
            generation,
            ServerMessage::Advance(crate::durability::horse_rental::rules::EXPIRY_TEXT.into()),
        );
    }
    pub(in crate::gameplay_transport) fn pumpkin_skinning_exhausted(
        &self,
        session: GameSessionId,
        connection_generation: u64,
    ) {
        self.cupcake_notice(
            session,
            connection_generation,
            ServerMessage::StatusText("You already used your knife on the corpse.".into()),
        );
    }
    pub(in crate::gameplay_transport) fn cupcake_status(
        &self,
        session: GameSessionId,
        connection_generation: u64,
        item: u32,
    ) {
        let notice = match item {
            28484 => ServerMessage::Advance("Your mana has been refilled.".into()),
            28485 => ServerMessage::Advance("Your health has been refilled.".into()),
            28486 => ServerMessage::StatusText("You feel more focused.".into()),
            _ => return,
        };
        self.cupcake_notice(session, connection_generation, notice);
    }
    pub(in crate::gameplay_transport) fn cupcake_exhausted(
        &self,
        session: GameSessionId,
        connection_generation: u64,
    ) {
        self.cupcake_notice(
            session,
            connection_generation,
            ServerMessage::Advance("You need to wait before using it again.".into()),
        );
    }
    fn cupcake_notice(
        &self,
        session: GameSessionId,
        connection_generation: u64,
        notice: ServerMessage,
    ) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let Some(recipient) = state.recipients.get_mut(&session) else {
            return;
        };
        if recipient.connection_generation != connection_generation
            || recipient.queue.len() >= MAX_ANNOUNCEMENTS
        {
            return;
        }
        if recipient.queue.try_reserve(1).is_err() {
            return;
        }
        recipient.queue.push_back(Arc::new(notice));
        drop(state);
        self.changed.notify_waiters();
    }
}
