//! Personal native goals share the retained lethal queue and exact principal fence.
use super::*;
use oteryn_gameplay_quests::quest::native_kill_goals as goals;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Recipient {
    actor: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease_generation: u64,
    goals: Vec<String>,
}

pub(super) fn active(tracks: &std::collections::BTreeMap<String, i64>) -> Vec<String> {
    goals::load()
        .map(|goals| {
            goals
                .iter()
                .filter(|g| g.active(tracks))
                .map(|g| g.key.clone())
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn tracks_creature(creature: &str) -> bool {
    goals::load().is_ok_and(|goals| {
        goals
            .iter()
            .any(|g| g.origins.iter().any(|p| p.creature == creature))
    })
}

pub(super) fn freeze(
    runtime: &ChannelRuntimeV1,
    entry: &PendingKillSettlement,
    present: bool,
    snapshots: &[TaskSnapshot],
) -> Option<Recipient> {
    if !present {
        return None;
    }
    let principal = entry.facts.principal;
    let snapshot = snapshots.iter().find(|s| {
        s.actor == principal.actor
            && s.session == principal.session
            && &s.character == principal.character.as_bytes()
            && s.lease_generation == principal.lease_generation
            && s.observed_revision > 0
            && !s.active_goals.is_empty()
    })?;
    let origin = runtime.creature_death_spawn_cell(entry.target)?;
    let actions: Vec<_> = goals::load()
        .ok()?
        .iter()
        .filter(|g| {
            snapshot.active_goals.contains(&g.key)
                && g.matches(&entry.creature, origin.x, origin.y, origin.floor)
        })
        .map(|g| g.key.clone())
        .collect();
    (!actions.is_empty()).then_some(Recipient {
        actor: snapshot.actor,
        session: snapshot.session,
        character: snapshot.character,
        lease_generation: snapshot.lease_generation,
        goals: actions,
    })
}

pub(super) async fn settle(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    entry: &PendingKillSettlement,
) -> bool {
    let Some(recipient) = entry.grizzly.as_ref().and_then(|f| f.native_goals.as_ref()) else {
        return true;
    };
    let Some(catalogue) = &admission.quest_catalogue else {
        return false;
    };
    let fence = match admission.current_quest_fence(recipient.session).await {
        Ok(Some(fence)) => fence,
        Ok(None) => return true,
        Err(()) => return false,
    };
    if fence.character_id.as_bytes() != &recipient.character
        || fence.game_session_id != recipient.session
        || fence.character_lease_generation != recipient.lease_generation
    {
        return true;
    }
    {
        let runtime = admission.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || !runtime
                .bound_attacker_lease(recipient.actor, recipient.session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == &recipient.character
                        && lease.generation() == recipient.lease_generation
                })
        {
            return true;
        }
    }
    let Ok(occurrence) = ExperienceRewardOccurrence::from_bytes(entry.facts.occurrence) else {
        return true;
    };
    let mut slot = admission
        .revision_sequencer
        .acquire(fence.character_id)
        .await;
    let mut changed = false;
    for goal in &recipient.goals {
        let outcome = slot
            .commit_quest_transition(
                admission.root,
                admission.character,
                admission.holder,
                fence,
                crate::durability::quest_state::QuestTransitionRequest {
                    transition_key: goal.clone(),
                    cause: QuestCause::CreatureDeath(occurrence),
                },
                Arc::clone(catalogue),
            )
            .await;
        match outcome {
            Ok(
                QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_),
            ) => changed = true,
            Ok(QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed) => {}
            Err(
                CharacterProgressionError::Unavailable(_)
                | CharacterProgressionError::CharacterRevisionMismatch,
            ) => {
                admission.schedule_quest_refresh(recipient.session);
                return false;
            }
            Err(error) => {
                super::super::operator_event(&format!("native_kill_goal_refused reason={error:?}"))
            }
        }
    }
    drop(slot);
    if changed {
        admission
            .refresh_quest_session(recipient.session, false)
            .await;
    }
    true
}
