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
    if oteryn_gameplay_quests::quest::source_deaths::overlord(&entry.creature).is_some() {
        return true; // The frozen group joins the native XP/death transaction instead.
    }
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
    let principal = entry.facts.principal;
    let needs_peer_admission = |recipient: &Recipient| {
        &recipient.character != principal.character.as_bytes()
            && oteryn_gameplay_quests::quest::source_deaths::rule(&recipient.death.transition)
                .is_some_and(|rule| rule.top_damage_party)
    };
    if frozen.source_party.get().is_none()
        && frozen.native.iter().any(needs_peer_admission)
        && let Some(party_at_death) = frozen.source_party_at_death
    {
        let party = match owner.current_quest_fence(principal.session).await {
            Ok(Some(fence))
                if fence.character_id.as_bytes() == principal.character.as_bytes()
                    && fence.character_lease_generation == principal.lease_generation
                    && fence.runtime_scope == entry.facts.death.runtime_scope()
                    && fence.scope_ownership_generation
                        == entry.facts.death.scope_ownership_generation() =>
            {
                let Ok(party) = owner
                    .root
                    .read_source_death_party(
                        owner.character,
                        owner.holder,
                        fence.item_fence(),
                        entry.facts.died_at_unix_ms,
                        party_at_death,
                    )
                    .await
                else {
                    return false;
                };
                party
            }
            Ok(_) => Vec::new(),
            Err(()) => return false,
        };
        let _ = frozen.source_party.set(party);
    }
    for recipient in &frozen.native {
        if needs_peer_admission(recipient)
            && !frozen
                .source_party
                .get()
                .is_some_and(|roster| roster.contains(&recipient.character))
        {
            continue;
        }
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
            .commit_native_quest_kill_with_admission(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestCause::CreatureDeath(occurrence),
                &recipient.death,
                Arc::clone(catalogue),
                &owner.premium_coordinator,
            )
            .await;
        drop(slot);
        match outcome {
            Ok(
                QuestTransitionOutcome::Committed(receipt)
                | QuestTransitionOutcome::AlreadyCommitted(receipt),
            ) => {
                if receipt.completes {
                    let mut runtime = owner.runtime.lock().await;
                    if owner
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
