//! Grizzly death capture on the existing kill queue. The most-damage principal and
//! the task runs observed at death never change while a delayed settlement is retried.
//! Crystal 201189f `creaturescripts_monster_kill.lua:18-56`, `party.lua:129-139`.

use std::sync::Arc;

use oteryn_gameplay_quests::quest::grizzly_adams::{self as quest, GrizzlyAction};

use super::{
    ComposedFreshAdmission,
    kill_reward::{KillSettlementQueue, PendingKillSettlement, SettleReport, SettleVerdict},
};
use crate::durability::{
    character_progression::{CharacterProgressionError, ExperienceRewardOccurrence},
    quest_state::{QuestCause, QuestTransitionOutcome},
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, PartySnapshot};

/// A coherent quest snapshot is a capture predecessor, never write permission.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskSnapshot {
    actor: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease_generation: u64,
    observed_revision: u64,
    active_tasks: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Recipient {
    actor: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease_generation: u64,
    actions: Vec<GrizzlyAction>,
}

/// At most the native PartySnapshot bound (50) recipients, each with at most the
/// source active-task bound (3) plus one boss action. This belongs to the queued death.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct FrozenGrizzlyDeath {
    source: Option<PartySnapshot>,
    recipients: Vec<Recipient>,
}

/// Capture memory only; there is no independently drained queue here. Sessions are
/// bounded by the runtime's committed player census. Replayed corpse projections retain
/// their first capture, bounded by the carrier's existing pinned/unpinned corpse ceilings.
#[derive(Debug, Default)]
pub(super) struct GrizzlyCaptureCache {
    sessions: Vec<TaskSnapshot>,
    deaths: Vec<(ExactActorRef, [u8; 16], Arc<FrozenGrizzlyDeath>)>,
}

impl GrizzlyCaptureCache {
    fn remember(
        &mut self,
        target: ExactActorRef,
        occurrence: [u8; 16],
        captured: impl FnOnce(&[TaskSnapshot]) -> FrozenGrizzlyDeath,
    ) -> Arc<FrozenGrizzlyDeath> {
        if let Some((_, _, death)) = self.deaths.iter().find(|(_, id, _)| *id == occurrence) {
            return Arc::clone(death);
        }
        let frozen = Arc::new(captured(&self.sessions));
        self.deaths.push((target, occurrence, Arc::clone(&frozen)));
        frozen
    }
}

/// Select only the observed source roster. Cache absence is unavailable, never an
/// invented solo group. Mutation authority is qualified separately at settlement.
fn participants(source: &PartySnapshot, principal: [u8; 16]) -> Option<Vec<[u8; 16]>> {
    if source.character != principal || !source.is_well_formed() {
        return None;
    }
    Some(if source.party_id.is_some() && source.shared_xp_enabled {
        source
            .members
            .iter()
            .map(|member| member.character)
            .collect()
    } else {
        vec![principal]
    })
}

fn freeze(
    source: Option<PartySnapshot>,
    principal: [u8; 16],
    principal_present: bool,
    creature: &str,
    snapshots: &[TaskSnapshot],
) -> FrozenGrizzlyDeath {
    if !principal_present {
        return FrozenGrizzlyDeath::default();
    }
    let Some(source) = source else {
        return FrozenGrizzlyDeath::default();
    };
    let Some(members) = participants(&source, principal) else {
        return FrozenGrizzlyDeath::default();
    };
    let Ok(catalogue) = quest::catalogue() else {
        return FrozenGrizzlyDeath::default();
    };
    let boss = catalogue.bosses.iter().find(|boss| {
        boss.creature_key == creature || boss.creature_name.eq_ignore_ascii_case(creature)
    });
    let mut recipients = Vec::new();
    for character in members {
        let Some(snapshot) = snapshots
            .iter()
            .find(|snapshot| snapshot.character == character)
        else {
            continue;
        };
        if snapshot.observed_revision == 0 || snapshot.active_tasks.len() > quest::ACTIVE_TASK_LIMIT
        {
            continue;
        }
        let mut actions: Vec<_> = snapshot
            .active_tasks
            .iter()
            .filter_map(|id| {
                let task = quest::task(*id)?;
                task.creatures
                    .iter()
                    .any(|member| member.matches(creature))
                    .then(|| GrizzlyAction::KillCreature {
                        task_id: *id,
                        creature: creature.to_owned(),
                        observed_revision: snapshot.observed_revision,
                    })
            })
            .collect();
        // The donor boss event counts even before the player has joined the task.
        if let Some(boss) = boss {
            actions.push(GrizzlyAction::BossKill {
                boss_id: boss.id,
                observed_revision: snapshot.observed_revision,
            });
        }
        if !actions.is_empty() {
            recipients.push(Recipient {
                actor: snapshot.actor,
                session: snapshot.session,
                character,
                lease_generation: snapshot.lease_generation,
                actions,
            });
        }
    }
    FrozenGrizzlyDeath {
        source: Some(source),
        recipients,
    }
}

/// Membership in the pinned Grizzly catalogue permits capture only, never a reward
/// amount or a recipient. The native lethal facts and frozen live task runs still
/// decide eligibility, and the fenced quest writer remains the sole value owner.
pub(super) fn tracks_creature(creature: &str) -> bool {
    quest::catalogue().is_ok_and(|catalogue| {
        catalogue.bosses.iter().any(|boss| {
            boss.creature_key == creature || boss.creature_name.eq_ignore_ascii_case(creature)
        }) || catalogue
            .tasks
            .iter()
            .any(|task| task.creatures.iter().any(|member| member.matches(creature)))
    })
}

