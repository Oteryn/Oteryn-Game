use super::super::connection::AdmittedSession;
use super::*;

impl ChatRuntime {
    pub(in super::super) fn peek_line(&self, session: GameSessionId) -> Option<ChatLine> {
        self.sessions
            .lock()
            .ok()?
            .get(&session)?
            .egress
            .peek()
            .cloned()
    }
    pub(in super::super) fn party_destination(
        &self,
        session: GameSessionId,
    ) -> Option<crate::chat::egress::PartyChatDestination> {
        self.sessions
            .lock()
            .ok()?
            .get(&session)?
            .egress
            .party_destination()
    }
    pub(in super::super) fn accept_party_line(
        &self,
        admitted: &AdmittedSession,
        candidate: &ChatLine,
        destination: crate::chat::egress::PartyChatDestination,
        member: bool,
    ) -> Option<ChatLine> {
        let mut sessions = self.sessions.lock().ok()?;
        let entry = sessions.get_mut(&admitted.game_session_id)?;
        if entry.egress.peek() != Some(candidate)
            || entry.egress.party_destination() != Some(destination)
        {
            return None;
        }
        let line = entry.egress.next_for_write()?;
        if member {
            Some(line)
        } else {
            entry.egress.delivered();
            None
        }
    }
    pub(in super::super) fn guild_sent_at_micros(
        &self,
        session: GameSessionId,
        candidate: &ChatLine,
    ) -> Option<u64> {
        let sessions = self.sessions.lock().ok()?;
        let entry = sessions.get(&session)?;
        if entry.egress.peek() != Some(candidate) {
            return None;
        }
        entry.egress.guild_sent_at_micros()
    }

    pub(in super::super) fn accept_guild_line(
        &self,
        admitted: &AdmittedSession,
        candidate: &ChatLine,
        sent_at_micros: Option<u64>,
        member: bool,
    ) -> Option<ChatLine> {
        let mut sessions = self.sessions.lock().ok()?;
        if !self.guild.matches(admitted) {
            return None;
        }
        let entry = sessions.get_mut(&admitted.game_session_id)?;
        if entry.egress.peek() != Some(candidate)
            || entry.egress.guild_sent_at_micros() != sent_at_micros
        {
            return None;
        }
        let line = entry.egress.next_for_write()?;
        if member {
            Some(line)
        } else {
            entry.egress.delivered();
            None
        }
    }
    pub(in super::super) fn discard_guild_line(
        &self,
        admitted: &AdmittedSession,
        candidate: &ChatLine,
        sent_at_micros: Option<u64>,
    ) {
        self.accept_guild_line(admitted, candidate, sent_at_micros, false);
    }
}
