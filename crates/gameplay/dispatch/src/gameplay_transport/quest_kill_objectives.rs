//! Freeze personal objective eligibility on the lethal turn; drain on the existing kill queue.
use super::{
    ComposedFreshAdmission,
    kill_reward::{KillSettlementQueue, PendingKillSettlement},
};
use crate::durability::{
    character_progression::CharacterProgressionError,
    quest_state::kill_objectives::NativeObjectiveDeath,
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
    let Ok(mut cache) = queue.3.lock() else {
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
        self.refresh_native_objective_snapshot_in_runtime(&runtime, session)
            .await;
    }

    /// Publish capture eligibility while the callback already holds the runtime owner.
    pub(super) async fn refresh_native_objective_snapshot_in_runtime(
        &self,
        runtime: &ChannelRuntimeV1,
        session: GameSessionId,
    ) {
        let snapshot = self.states.quest_sessions.lock().ok().and_then(|sessions| {
            let copy = sessions.get(&session)?.copy.as_ref()?;
            let revision = copy.character_revision()?.get();
            let tracks = copy.read_tracks(self.quest_catalogue.as_deref()?)?;
            let phases = rules::OBJECTIVES
                .iter()
                .map(|rule| {
                    (
                        rule.phase_track.to_owned(),
                        tracks.get(rule.phase_track).copied().unwrap_or(0),
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
        let Ok(mut cache) = attack.kills().3.lock() else {
            return;
        };
        if snapshot.as_ref().is_some_and(|new| {
            cache.sessions.iter().any(|old| {
                old.session == new.session
                    && old.character == new.character
                    && old.lease == new.lease
                    && old.revision > new.revision
            })
        }) {
            return;
        }
        cache.sessions.retain(|old| {
            old.session != session && runtime.player_control_facts(old.actor, old.session).is_ok()
        });
        if let Some(snapshot) = snapshot {
            cache.sessions.push(snapshot);
        }
    }

    pub(super) async fn refresh_native_objective_recipients(&self, entry: &PendingKillSettlement) {
        if let Some(session) = super::grizzly_kills::native_goals::session(entry) {
            self.refresh_quest_session(session, false).await;
        }
        if let Some(captured) = &entry.objectives {
            for objective in captured.iter() {
                self.refresh_quest_session(objective.snapshot.session, false)
                    .await;
            }
        }
    }
}

/// The XP owner holds all captured Character slots and the native Channel turn.
/// Only the exact original damage principal may consume a captured objective.
pub(super) async fn apply_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
    entry: &PendingKillSettlement,
    participants: &[(
        crate::durability::character_progression::CurrentCharacterGameplayFence,
        bool,
    )],
    catalogue: Option<&oteryn_gameplay_quests::quest::QuestStateCatalogue>,
) -> Result<Vec<crate::durability::quest_state::CommittedQuestTransition>, CharacterProgressionError>
{
    let principal = entry.facts.principal;
    let mut requests = super::grizzly_kills::native_goals::requests(entry)?;
    if let Some(captured) = &entry.objectives {
        for objective in captured.iter() {
            let snapshot = &objective.snapshot;
            if snapshot.actor != principal.actor
                || snapshot.session != principal.session
                || snapshot.character != *principal.character.as_bytes()
                || snapshot.lease != principal.lease_generation
            {
                return Err(CharacterProgressionError::AuthorityRejected);
            }
            requests.push((objective.transition, &objective.source));
        }
    }
    if requests.is_empty() {
        return Ok(Vec::new());
    }
    let fence = participants
        .iter()
        .find_map(|(fence, damaged)| {
            (*damaged
                && fence.character_id.as_bytes() == principal.character.as_bytes()
                && fence.game_session_id == principal.session
                && fence.character_lease_generation == principal.lease_generation)
                .then_some(*fence)
        })
        .ok_or(CharacterProgressionError::AuthorityRejected)?;
    let catalogue = catalogue.ok_or(CharacterProgressionError::InvalidInput)?;
    let receipt = crate::durability::quest_state::kill_objectives::apply_batch_in_transaction(
        tx, node, fence, &requests, catalogue,
    )
    .await?;
    Ok(receipt.into_iter().collect())
}

/// Publish only after the enclosing XP/Quest transaction commits, while the
/// lethal owner still holds runtime/attack guards. A replay cannot regress it.
pub(super) fn publish(
    queue: &KillSettlementQueue,
    committed: &[crate::durability::quest_state::CommittedQuestTransition],
) {
    let Ok(mut cache) = queue.3.lock() else {
        return;
    };
    for receipt in committed {
        for snapshot in &mut cache.sessions {
            let revision = receipt.committed_character_revision.get();
            if snapshot.character != *receipt.character_id.as_bytes()
                || revision < snapshot.revision
            {
                continue;
            }
            for change in &receipt.changes {
                if let Some(value) = snapshot.phases.get_mut(&change.track) {
                    *value = change.after;
                }
            }
            snapshot.revision = revision;
        }
    }
}
