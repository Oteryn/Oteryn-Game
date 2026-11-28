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
use oteryn_gameplay_quests::quest::{death_objectives, kill_objectives as rules};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct Snapshot {
    actor: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease: u64,
    revision: u64,
    phases: std::collections::BTreeMap<String, i64>,
    grave_objectives: Vec<&'static str>,
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
    if let Some(rule) = oteryn_gameplay_quests::quest::source_deaths::overlord(&entry.creature) {
        let Ok(census) = runtime.positioned_actor_census() else {
            return;
        };
        // Eligibility is physical damage plus the bound lease, never a quest-cache read.
        for (actor, _, session) in census {
            let Some(session) = session else {
                continue;
            };
            let Ok(lease) = runtime.bound_attacker_lease(actor, session) else {
                continue;
            };
            let snapshot = Snapshot {
                actor,
                session,
                character: *lease.character_id().as_bytes(),
                lease: lease.generation(),
                revision: 0,
                phases: Default::default(),
                grave_objectives: Vec::new(),
            };
            if let Some(source) =
                NativeObjectiveDeath::capture_overlord(runtime, entry.target, actor, session)
            {
                captured.push(CapturedObjective {
                    snapshot,
                    transition: &rule.transition,
                    source,
                });
            }
        }
    }
    capture_grave_objectives(runtime, entry, &cache.sessions, &mut captured);
    let captured = Arc::new(captured);
    cache
        .deaths
        .push((entry.target, entry.facts.occurrence, Arc::clone(&captured)));
    entry.objectives = Some(captured);
}

/// The donor dispatches Grave Danger from the living encounter's physical
/// damage map. Session, stage and boss-room eligibility are frozen at death.
fn capture_grave_objectives(
    runtime: &mut ChannelRuntimeV1,
    entry: &PendingKillSettlement,
    sessions: &[Snapshot],
    captured: &mut Vec<CapturedObjective>,
) {
    let at = entry.facts.corpse;
    let Ok(objectives) = death_objectives::load() else {
        return;
    };
    let Some(rule) = objectives.iter().find(|rule| {
        death_objectives::is_grave_boss(&rule.key)
            && rule.matches(&entry.creature, at.x, at.y, at.floor)
    }) else {
        return;
    };
    for snapshot in sessions {
        // The Grave Danger knight hook owns contributor credit. Scarlett
        // keeps her separate Cobra objective's principal eligibility.
        if rule.creature == "oteryn:creature.scarlett_etzel"
            && snapshot.actor != entry.facts.principal.actor
        {
            continue;
        }
        if snapshot.grave_objectives.contains(&rule.key.as_str())
            && let Some(source) = NativeObjectiveDeath::capture_grave_boss(
                runtime,
                entry.target,
                snapshot.actor,
                snapshot.session,
                snapshot.revision,
            )
        {
            captured.push(CapturedObjective {
                snapshot: snapshot.clone(),
                transition: &rule.key,
                source,
            });
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn refresh_native_objective_snapshot(&self, session: GameSessionId) {
        let (runtime_guard, attack) = self.quest_turn_with(&self.states.attack).await;
        let runtime = &*runtime_guard;

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
                grave_objectives: death_objectives::load()
                    .ok()
                    .into_iter()
                    .flatten()
                    .filter(|rule| {
                        death_objectives::is_grave_boss(&rule.key) && rule.active(&tracks)
                    })
                    .map(|rule| rule.key.as_str())
                    .collect(),
            })
        });
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

/// Additional quest-only recipients do not become XP or loot members.
pub(super) fn boss_objective_recipients(
    entry: &PendingKillSettlement,
) -> Vec<(crate::domain::CharacterId, GameSessionId, u64)> {
    entry
        .objectives
        .iter()
        .flat_map(|items| items.iter())
        .filter(|item| is_boss_objective(entry, item))
        .filter_map(|item| {
            crate::domain::CharacterId::from_bytes(item.snapshot.character)
                .ok()
                .map(|character| (character, item.snapshot.session, item.snapshot.lease))
        })
        .collect()
}

fn is_boss_objective(entry: &PendingKillSettlement, item: &CapturedObjective) -> bool {
    death_objectives::is_grave_boss(item.transition)
        || oteryn_gameplay_quests::quest::source_deaths::overlord(&entry.creature).is_some()
}

pub(super) fn has_grave_boss_credit(entry: &PendingKillSettlement) -> bool {
    entry.objectives.as_ref().is_some_and(|items| {
        items
            .iter()
            .any(|item| death_objectives::is_grave_boss(item.transition))
    })
}

/// The XP owner holds all captured Character slots and the native Channel turn.
/// Ordinary objectives require the original principal. Boss objectives prove physical damage.
pub(super) async fn apply_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
    entry: &PendingKillSettlement,
    participants: &[(
        crate::durability::character_progression::CurrentCharacterGameplayFence,
        bool,
    )],
    quest_fences: &[crate::durability::character_progression::CurrentCharacterGameplayFence],
    catalogue: Option<&oteryn_gameplay_quests::quest::QuestStateCatalogue>,
) -> Result<Vec<crate::durability::quest_state::CommittedQuestTransition>, CharacterProgressionError>
{
    if oteryn_gameplay_quests::quest::source_deaths::overlord(&entry.creature).is_some()
        || has_grave_boss_credit(entry)
    {
        let captured = entry
            .objectives
            .as_ref()
            .ok_or(CharacterProgressionError::AuthorityRejected)?;
        let catalogue = catalogue.ok_or(CharacterProgressionError::InvalidInput)?;
        let mut committed = Vec::new();
        for objective in captured
            .iter()
            .filter(|item| is_boss_objective(entry, item))
        {
            let snapshot = &objective.snapshot;
            let Some(fence) = quest_fences.iter().find(|fence| {
                fence.character_id.as_bytes() == &snapshot.character
                    && fence.game_session_id == snapshot.session
                    && fence.character_lease_generation == snapshot.lease
            }) else {
                continue;
            };
            if let Some(receipt) =
                crate::durability::quest_state::kill_objectives::apply_batch_in_transaction(
                    tx,
                    node,
                    *fence,
                    &[(objective.transition, &objective.source)],
                    catalogue,
                )
                .await?
            {
                committed.push(receipt);
            }
        }
        return Ok(committed);
    }
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
