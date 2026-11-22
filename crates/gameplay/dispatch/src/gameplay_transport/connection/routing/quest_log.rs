//! Quest log command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, QuestLogState, decode_quest_log_query,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;
use oteryn_protocol_oteryn::quest_log::{
    COMMAND_TYPE_QUEST_MISSION_TRACK, MAX_FREE_TRACKED_MISSIONS, MAX_TRACKED_MISSIONS,
    decode_quest_mission_track,
};

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
        if command.command_type == COMMAND_TYPE_QUEST_MISSION_TRACK {
            let Some(state) = quest_log.as_mut().filter(|_| admitted_rate) else {
                return Some(Dispatch::Unregistered);
            };
            let Ok(pairs) = decode_quest_mission_track(command.payload) else {
                return Some(Dispatch::Unregistered);
            };
            let limit = if authority.quest_tracker_is_premium(admitted.game_session_id).await {
                MAX_TRACKED_MISSIONS
            } else {
                MAX_FREE_TRACKED_MISSIONS
            };
            let observation = authority.observe_quest_log(actor, admitted.game_session_id, None).await;
            return match state.track_missions(&mut admitted.continuity.quest_log, &pairs, limit, observation) {
                Some(Ok(domain)) => Some(Dispatch::QuestLog(domain)),
                Some(Err(_)) => None,
                None => Some(Dispatch::Unregistered),
            };
        }
        match (
            quest_log.as_mut().filter(|_| admitted_rate),
            decode_quest_log_query(command.payload),
        ) {
            (Some(state), Ok(query)) => {
                let observation = authority
                    .observe_quest_log(actor, admitted.game_session_id, None)
                    .await;
                if query == oteryn_protocol_oteryn::quest_log::QuestLogQuery::ListOnly {
                    return Some(match QuestLogState::list_only(observation,
                        admitted.continuity.selected_capabilities.contains(
                            oteryn_protocol_oteryn::personal_map_marks::CAPABILITY_PERSONAL_MAP_MARKS_V1)) {
                        Some(payload) => {
                            Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload)
                        }
                        None => Dispatch::Unregistered,
                    });
                }
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
