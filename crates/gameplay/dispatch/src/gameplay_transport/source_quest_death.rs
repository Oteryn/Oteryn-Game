//! Source-qualified damaging-player callbacks on the existing native kill queue.
use super::{
    ComposedFreshAdmission,
    kill_reward::{KillSettlementQueue, PendingKillSettlement, SettleReport, SettleVerdict},
};
use crate::durability::{
    character_progression::{CharacterProgressionError, ExperienceRewardOccurrence},
    quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
        creature_death::CreatureDeathContext,
    },
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use oteryn_gameplay_quests::quest::creature_death as rules;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct Snapshot {
    actor: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease: u64,
    revision: u64,
    callbacks: Vec<usize>,
}
#[derive(Debug, Clone, Default)]
pub(super) struct FrozenSourceDeath {
    recipients: Vec<Snapshot>,
}

/// Bounded by live player census and retained native corpse projections. This is
/// capture memory, not a second settlement queue or persistent authority.
#[derive(Debug, Default)]
pub(super) struct CaptureCache {
    sessions: Vec<Snapshot>,
    deaths: Vec<(ExactActorRef, [u8; 16], Arc<FrozenSourceDeath>)>,
}

pub(super) fn capture(
    runtime: &ChannelRuntimeV1,
    queue: &KillSettlementQueue,
    entry: &mut PendingKillSettlement,
) {
    let Ok(mut cache) = queue.3.lock() else {
        return;
    };
    cache
        .deaths
        .retain(|(target, _, _)| runtime.retains_corpse_projection(*target));
    cache.sessions.retain(|snapshot| {
        runtime
            .player_control_facts(snapshot.actor, snapshot.session)
            .is_ok()
    });
    if !rules::tracks_creature(&entry.creature) || !runtime.retains_corpse_projection(entry.target)
    {
        return;
    }
    if let Some((_, _, frozen)) = cache
        .deaths
        .iter()
        .find(|(_, occurrence, _)| *occurrence == entry.facts.occurrence)
    {
        entry.source_death = Some(Arc::clone(frozen));
        return;
    }
    let recipients = cache
        .sessions
        .iter()
        .filter_map(|snapshot| {
            let damaged = entry.damage_shares.iter().any(|share| {
                share.total > 0
                    && share.character.as_bytes() == &snapshot.character
                    && share.session == Some(snapshot.session)
                    && share.lease_generation == Some(snapshot.lease)
            });
            if !damaged
                || !runtime
                    .bound_attacker_lease(snapshot.actor, snapshot.session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == &snapshot.character
                            && lease.generation() == snapshot.lease
                    })
            {
                return None;
            }
            let mut recipient = snapshot.clone();
            recipient
                .callbacks
                .retain(|index| rules::CALLBACKS[*index].creature == entry.creature);
            (!recipient.callbacks.is_empty()).then_some(recipient)
        })
        .collect();
    let frozen = Arc::new(FrozenSourceDeath { recipients });
    cache
        .deaths
        .push((entry.target, entry.facts.occurrence, Arc::clone(&frozen)));
    entry.source_death = Some(frozen);
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn refresh_source_death_snapshot(&self, session: GameSessionId) {
        let runtime = self.runtime.lock().await;
        let snapshot = {
            let sessions = self.states.quest_sessions.lock().ok();
            sessions
                .as_ref()
                .and_then(|sessions| sessions.get(&session))
                .and_then(|entry| entry.copy.as_ref())
                .and_then(|copy| {
                    let revision = copy.character_revision()?.get();
                    let actor = runtime
                        .positioned_actor_census()
                        .ok()?
                        .into_iter()
                        .find(|(_, _, observed)| *observed == Some(session))?
                        .0;
                    let lease = runtime.bound_attacker_lease(actor, session).ok()?;
                    let callbacks = rules::CALLBACKS
                        .iter()
                        .enumerate()
                        .filter_map(|(index, callback)| {
                            (copy.tracks().get(callback.track) == Some(&callback.expected))
                                .then_some(index)
                        })
                        .collect();
                    Some(Snapshot {
                        actor,
                        session,
                        character: *lease.character_id().as_bytes(),
                        lease: lease.generation(),
                        revision,
                        callbacks,
                    })
                })
        };
        let attack = self.states.attack.lock().await;
        let Ok(mut cache) = attack.kills().3.lock() else {
            return;
        };
        cache.sessions.retain(|snapshot| {
            snapshot.session != session
                && runtime
                    .player_control_facts(snapshot.actor, snapshot.session)
                    .is_ok()
        });
        if let Some(snapshot) = snapshot {
            cache.sessions.push(snapshot);
        }
    }

    async fn credit_source_death(&self, entry: &PendingKillSettlement) -> bool {
        let (Some(frozen), Some(catalogue)) = (&entry.source_death, &self.quest_catalogue) else {
            return true;
        };
        let Ok(occurrence) = ExperienceRewardOccurrence::from_bytes(entry.facts.occurrence) else {
            return true;
        };
        for recipient in &frozen.recipients {
            let fence = match self.current_quest_fence(recipient.session).await {
                Ok(Some(fence)) => fence,
                Ok(None) => continue,
                Err(()) => return false,
            };
            if fence.character_id.as_bytes() != &recipient.character
                || fence.game_session_id != recipient.session
                || fence.character_lease_generation != recipient.lease
            {
                continue;
            }
            {
                let runtime = self.runtime.lock().await;
                if runtime.owner_fence().is_err()
                    || !runtime
                        .bound_attacker_lease(recipient.actor, recipient.session)
                        .is_ok_and(|lease| {
                            lease.character_id().as_bytes() == &recipient.character
                                && lease.generation() == recipient.lease
                        })
                {
                    continue;
                }
            }
            let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
            let mut changed = false;
            for index in &recipient.callbacks {
                let request = QuestTransitionRequest {
                    transition_key: rules::CALLBACKS[*index].transition.into(),
                    cause: QuestCause::CreatureDeath(occurrence),
                };
                let result = slot
                    .commit_creature_death(
                        self.root,
                        self.character,
                        self.holder,
                        fence,
                        request,
                        Arc::clone(catalogue),
                        CreatureDeathContext {
                            callback: *index,
                            observed_revision: recipient.revision,
                        },
                    )
                    .await;
                match result {
                    Ok(
                        QuestTransitionOutcome::Committed(_)
                        | QuestTransitionOutcome::AlreadyCommitted(_),
                    ) => changed = true,
                    Ok(
                        QuestTransitionOutcome::Refused(_)
                        | QuestTransitionOutcome::ObligationClosed,
                    ) => {}
                    Err(
                        CharacterProgressionError::Unavailable(_)
                        | CharacterProgressionError::CharacterRevisionMismatch,
                    ) => {
                        self.schedule_quest_refresh(recipient.session);
                        return false;
                    }
                    Err(error) => super::operator_event(&format!(
                        "source_quest_death_refused reason={error:?}"
                    )),
                }
            }
            drop(slot);
            if changed {
                self.refresh_quest_session(recipient.session, false).await;
            }
        }
        true
    }
}
pub(super) async fn settle(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    entry: &PendingKillSettlement,
    ordinary: impl std::future::Future<Output = SettleReport>,
) -> SettleReport {
    if !admission.credit_source_death(entry).await {
        return SettleReport {
            verdict: SettleVerdict::Retry,
            no_progression: false,
        };
    }
    ordinary.await
}
