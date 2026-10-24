//! Quest log command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, QuestLogState, decode_quest_log_query,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    quest_log: &mut Option<QuestLogState>,
) -> Option<Dispatch> {
    Some({
        // QUEST-LOG-WIRE-1: a read of the session's own quest copy. Over QUESTGATE0-RL-09 (2
        // per second per GameSession, sliding window) it is REJECTED with an empty payload
        // before decoding, as is a malformed query, one naming a quest that is not listed,
        // and any query while the copy is not loaded.
        let admitted_rate = quest_log.is_some()
            && admitted
                .continuity
                .quest_log
                .queries
                .admit(tokio::time::Instant::now());
        match (
            quest_log.as_mut().filter(|_| admitted_rate),
            decode_quest_log_query(command.payload),
        ) {
            (Some(state), Ok(query)) => {
                let observation = authority
                    .observe_quest_log(actor, admitted.game_session_id, None)
                    .await;
                match state.query(&mut admitted.continuity.quest_log, &query, observation) {
                    Some(Ok(domain)) => Dispatch::QuestLog(domain),
                    Some(Err(_)) => return None,
                    None => Dispatch::Unregistered,
                }
            }
            _ => Dispatch::Unregistered,
        }
    })
}
