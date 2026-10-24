use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// QUEST-STATE-0 §7 and §5.4: load the admitted session's quest copy and request its
    /// pending obligations again, in the Character's revision slot. A failed load fails the
    /// session's quest actions closed, never the login.
    pub(in crate::gameplay_transport) async fn admit_quest_session(
        &self,
        admitted: &AdmittedSession,
    ) {
        self.refresh_quest_session(admitted.game_session_id, true)
            .await;
        self.admit_wheel_session(admitted.game_session_id).await;
    }

    /// The owner cadence: refresh every session whose backoff has passed. The map lock is
    /// never held across an await.
    pub(in crate::gameplay_transport) async fn refresh_due_quest_sessions(&self) {
        let now = std::time::Instant::now();
        let due: Vec<GameSessionId> = match self.states.quest_sessions.lock() {
            Ok(mut sessions) => sessions
                .iter_mut()
                .filter(|(_, entry)| entry.retry_at.is_some_and(|at| at <= now))
                .map(|(session, entry)| {
                    entry.retry_at = None;
                    *session
                })
                .collect(),
            Err(_) => return,
        };
        for session in due {
            self.refresh_quest_session(session, false).await;
        }
    }

    /// Refresh `session` on the owner cadence after the backoff.
    pub(in crate::gameplay_transport) fn schedule_quest_refresh(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.states.quest_sessions.lock()
            && let Some(entry) = sessions.get_mut(&session)
        {
            entry.retry_at = Some(std::time::Instant::now() + QUEST_OBLIGATION_RETRY);
        }
    }
}
