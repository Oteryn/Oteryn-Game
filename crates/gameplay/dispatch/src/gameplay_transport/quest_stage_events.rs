//! Common fenced/sequenced commit path for native quest observations.
use super::{ComposedFreshAdmission, connection::UseCommand};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use std::sync::Arc;

pub(super) async fn commit_keys(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    keys: Vec<String>,
) {
    let Some(catalogue) = owner.quest_catalogue.as_ref() else {
        return;
    };
    if keys.is_empty() {
        return;
    }
    if owner
        .runtime
        .lock()
        .await
        .player_control_facts(actor, command.game_session_id)
        .is_err()
    {
        return;
    }
    let Ok(ordinal) = CommandId::new(command.command_id) else {
        return;
    };
    for key in keys {
        let Ok(Some(fence)) = owner.current_quest_fence(command.game_session_id).await else {
            return;
        };
        if command.item_fence != Some(fence.item_fence()) {
            return;
        }
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: key,
                    cause: QuestCause::Command(CommandRef::new(command.game_session_id, ordinal)),
                },
                Arc::clone(catalogue),
            )
            .await;
        drop(slot);
        match outcome {
            Ok(
                QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_),
            ) => {
                owner
                    .refresh_quest_session(command.game_session_id, false)
                    .await
            }
            Ok(_) => {}
            Err(error) => {
                super::operator_event(&format!("quest_stage_observation_refused reason={error:?}"));
                owner.schedule_quest_refresh(command.game_session_id);
                return;
            }
        }
    }
}
