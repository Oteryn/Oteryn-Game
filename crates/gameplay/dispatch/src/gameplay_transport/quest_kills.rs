//! Source personal kill counters ride the existing native death queue and revision slot.
use super::{ComposedFreshAdmission, kill_reward::PendingKillSettlement};
use crate::durability::{
    character_progression::{CharacterProgressionError, ExperienceRewardOccurrence},
    quest_state::{QuestCause, QuestTransitionOutcome},
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Recipient {
    pub(super) actor: crate::foundation::ExactActorRef,
    pub(super) session: crate::foundation::GameSessionId,
    pub(super) character: [u8; 16],
    pub(super) lease_generation: u64,
    pub(super) death: crate::durability::quest_state::native_kills::NativeQuestDeath,
}

pub(super) async fn settle(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    entry: &PendingKillSettlement,
) -> bool {
    if !owner.quest_catalogue.as_deref().map_or_else(
        || oteryn_gameplay_quests::quest::native_kills::tracks_creature(&entry.creature),
        |catalogue| {
            oteryn_gameplay_quests::quest::native_kills::tracks_creature_for(
                catalogue,
                &entry.creature,
            )
        },
    ) && !oteryn_gameplay_quests::quest::source_deaths::tracks_creature(&entry.creature)
    {
        return true;
    }
    let Some(frozen) = &entry.grizzly else {
        return false;
    };
    if frozen.native.is_empty() {
        return true;
    }
    let Some(catalogue) = &owner.quest_catalogue else {
        return false;
    };
    let Ok(occurrence) = ExperienceRewardOccurrence::from_bytes(entry.facts.occurrence) else {
        return false;
    };
    for recipient in &frozen.native {
        let fence = match owner.current_quest_fence(recipient.session).await {
            Ok(Some(fence)) => fence,
            Ok(None) => continue,
            Err(()) => return false,
        };
        if fence.character_id.as_bytes() != &recipient.character
            || fence.character_lease_generation != recipient.lease_generation
        {
            continue;
        }
        {
            let runtime = owner.runtime.lock().await;
            if runtime.owner_fence().is_err()
                || !runtime
                    .bound_attacker_lease(recipient.actor, recipient.session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == &recipient.character
                            && lease.generation() == recipient.lease_generation
                    })
            {
                continue;
            }
        }
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_native_quest_kill(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestCause::CreatureDeath(occurrence),
                &recipient.death,
                Arc::clone(catalogue),
            )
            .await;
        drop(slot);
        match outcome {
            Ok(
                QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_),
            ) => {
                owner.refresh_quest_session(recipient.session, false).await;
            }
            Ok(QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed) => {}
            Err(
                CharacterProgressionError::Unavailable(_)
                | CharacterProgressionError::CharacterRevisionMismatch,
            ) => {
                owner.schedule_quest_refresh(recipient.session);
                return false;
            }
            Err(error) => super::operator_event(&format!("quest_kill_refused reason={error:?}")),
        }
    }
    true
}
