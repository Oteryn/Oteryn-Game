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
        let occurrence =
            if let crate::durability::quest_state::native_kills::NativeDeathSource::Objective(
                source,
            ) = &recipient.death.source
            {
                let Ok(occurrence) = source.occurrence() else {
                    return false;
                };
                occurrence
            } else {
                occurrence
            };
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

/// The durable advanced stage is the removal intent. Admission/refresh replays it
/// after a crash, without replaying the kill or depending on an in-memory callback.
pub(super) async fn refresh_creature_removals(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
) -> bool {
    use oteryn_gameplay_quests::quest::authored_bindings::Trigger;
    let Some(catalogue) = owner.quest_catalogue.as_deref() else {
        return true;
    };
    let Some(bindings) = catalogue.authored_bindings() else {
        return true;
    };
    let tracks = owner
        .states
        .quest_sessions
        .lock()
        .ok()
        .and_then(|sessions| {
            let entry = sessions.get(&session)?;
            let copy = entry.copy.as_ref()?;
            Some((
                entry.version,
                copy.character_revision()?.get(),
                copy.read_tracks(catalogue)?,
            ))
        });
    let Some((version, revision, tracks)) = tracks else {
        return true;
    };
    let mut removals: Vec<_> = bindings
        .quests
        .iter()
        .flat_map(|quest| &quest.stages)
        .filter_map(|stage| match &stage.trigger {
            Trigger::Kill { counter }
                if counter.advance_gate_to.is_some()
                    && tracks.get(&counter.gate).copied() == counter.advance_gate_to
                    && tracks.get(&counter.track).copied() == Some(counter.maximum) =>
            {
                Some(&counter.remove_on_advance)
            }
            _ => None,
        })
        .flatten()
        .collect();
    removals.extend(
        bindings
            .quests
            .iter()
            .flat_map(|quest| &quest.creature_removals)
            .filter(|removal| tracks.get(&removal.track).copied() == Some(removal.value))
            .flat_map(|removal| &removal.creatures),
    );
    if removals.is_empty() {
        return true;
    }
    let Ok(Some(fence)) = owner.current_quest_fence(session).await else {
        return false;
    };
    if fence.expected_character_revision.get() != revision {
        return false;
    }
    let mut runtime = owner.runtime.lock().await;
    if !owner.states.quest_sessions.lock().is_ok_and(|sessions| {
        sessions
            .get(&session)
            .is_some_and(|entry| entry.version == version)
    }) || runtime.binding().runtime_scope() != fence.runtime_scope
        || runtime.binding().scope_generation() != fence.scope_ownership_generation
        || runtime.owner_fence().is_err()
    {
        return false;
    }
    let present = runtime.positioned_actor_census().is_ok_and(|census| {
        census.iter().any(|(actor, _, observed)| {
            *observed == Some(session)
                && runtime
                    .bound_attacker_lease(*actor, session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
        })
    });
    if !present {
        return false;
    }
    let Ok(census) = runtime.current_live_creature_census() else {
        return false;
    };
    let mut selected = Vec::new();
    for actor in census {
        let (Ok(identity), Ok(position)) = (
            runtime.creature_spell_target_atom(actor),
            runtime.read_actor_position(actor),
        ) else {
            return false;
        };
        let position = position.position();
        if removals.iter().any(|removal| {
            removal.matches(
                &identity,
                [position.x, position.y, i32::from(position.floor)],
            )
        }) {
            selected.push(actor);
        }
    }
    for actor in selected {
        if runtime.remove_encounter_actor(actor).is_err() {
            return false;
        }
    }
    true
}
