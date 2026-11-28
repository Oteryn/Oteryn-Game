//! Quest log command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, QuestLogObservation, QuestLogState,
    decode_quest_log_query,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;
use oteryn_protocol_oteryn::quest_log::{
    COMMAND_TYPE_QUEST_MISSION_TRACK, MAX_FREE_TRACKED_MISSIONS, MAX_TRACKED_MISSIONS,
    QuestLogQuery, decode_quest_mission_track,
};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    quest_log: &mut Option<QuestLogState>,
) -> Option<Dispatch> {
    // Both tracker commands share the journal's per-GameSession query window.
    let Some(state) = quest_log.as_mut().filter(|_| {
        admitted
            .continuity
            .quest_log
            .queries
            .admit(tokio::time::Instant::now())
    }) else {
        return Some(Dispatch::Unregistered);
    };
    if command.command_type == COMMAND_TYPE_QUEST_MISSION_TRACK {
        let Ok(pairs) = decode_quest_mission_track(command.payload) else {
            return Some(Dispatch::Unregistered);
        };
        let limit = if authority
            .quest_tracker_is_premium(admitted.game_session_id)
            .await
        {
            MAX_TRACKED_MISSIONS
        } else {
            MAX_FREE_TRACKED_MISSIONS
        };
        let observation = authority
            .observe_quest_log(actor, admitted.game_session_id, None)
            .await;
        let QuestLogObservation::Changed(source) = &observation else {
            return Some(Dispatch::Unregistered);
        };
        let Some(selection) = pairs
            .iter()
            .copied()
            .map(|pair| source.saved_mission(pair))
            .collect()
        else {
            return Some(Dispatch::Unregistered);
        };
        let mut next_state = state.clone();
        let mut continuity = admitted.continuity.quest_log;
        let Some(result) = next_state.track_missions(&mut continuity, &pairs, limit, observation)
        else {
            return Some(Dispatch::Unregistered);
        };
        let domain = result.ok()?;
        if !authority
            .save_quest_tracker(*admitted, true, selection)
            .await
        {
            return Some(Dispatch::Unregistered);
        }
        next_state.tracker_loaded = true;
        *state = next_state;
        admitted.continuity.quest_log = continuity;
        return Some(Dispatch::QuestLog(domain));
    }

    let Ok(query) = decode_quest_log_query(command.payload) else {
        return Some(Dispatch::Unregistered);
    };
    let observation = authority
        .observe_quest_log(actor, admitted.game_session_id, None)
        .await;
    match &query {
        QuestLogQuery::ListOnly => Some(
            match QuestLogState::list_only(
                observation,
                admitted.continuity.selected_capabilities.contains(
                    oteryn_protocol_oteryn::personal_map_marks::CAPABILITY_PERSONAL_MAP_MARKS_V1,
                ),
            ) {
                Some(payload) => {
                    Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload)
                }
                None => Dispatch::Unregistered,
            },
        ),
        QuestLogQuery::Track { quest_indexes } => {
            let QuestLogObservation::Changed(source) = &observation else {
                return Some(Dispatch::Unregistered);
            };
            let Some(selection) = quest_indexes
                .iter()
                .map(|&index| source.saved_quest(index))
                .collect()
            else {
                return Some(Dispatch::Unregistered);
            };
            let mut next_state = state.clone();
            let mut continuity = admitted.continuity.quest_log;
            let Some(result) = next_state.query(&mut continuity, &query, observation) else {
                return Some(Dispatch::Unregistered);
            };
            let domain = result.ok()?;
            if !authority
                .save_quest_tracker(*admitted, false, selection)
                .await
            {
                return Some(Dispatch::Unregistered);
            }
            next_state.tracker_loaded = true;
            *state = next_state;
            admitted.continuity.quest_log = continuity;
            Some(Dispatch::QuestLog(domain))
        }
        QuestLogQuery::List | QuestLogQuery::Quest { .. } => {
            match state.query(&mut admitted.continuity.quest_log, &query, observation) {
                Some(result) => Some(Dispatch::QuestLog(result.ok()?)),
                None => Some(Dispatch::Unregistered),
            }
        }
    }
}