/// Runs synchronously under the lethal turn's runtime and attack guards. The existing
/// kill queue owns acceptance, replay, capacity, release handshakes and draining.
pub(super) fn capture(
    runtime: &mut ChannelRuntimeV1,
    queue: &KillSettlementQueue,
    entry: &mut PendingKillSettlement,
) {
    let Ok(mut cache) = queue.1.lock() else {
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
    if !runtime.retains_corpse_projection(entry.target) {
        return;
    }
    let principal = entry.facts.principal;
    // Gone and untracked contributors make the ordinary queue use a lethal-hit fallback.
    // Crystal functions.lua:1064-1072 requires a present most-damage player; neither
    // ordinary fallback supplies that Grizzly predecessor.
    let original_principal_present = runtime
        .borrow_combat_death()
        .top_damage_contributor(entry.target)
        .is_ok_and(|top| {
            top.present().is_some_and(|winner| {
                winner.character == principal.character
                    && winner.session == principal.session
                    && winner.lease_generation == principal.lease_generation
                    && winner.actor == principal.actor
            })
        });
    let principal_present = original_principal_present
        && runtime
            .bound_attacker_lease(principal.actor, principal.session)
            .is_ok_and(|lease| {
                lease.character_id() == principal.character
                    && lease.generation() == principal.lease_generation
            });
    let source = runtime
        .cached_world_party(principal.actor, principal.session)
        .ok()
        .flatten()
        .filter(|source| source.world == *runtime.binding().world_id().as_bytes())
        .cloned();
    entry.grizzly = Some(
        cache.remember(entry.target, entry.facts.occurrence, |snapshots| {
            let live: Vec<_> = snapshots
                .iter()
                .filter(|snapshot| {
                    runtime
                        .bound_attacker_lease(snapshot.actor, snapshot.session)
                        .is_ok_and(|lease| {
                            lease.character_id().as_bytes() == &snapshot.character
                                && lease.generation() == snapshot.lease_generation
                        })
                })
                .cloned()
                .collect();
            freeze(
                source,
                *principal.character.as_bytes(),
                principal_present,
                &entry.creature,
                &live,
            )
        }),
    );
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// A single installation follows every successful or failed quest copy refresh. No
    /// database await holds these guards; a task must be visible before a later death can
    /// capture it. The durable owner also rejects a task run restarted after this revision.
    pub(super) async fn refresh_grizzly_kill_snapshot(&self, session: GameSessionId) {
        let runtime = self.runtime.lock().await;
        let snapshot = {
            let sessions = self.states.quest_sessions.lock().ok();
            sessions
                .as_ref()
                .and_then(|sessions| sessions.get(&session))
                .and_then(|entry| entry.copy.as_ref())
                .and_then(|copy| {
                    let revision = copy.character_revision()?.get();
                    let active_tasks = quest::started_tasks(copy.tracks());
                    if active_tasks.len() > quest::ACTIVE_TASK_LIMIT {
                        return None;
                    }
                    let actor = runtime
                        .positioned_actor_census()
                        .ok()?
                        .into_iter()
                        .find(|(_, _, observed)| *observed == Some(session))?
                        .0;
                    let lease = runtime.bound_attacker_lease(actor, session).ok()?;
                    Some(TaskSnapshot {
                        actor,
                        session,
                        character: *lease.character_id().as_bytes(),
                        lease_generation: lease.generation(),
                        observed_revision: revision,
                        active_tasks,
                    })
                })
        };
        let attack = self.states.attack.lock().await;
        let Ok(mut cache) = attack.kills().1.lock() else {
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

    async fn credit_grizzly_death(&self, entry: &PendingKillSettlement) -> bool {
        let Some(frozen) = &entry.grizzly else {
            return true;
        };
        let (Some(source), Some(catalogue)) = (&frozen.source, &self.quest_catalogue) else {
            return true;
        };
        let Ok(occurrence) = ExperienceRewardOccurrence::from_bytes(entry.facts.occurrence) else {
            return true;
        };
        let cause = QuestCause::CreatureDeath(occurrence);
        for recipient in &frozen.recipients {
            let fence = match self.current_quest_fence(recipient.session).await {
                Ok(Some(fence)) => fence,
                Ok(None) => continue,
                Err(()) => return false,
            };
            if fence.character_id.as_bytes() != &recipient.character
                || fence.game_session_id != recipient.session
                || fence.character_lease_generation != recipient.lease_generation
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
                                && lease.generation() == recipient.lease_generation
                        })
                {
                    continue;
                }
            }
            // The writer independently checks the live generation fence, native party
            // id/revision and the task-start receipt under its existing SQL owners.
            let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
            let mut changed = false;
            for action in &recipient.actions {
                let outcome = slot
                    .commit_grizzly_kill(
                        self.root,
                        self.character,
                        self.holder,
                        fence,
                        cause,
                        action.clone(),
                        Arc::clone(catalogue),
                        source.clone(),
                    )
                    .await;
                match outcome {
                    Ok(
                        QuestTransitionOutcome::Committed(_)
                        | QuestTransitionOutcome::AlreadyCommitted(_),
                    ) => {
                        changed = true;
                    }
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
                    Err(error) => {
                        super::operator_event(&format!("grizzly_kill_refused reason={error:?}"))
                    }
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

/// Keep an unavailable quest write on the existing kill retry path. Grizzly settles
/// before the ordinary body so that its retry cannot follow an already released corpse
/// pin. The ordinary body's own retry reuses the same quest receipts without incrementing.
pub(super) async fn settle(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    entry: &PendingKillSettlement,
    ordinary: impl std::future::Future<Output = SettleReport>,
) -> SettleReport {
    if !admission.credit_grizzly_death(entry).await {
        return SettleReport {
            verdict: SettleVerdict::Retry,
            no_progression: false,
        };
    }
    ordinary.await
}

#[cfg(test)]
#[path = "grizzly_kills/tests.rs"]
mod tests;
