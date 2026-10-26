//! TASKBOARD-1 weekly Task Board composed from the admitted actor and current Channel owner
//! (`OTERYN_GAME_TASKBOARD1_SCOPE_DECISION_2026-10-25.md` §2). Every write is one quest
//! transition of the code-built board quest in the Character's revision slot.
use std::sync::Arc;

use super::{ComposedFreshAdmission, GameSessionId, connection::AdmittedSession, operator_event};
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceRewardOccurrence,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, RuntimeScopeRefV1};
use oteryn_gameplay_quests::quest::task_board;
use oteryn_protocol_oteryn::task_board as wire;

#[derive(Debug)]
pub enum TaskBoardReply {
    Rejected,
    Payload(Vec<u8>),
    Unavailable,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn task_board_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> TaskBoardReply {
        let Ok(request) = wire::decode_task_board_request(payload) else {
            return TaskBoardReply::Rejected;
        };
        if kind != wire::COMMAND_TYPE_TASK_BOARD
            || wire::encode_task_board_request(&request).is_err()
        {
            return TaskBoardReply::Rejected;
        }
        let Some(item_fence) = admitted.item_fence else {
            return TaskBoardReply::Rejected;
        };
        let Ok(command) = CommandId::new(id) else {
            return TaskBoardReply::Rejected;
        };
        if self.quest_catalogue.is_none() {
            return TaskBoardReply::Rejected;
        }
        {
            let runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_TASK_BOARD_V1)
                || item_fence.game_session_id != admitted.game_session_id
                || item_fence.connection_generation.get()
                    != admitted.continuity.connection_generation
                || admitted.world_id != binding.world_id()
                || admitted.channel_id != binding.channel_id()
                || item_fence.runtime_scope
                    != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
                || item_fence.scope_ownership_generation != binding.scope_generation()
                || runtime.owner_fence().is_err()
                || !admitted.runtime_actor.is_some_and(|actor| {
                    runtime
                        .player_control_facts(actor, admitted.game_session_id)
                        .is_ok_and(|facts| facts.control_loss.is_none())
                })
            {
                return TaskBoardReply::Rejected;
            }
        } // No runtime mutex is held across the revision slot or the durable write.
        let session = admitted.game_session_id;
        let transition_key = match request.action {
            0 => None,
            1 => Some(task_board::select_transition(request.task)),
            2 => Some(task_board::claim_transition(request.task)),
            3 => Some(task_board::bounty_select_transition(request.task)),
            4 => Some(task_board::bounty_claim_transition(request.task)),
            _ => return TaskBoardReply::Rejected,
        };
        let mut refused = false;
        if let Some(transition_key) = transition_key {
            let Ok(Some(fence)) = self.current_quest_fence(session).await else {
                return TaskBoardReply::Rejected;
            };
            if fence.character_id.as_bytes() != item_fence.character_id.as_bytes()
                || fence.connection_generation != item_fence.connection_generation
                || fence.character_lease_generation != item_fence.character_lease_generation
                || fence.runtime_scope != item_fence.runtime_scope
                || fence.scope_ownership_generation != item_fence.scope_ownership_generation
            {
                return TaskBoardReply::Rejected;
            }
            let cause = QuestCause::Command(CommandRef::new(session, command));
            match self.commit_task_board(fence, transition_key, cause).await {
                Some(QuestTransitionOutcome::Refused(_)) => refused = true,
                Some(_) => {}
                None => return TaskBoardReply::Rejected,
            }
            self.refresh_quest_session(session, false).await;
        }
        let Some(tracks) = self.task_board_tracks(session) else {
            return TaskBoardReply::Rejected;
        };
        wire::encode_task_board_reply(&board_reply(refused, &tracks))
            .map_or(TaskBoardReply::Unavailable, TaskBoardReply::Payload)
    }

    /// One board transition in the Character's revision slot; `None` when the write failed
    /// without an outcome.
    async fn commit_task_board(
        &self,
        fence: CurrentCharacterGameplayFence,
        transition_key: String,
        cause: QuestCause,
    ) -> Option<QuestTransitionOutcome> {
        let catalogue = self.quest_catalogue.as_ref()?;
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                self.root,
                self.character,
                self.holder,
                fence,
                QuestTransitionRequest {
                    transition_key,
                    cause,
                },
                Arc::clone(catalogue),
            )
            .await;
        drop(slot);
        match outcome {
            Ok(outcome) => Some(outcome),
            Err(error) => {
                operator_event(&format!("task_board_write_failed reason={error:?}"));
                None
            }
        }
    }

    /// The board tracks of the session's current quest copy; `None` while the copy is unusable.
    fn task_board_tracks(
        &self,
        session: GameSessionId,
    ) -> Option<std::collections::BTreeMap<String, i64>> {
        let sessions = self.states.quest_sessions.lock().ok()?;
        let copy = sessions.get(&session)?.copy.as_ref()?;
        Some(
            copy.tracks()
                .range(task_board::QUEST.to_owned()..)
                .take_while(|(key, _)| key.starts_with(task_board::QUEST))
                .map(|(key, value)| (key.clone(), *value))
                .collect(),
        )
    }

    /// §2.4: one kill of `race` settled for the death's reward principal credits that
    /// Character's active task of the race, keyed by the same memoized (death, Character)
    /// occurrence as the XP award, so a settle retry never counts twice.
    pub(super) async fn credit_task_board_kill(
        &self,
        fence: CurrentCharacterGameplayFence,
        race: &str,
        occurrence: [u8; 16],
    ) {
        let session = fence.game_session_id;
        let Some(tracks) = self.task_board_tracks(session) else {
            return;
        };
        let transition_keys = [
            task_board::kill_credit(&tracks, race),
            task_board::bounty_kill_credit(&tracks, race),
        ];
        let Ok(occurrence) = ExperienceRewardOccurrence::from_bytes(occurrence) else {
            return;
        };
        for transition_key in transition_keys.into_iter().flatten() {
            match self
                .commit_task_board(fence, transition_key, QuestCause::CreatureDeath(occurrence))
                .await
            {
                Some(QuestTransitionOutcome::Committed(_)) => {
                    self.refresh_quest_session(session, false).await;
                }
                Some(_) => {}
                None => self.schedule_quest_refresh(session),
            }
        }
    }
}

fn board_reply(refused: bool, tracks: &std::collections::BTreeMap<String, i64>) -> wire::TaskBoardReply {
    let state = task_board::state(tracks);
    wire::TaskBoardReply {
        refused,
        selected: state.selected,
        kills: state.kills,
        points: state.points,
        tasks: task_board::TASKS
            .iter()
            .zip(state.claimed_at)
            .map(|(task, claimed_at_unix)| wire::TaskBoardTask {
                race: task.race.to_owned(),
                required: task.required,
                claimed_at_unix,
            })
            .collect(),
    }
}

macro_rules! authority_methods {
    () => {
        fn task_board_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _kind: u32,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = $crate::gameplay_transport::task_board::TaskBoardReply>
        {
            async { $crate::gameplay_transport::task_board::TaskBoardReply::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn task_board_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
        ) -> task_board::TaskBoardReply {
            self.task_board_wire(admitted, id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
#[path = "task_board_tests.rs"]
mod tests;
