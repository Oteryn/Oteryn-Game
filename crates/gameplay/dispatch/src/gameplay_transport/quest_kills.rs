//! Source personal kill counters ride the existing native death queue and revision slot.
use super::{ComposedFreshAdmission, kill_reward::PendingKillSettlement};
use crate::durability::{
    character_progression::{CharacterProgressionError, ExperienceRewardOccurrence},
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
};
use oteryn_gameplay_quests::quest::stampede;
use std::sync::Arc;

pub(super) async fn settle(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    entry: &PendingKillSettlement,
) -> bool {
    if entry.creature != stampede::CREATURE {
        return true;
    }
    let principal = entry.facts.principal;
    if entry.facts.top_damage_character != principal.character {
        return true;
    }
    let Some(catalogue) = &owner.quest_catalogue else {
        return false;
    };
    let fence = match owner.current_quest_fence(principal.session).await {
        Ok(Some(fence)) => fence,
        Ok(None) => return true,
        Err(()) => return false,
    };
    if fence.character_id.as_bytes() != principal.character.as_bytes()
        || fence.character_lease_generation != principal.lease_generation
    {
        return true;
    }
    {
        let runtime = owner.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || !runtime
                .bound_attacker_lease(principal.actor, principal.session)
                .is_ok_and(|lease| {
                    lease.character_id() == principal.character
                        && lease.generation() == principal.lease_generation
                })
        {
            return true;
        }
    }
    let Ok(occurrence) = ExperienceRewardOccurrence::from_bytes(entry.facts.occurrence) else {
        return false;
    };
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: stampede::KILL.into(),
                cause: QuestCause::CreatureDeath(occurrence),
            },
            Arc::clone(catalogue),
        )
        .await;
    match outcome {
        Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => {
            drop(slot);
            owner.refresh_quest_session(principal.session, false).await;
            true
        }
        Ok(QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed) => true,
        Err(
            CharacterProgressionError::Unavailable(_)
            | CharacterProgressionError::CharacterRevisionMismatch,
        ) => {
            owner.schedule_quest_refresh(principal.session);
            false
        }
        Err(error) => {
            super::operator_event(&format!("quest_kill_refused reason={error:?}"));
            true
        }
    }
}
