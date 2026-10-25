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
    pub(in super::super) fn accept_guild_line(
        &self,
        admitted: &AdmittedSession,
        candidate: &ChatLine,
        member: bool,
    ) -> Option<ChatLine> {
        let mut sessions = self.sessions.lock().ok()?;
        if !self.guild.matches(admitted) {
            return None;
        }
        let entry = sessions.get_mut(&admitted.game_session_id)?;
        if entry.egress.peek() != Some(candidate) {
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
    ) {
        self.accept_guild_line(admitted, candidate, false);
    }
}
