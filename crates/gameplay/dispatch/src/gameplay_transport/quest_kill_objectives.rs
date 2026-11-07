//! Freeze personal objective eligibility on the lethal turn; drain on the existing kill queue.
use super::{
    ComposedFreshAdmission,
    kill_reward::{KillSettlementQueue, PendingKillSettlement},
};
use crate::durability::{
    character_progression::CharacterProgressionError,
    quest_state::{QuestTransitionOutcome, kill_objectives::NativeObjectiveDeath},
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use oteryn_gameplay_quests::quest::kill_objectives as rules;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct Snapshot {
    actor: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease: u64,
    revision: u64,
    phases: std::collections::BTreeMap<String, i64>,
}

#[derive(Debug, Clone)]
pub(super) struct CapturedObjective {
    snapshot: Snapshot,
    transition: &'static str,
    source: NativeObjectiveDeath,
}

#[derive(Debug, Default)]
pub(super) struct ObjectiveCaptureCache {
    sessions: Vec<Snapshot>,
    deaths: Vec<(ExactActorRef, [u8; 16], Arc<Vec<CapturedObjective>>)>,
}

pub(super) fn capture(
    runtime: &mut ChannelRuntimeV1,
    queue: &KillSettlementQueue,
    entry: &mut PendingKillSettlement,
) {
    let Ok(mut cache) = queue.2.lock() else {
        return;
    };
    cache
        .deaths
        .retain(|(target, _, _)| runtime.retains_corpse_projection(*target));
    if let Some((_, _, original)) = cache
        .deaths
        .iter()
        .find(|(_, occurrence, _)| *occurrence == entry.facts.occurrence)
    {
        entry.objectives = Some(Arc::clone(original));
        return;
    }
    let principal = entry.facts.principal;
    let mut captured = Vec::new();
    if entry.facts.top_damage_character == principal.character {
        for snapshot in &cache.sessions {
            if snapshot.session != principal.session
                || snapshot.actor != principal.actor
                || snapshot.character != *principal.character.as_bytes()
                || snapshot.lease != principal.lease_generation
                || !runtime
                    .bound_attacker_lease(snapshot.actor, snapshot.session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == &snapshot.character
                            && lease.generation() == snapshot.lease
                    })
            {
                continue;
            }
            let at = entry.facts.corpse;
            for rule in rules::OBJECTIVES {
                if rule.creature != entry.creature
                    || snapshot.phases.get(rule.phase_track).copied() != Some(rule.phase_before)
                    || !rule.contains(at.x, at.y, at.floor)
                {
                    continue;
                }
                let Ok(character) = crate::domain::CharacterId::from_bytes(snapshot.character)
                else {
                    continue;
                };
                if let Some(source) = NativeObjectiveDeath::capture(
                    runtime,
                    entry.target,
                    character,
                    snapshot.revision,
                    rule.phase_before,
                ) {
                    captured.push(CapturedObjective {
                        snapshot: snapshot.clone(),
                        transition: rule.transition,
                        source,
                    });
                }
            }
        }
    }
    let captured = Arc::new(captured);
    cache
        .deaths
        .push((entry.target, entry.facts.occurrence, Arc::clone(&captured)));
    entry.objectives = Some(captured);
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn refresh_native_objective_snapshot(&self, session: GameSessionId) {
        let runtime = self.runtime.lock().await;
        let snapshot = self.states.quest_sessions.lock().ok().and_then(|sessions| {
            let copy = sessions.get(&session)?.copy.as_ref()?;
            let revision = copy.character_revision()?.get();
            let phases = rules::OBJECTIVES
                .iter()
                .map(|rule| {
                    (
                        rule.phase_track.to_owned(),
                        copy.tracks().get(rule.phase_track).copied().unwrap_or(0),
                    )
                })
                .collect();
            let actor = runtime
                .positioned_actor_census()
                .ok()?
                .into_iter()
                .find(|(_, _, observed)| *observed == Some(session))?
                .0;
            let lease = runtime.bound_attacker_lease(actor, session).ok()?;
            Some(Snapshot {
                actor,
                session,
                character: *lease.character_id().as_bytes(),
                lease: lease.generation(),
                revision,
                phases,
            })
        });
        let attack = self.states.attack.lock().await;
        let Ok(mut cache) = attack.kills().2.lock() else {
            return;
        };
        cache.sessions.retain(|old| {
            old.session != session && runtime.player_control_facts(old.actor, old.session).is_ok()
        });
        if let Some(snapshot) = snapshot {
            cache.sessions.push(snapshot);
        }
    }

    pub(super) async fn credit_native_objectives(&self, entry: &PendingKillSettlement) -> bool {
        let Some(captured) = &entry.objectives else {
            return true;
        };
        let Some(catalogue) = &self.quest_catalogue else {
            return captured.is_empty();
        };
        for objective in captured.iter() {
            let snapshot = &objective.snapshot;
            let fence = match self.current_quest_fence(snapshot.session).await {
                Ok(Some(fence)) => fence,
                Ok(None) => continue,
                Err(()) => return false,
            };
            if fence.character_id.as_bytes() != &snapshot.character
                || fence.character_lease_generation != snapshot.lease
            {
                continue;
            }
            let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
            // The writer reacquires all gameplay/recovery/root fences and independently
            // proves this exact native principal, creature, scope and death receipt.
            slot.observe_endgame_progression();
            let result = self
                .root
                .commit_character_native_kill_objective(
                    self.character,
                    self.holder,
                    fence,
                    objective.transition,
                    objective.source.clone(),
                    Arc::clone(catalogue),
                )
                .await;
            drop(slot);
            match result {
                Ok(
                    QuestTransitionOutcome::Committed(_)
                    | QuestTransitionOutcome::AlreadyCommitted(_),
                ) => {
                    self.refresh_quest_session(snapshot.session, false).await;
                }
                Ok(
                    QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed,
                ) => {}
                Err(
                    CharacterProgressionError::Unavailable(_)
                    | CharacterProgressionError::CharacterRevisionMismatch,
                ) => {
                    self.schedule_quest_refresh(snapshot.session);
                    return false;
                }
                Err(error) => {
                    super::operator_event(&format!("native_objective_refused reason={error:?}"))
                }
            }
        }
        true
    }
}
