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

impl Recipient {
    fn matches_live_session(&self, runtime: &crate::foundation::ChannelRuntimeV1) -> bool {
        runtime.owner_fence().is_ok()
            && runtime
                .bound_attacker_lease(self.actor, self.session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == &self.character
                        && lease.generation() == self.lease_generation
                })
    }
}

pub(super) async fn settle(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    entry: &PendingKillSettlement,
) -> bool {
    // Encounter-only counters are absent from ordinary creature discovery.
    // Their admitted phase owner already froze exact recipients on this death;
    // the durable writer still revalidates each counter and its full source fence.
    if !owner.quest_catalogue.as_deref().map_or_else(
        || oteryn_gameplay_quests::quest::native_kills::tracks_creature(&entry.creature),
        |catalogue| {
            oteryn_gameplay_quests::quest::native_kills::tracks_creature_for(
                catalogue,
                &entry.creature,
            )
        },
    ) && !oteryn_gameplay_quests::quest::source_deaths::tracks_creature(&entry.creature)
        && entry
            .grizzly
            .as_ref()
            .is_none_or(|frozen| frozen.native.is_empty())
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
            if !recipient.matches_live_session(&runtime) {
                continue;
            }
        }
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_native_quest_kill_with_admission(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestCause::CreatureDeath(occurrence),
                &recipient.death,
                Arc::clone(catalogue),
                &owner.premium_coordinator,
                owner
                    .active_generation
                    .and_then(|generation| generation.native_gameplay()),
            )
            .await;
        drop(slot);
        match outcome {
            Ok(
                QuestTransitionOutcome::Committed(receipt)
                | QuestTransitionOutcome::AlreadyCommitted(receipt),
            ) => {
                if receipt.completes || receipt.native_feedback.is_some() {
                    let mut runtime = owner.runtime.lock().await;
                    if receipt.completes
                        && owner
                            .states
                            .quest_callbacks
                            .complete_native_quest(
                                &mut runtime,
                                &receipt,
                                recipient.session,
                                recipient.death.observed_revision,
                            )
                            .is_err()
                    {
                        runtime.retire_owner_cycle();
                        return false;
                    }
                    if let Some(message) = receipt.native_feedback.as_deref()
                        && recipient.matches_live_session(&runtime)
                    {
                        owner
                            .states
                            .chat
                            .push_quest_feedback(recipient.session, message);
                    }
                }
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
