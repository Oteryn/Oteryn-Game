//! Native inventory moves drive read-only imported COLLECT predicates.
use super::{ComposedFreshAdmission, connection::UseCommand};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_quests::quest::authored_bindings::Trigger;
use std::sync::Arc;

pub(super) async fn after_move(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
) {
    let Some(catalogue) = owner.quest_catalogue.as_ref() else {
        return;
    };
    let Some(bindings) = catalogue.authored_bindings() else {
        return;
    };
    let keys: Vec<_> = {
        let Ok(sessions) = owner.states.quest_sessions.lock() else {
            return;
        };
        let Some(copy) = sessions
            .get(&command.game_session_id)
            .and_then(|state| state.copy.as_ref())
        else {
            return;
        };
        bindings
            .quests
            .iter()
            .flat_map(|quest| &quest.stages)
            .filter_map(|stage| {
                let Trigger::Collect { collection } = &stage.trigger else {
                    return None;
                };
                let active = collection.storage.iter().all(|guard| {
                    catalogue.track(&guard.track).is_some_and(|track| {
                        let value = copy
                            .tracks()
                            .get(&guard.track)
                            .copied()
                            .unwrap_or(track.initial);
                        (guard.minimum..=guard.maximum).contains(&value)
                    })
                });
                if active {
                    Some(collection.key.clone())
                } else {
                    None
                }
            })
            .collect()
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
                super::operator_event(&format!("quest_collection_refused reason={error:?}"));
                owner.schedule_quest_refresh(command.game_session_id);
                return;
            }
        }
    }
}
