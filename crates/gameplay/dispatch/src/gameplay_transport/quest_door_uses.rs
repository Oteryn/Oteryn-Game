//! Project only the native key-door owner's opaque successful opening.
use super::{ComposedFreshAdmission, connection::UseCommand};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::quest_state::{
    QuestCause, QuestTransitionOutcome, native_doors::NativeQuestDoorOpening,
};
use crate::foundation::{CommandId, CommandRef};
use crate::map::door::KeyDoorOpening;
use std::sync::Arc;

pub(super) async fn after_opening(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterGameplayFence,
    command: UseCommand,
    key_instance: [u8; 16],
    placement: &[u8],
    opening: KeyDoorOpening,
) {
    let Ok(placement): Result<[u8; 40], _> = placement.try_into() else {
        return;
    };
    if placement[32..] != opening.placement_key().to_be_bytes()
        || command.game_session_id != fence.game_session_id
        || command.item_fence != Some(fence.item_fence())
    {
        return;
    }
    let Some(catalogue) = owner.quest_catalogue.as_ref() else {
        return;
    };
    let Some(bindings) = catalogue.authored_bindings() else {
        return;
    };
    let keys = {
        let Ok(sessions) = owner.states.quest_sessions.lock() else {
            return;
        };
        let Some(copy) = sessions
            .get(&command.game_session_id)
            .and_then(|state| state.copy.as_ref())
        else {
            return;
        };
        bindings.door_use_candidates(catalogue, copy.tracks(), &placement, opening.key_number())
    };
    let Ok(ordinal) = CommandId::new(command.command_id) else {
        return;
    };
    for key in keys {
        let Some(hash) = catalogue
            .transition(&key)
            .and_then(|transition| catalogue.definition_hash(&transition.quest))
        else {
            continue;
        };
        let captured = NativeQuestDoorOpening {
            transition: key,
            placement,
            key_instance,
            key_number: opening.key_number(),
            door_revision: opening.revision(),
            observed_character_revision: fence.expected_character_revision.get(),
            definition_hash: hash,
        };
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_native_quest_door(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestCause::Command(CommandRef::new(command.game_session_id, ordinal)),
                &captured,
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
                super::operator_event(&format!("quest_door_observation_refused reason={error:?}"));
                owner.schedule_quest_refresh(command.game_session_id);
                return;
            }
        }
    }
}
