//! Resolve the retained NPC request before the existing session stream can reopen.
use super::*;
use crate::gameplay_transport::connection::AdmittedSession;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn prepare_grizzly_npc_resume(
        &self,
        admitted: &AdmittedSession,
    ) -> bool {
        forget_terminal(self, admitted.game_session_id);
        let pending = match self.states.npc_conversations.sessions.lock() {
            Ok(sessions) => sessions
                .get(&admitted.game_session_id)
                .and_then(|state| state.grizzly.pending.clone()),
            Err(_) => return false,
        };
        let Some(pending) = pending else {
            return true;
        };
        let QuestCause::Command(original) = pending.cause else {
            return false;
        };
        if original.game_session_id() != admitted.game_session_id
            || !self.states.command_ingress.reconcilable(original)
        {
            return false;
        }
        let Some(actor) = admitted.runtime_actor else {
            return false;
        };
        let Some(_input) = self.states.input_gate.enter().await else {
            return false;
        };
        // `resume` resolves durable receipts first. Any remaining action requires the
        // current gameplay fence, exact actor/lease, original NPC context and range.
        let Some(outcome) = resume(
            self,
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: original.command_id().get(),
                item_fence: admitted.item_fence,
            },
        )
        .await
        else {
            return false;
        };
        if outcome.unknown
            || self
                .states
                .command_ingress
                .complete_npc(original, outcome.disposition)
                .is_none()
        {
            return false;
        }
        // Until the ingress verdict is retained, cancellation must leave this original
        // request available. The following snapshot reads any committed inventory/XP.
        clear(self, admitted.game_session_id);
        true
    }
}
