use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn resume_session(
        &self,
        attempt: connection::ResumeAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        // OPS-SAVE-2: no session begins while the node quiesces for its shutdown save.
        let Some(_permit) = self.states.input_gate.enter().await else {
            return Err(AdmissionRefusal::Unavailable);
        };
        let admitted = self.resume_lost(attempt).await?;
        // ATTACK-0 §3 Reconnect: only the target is cleared; the in-fight deadline is kept.
        if let Some(actor) = admitted.runtime_actor {
            self.states
                .attack
                .lock()
                .await
                .clear_target(actor, admitted.game_session_id);
        }
        // PREM-1b: a reconnect pulls Premium again before any Premium read, without waiting.
        if let Some(controller) = admitted.controller {
            self.admit_premium(controller.account_id, admitted.game_session_id);
        }
        self.admit_quest_session(&admitted).await;
        Ok(admitted)
    }
}
