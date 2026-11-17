//! The existing native death queue credits the personal Kollos objective only.
use super::{ComposedFreshAdmission, kill_reward::PendingKillSettlement};
use crate::durability::{
    character_progression::{CurrentCharacterGameplayFence, ExperienceRewardOccurrence},
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
};
use oteryn_gameplay_quests::quest::shared_world_progress;

/// Called by DurableKillSettle for its independently checked queue principal.
/// Source death identity, frozen creature and current Character fence are supplied
/// by that existing owner; client commands cannot choose a kill objective.
pub(super) async fn settle(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    entry: &PendingKillSettlement,
    fence: CurrentCharacterGameplayFence,
) -> bool {
    let Some(key) = shared_world_progress::kill_objective(&entry.creature) else {
        return true;
    };
    let Some(catalogue) = &admission.quest_catalogue else {
        return true;
    };
    let Ok(occurrence) = ExperienceRewardOccurrence::from_bytes(entry.facts.occurrence) else {
        return false;
    };
    let mut slot = admission
        .revision_sequencer
        .acquire(fence.character_id)
        .await;
    let outcome = slot
        .commit_quest_transition(
            admission.root,
            admission.character,
            admission.holder,
            fence,
            QuestTransitionRequest {
                transition_key: key.into(),
                cause: QuestCause::CreatureDeath(occurrence),
            },
            std::sync::Arc::clone(catalogue),
        )
        .await;
    drop(slot);
    match outcome {
        Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => {
            admission.schedule_quest_refresh(fence.game_session_id);
            true
        }
        Ok(QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed) => true,
        Err(_) => {
            admission.schedule_quest_refresh(fence.game_session_id);
            false
        }
    }
}
