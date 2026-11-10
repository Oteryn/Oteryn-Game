//! Grizzly death capture on the existing kill queue. The most-damage principal and
//! the task runs observed at death never change while a delayed settlement is retried.
//! Crystal 201189f `creaturescripts_monster_kill.lua:18-56`, `party.lua:129-139`.

use std::sync::Arc;
mod encounters;

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
    native_counters: Vec<NativeCounterCapture>,
    source_objectives: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeCounterCapture {
    transition: String,
    creature: String,
    definition_hash: Option<[u8; 32]>,
    death_area: Option<oteryn_gameplay_quests::quest::authored_bindings::kills::DeathArea>,
    encounter_source: Option<String>,
}

fn capture_native(
    catalogue: &oteryn_gameplay_quests::quest::QuestStateCatalogue,
    tracks: &std::collections::BTreeMap<String, i64>,
) -> Vec<NativeCounterCapture> {
    use oteryn_gameplay_quests::quest::native_kills;
    native_kills::active_for(catalogue, tracks)
        .into_iter()
        .filter_map(|key| {
            let counter = native_kills::counter_for(catalogue, &key)?;
            let definition_hash = if native_kills::counter(&key).is_some() {
                None
            } else {
                Some(catalogue.definition_hash(counter.quest)?)
            };
            Some(NativeCounterCapture {
                transition: key.clone(),
                creature: counter.creature.into(),
                definition_hash,
                death_area: counter.death_area,
                encounter_source: counter.encounter_source.map(str::to_owned),
            })
        })
        .collect()
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
    pub(super) native: Vec<super::quest_kills::Recipient>,
}

/// Capture memory only; there is no independently drained queue here. Sessions are
/// bounded by the runtime's committed player census. Replayed corpse projections retain
/// their first capture, bounded by the carrier's existing pinned/unpinned corpse ceilings.
#[derive(Debug, Default)]
pub(super) struct GrizzlyCaptureCache {
    sessions: Vec<TaskSnapshot>,
    deaths: Vec<(ExactActorRef, [u8; 16], Arc<FrozenGrizzlyDeath>)>,
    encounters: Vec<encounters::EncounterRun>,
    pub(super) hits: super::quest_hits::queue::HitQueue,
}

impl GrizzlyCaptureCache {
    fn remember(
        &mut self,
        target: ExactActorRef,
        occurrence: [u8; 16],
        captured: impl FnOnce(&[TaskSnapshot], &[encounters::EncounterRun]) -> FrozenGrizzlyDeath,
    ) -> Arc<FrozenGrizzlyDeath> {
        if let Some((_, _, death)) = self.deaths.iter().find(|(_, id, _)| *id == occurrence) {
            return Arc::clone(death);
        }
        let frozen = Arc::new(captured(&self.sessions, &self.encounters));
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

#[cfg(test)]
fn freeze(
    source: Option<PartySnapshot>,
    principal: [u8; 16],
    principal_present: bool,
    creature: &str,
    snapshots: &[TaskSnapshot],
) -> FrozenGrizzlyDeath {
    freeze_at(
        source,
        principal,
        principal_present,
        creature,
        snapshots,
        None,
    )
}

fn freeze_at(
    source: Option<PartySnapshot>,
    principal: [u8; 16],
    principal_present: bool,
    creature: &str,
    snapshots: &[TaskSnapshot],
    death_position: Option<[i32; 3]>,
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
    let mut native = Vec::new();
    let native_source = Arc::new(source.clone());
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
        for counter in &snapshot.native_counters {
            if counter.creature == creature
                && counter.encounter_source.is_none()
                && counter.death_area.is_none_or(|area| {
                    death_position.is_some_and(|position| area.contains(position))
                })
            {
                native.push(super::quest_kills::Recipient {
                    actor: snapshot.actor,
                    session: snapshot.session,
                    character,
                    lease_generation: snapshot.lease_generation,
                    death: crate::durability::quest_state::native_kills::NativeQuestDeath {
                        transition: counter.transition.clone(),
                        definition_hash: counter.definition_hash,
                        creature: creature.into(),
                        observed_revision: snapshot.observed_revision,
                        source:
                            crate::durability::quest_state::native_kills::NativeDeathSource::Party(
                                Arc::clone(&native_source),
                            ),
                    },
                });
            }
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
        native,
    }
}

/// Membership in the pinned Grizzly catalogue permits capture only, never a reward
/// amount or a recipient. The native lethal facts and frozen live task runs still
/// decide eligibility, and the fenced quest writer remains the sole value owner.
pub(super) fn tracks_native_creature(queue: &KillSettlementQueue, creature: &str) -> bool {
    oteryn_gameplay_quests::quest::native_kills::tracks_creature(creature)
        || queue.1.lock().is_ok_and(|cache| {
            cache.sessions.iter().any(|snapshot| {
                snapshot
                    .native_counters
                    .iter()
                    .any(|counter| counter.creature == creature)
            })
        })
}

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
    for run in &mut cache.encounters {
        let _ = run.roster.record_death(entry.facts.death);
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
        cache.remember(entry.target, entry.facts.occurrence, |snapshots, runs| {
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
            let mut frozen = freeze_at(
                source,
                *principal.character.as_bytes(),
                principal_present,
                &entry.creature,
                &live,
                Some([
                    entry.facts.corpse.x,
                    entry.facts.corpse.y,
                    i32::from(entry.facts.corpse.floor),
                ]),
            );
            if matches!(entry.facts.death.runtime_scope(), crate::foundation::RuntimeScopeRefV1::Instance { .. }) {
                for recipient in &mut frozen.native {
                    if recipient.death.transition != "oteryn:quest-transition/native-kills/first-dragon-selected" { continue; }
                    let crate::durability::quest_state::native_kills::NativeDeathSource::Party(party) = &recipient.death.source else { continue; };
                    recipient.death.source = crate::durability::quest_state::native_kills::NativeDeathSource::InstanceBoss {
                        party: Arc::clone(party), death: entry.facts.death,
                        died_at_unix_ms: entry.facts.died_at_unix_ms,
                    };
                }
            }
            let origin = runtime
                .read_actor_position(entry.target)
                .ok()
                .and_then(|facts| {
                    let at = facts.position();
                    Some((
                        u16::try_from(at.x).ok()?,
                        u16::try_from(at.y).ok()?,
                        i8::try_from(at.floor.checked_neg()?).ok()?,
                    ))
                });
            frozen.native.retain_mut(|recipient| {
                if !recipient
                    .death
                    .transition
                    .starts_with("oteryn:quest-transition/native-kills/m31/")
                {
                    return true;
                }
                if !oteryn_gameplay_quests::quest::world_change_journals::admits_position(
                    &recipient.death.transition,
                    origin,
                ) {
                    return false;
                }
                use crate::durability::quest_state::native_kills::NativeDeathSource;
                let (NativeDeathSource::Party(source), Some(position)) =
                    (&recipient.death.source, origin)
                else {
                    return false;
                };
                recipient.death.source = NativeDeathSource::PositionedParty {
                    source: Arc::clone(source),
                    position,
                };
                true
            });
            frozen.native.extend(super::quest_batch3_c::capture(
                runtime,
                entry,
                live.iter().map(|snapshot| super::quest_batch3_c::Snapshot {
                    actor: snapshot.actor,
                    session: snapshot.session,
                    character: snapshot.character,
                    lease_generation: snapshot.lease_generation,
                    observed_revision: snapshot.observed_revision,
                    objectives: snapshot.source_objectives.clone(),
                }),
            ));
                frozen.native.extend(encounters::capture(
                    runtime,
                    entry,
                    &live,
                    runs,
                    principal_present,
                ));
                frozen.native.extend(super::quest_batch3_c::capture(
                    runtime,
                    entry,
                    live.iter().map(|snapshot| super::quest_batch3_c::Snapshot {
                        actor: snapshot.actor,
                        session: snapshot.session,
                        character: snapshot.character,
                        lease_generation: snapshot.lease_generation,
                        observed_revision: snapshot.observed_revision,
                        objectives: snapshot.source_objectives.clone(),
                    }),
                ));
            frozen
        }),
    );
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// A single installation follows every successful or failed quest copy refresh. No
    /// database await holds these guards; a task must be visible before a later death can
    /// capture it. The durable owner also rejects a task run restarted after this revision.
    pub(super) async fn refresh_grizzly_kill_snapshot(&self, session: GameSessionId) {
        let runtime = self.runtime.lock().await;
        let (snapshot, hit_snapshot) = self.grizzly_kill_snapshots_in_turn(&runtime, session);
        let attack = self.states.attack.lock().await;
        let Ok(mut cache) = attack.kills().1.lock() else {
            return;
        };
        if !cache.hits.install(&runtime, session, hit_snapshot) {
            super::operator_event("quest_hit_snapshot_capacity_refused");
        }
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

    fn grizzly_kill_snapshots_in_turn(
        &self,
        runtime: &ChannelRuntimeV1,
        session: GameSessionId,
    ) -> (
        Option<TaskSnapshot>,
        Option<super::quest_hits::queue::Snapshot>,
    ) {
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
                    let hit_snapshot = self.quest_catalogue.as_ref().and_then(|catalogue| {
                        super::quest_hits::queue::Snapshot::new(
                            Arc::clone(catalogue),
                            actor,
                            session,
                            lease,
                            revision,
                            copy.tracks(),
                        )
                    });
                    Some((
                        TaskSnapshot {
                            actor,
                            session,
                            character: *lease.character_id().as_bytes(),
                            lease_generation: lease.generation(),
                            observed_revision: revision,
                            active_tasks,
                            native_counters: self
                                .quest_catalogue
                                .as_deref()
                                .map(|catalogue| capture_native(catalogue, copy.tracks()))
                                .unwrap_or_default(),
                            source_objectives: oteryn_gameplay_quests::quest::source_deaths::active(
                                copy.tracks(),
                            ),
                        },
                        hit_snapshot,
                    ))
                })
        };
        let (snapshot, hit_snapshot) = match snapshot {
            Some((snapshot, hit_snapshot)) => (Some(snapshot), hit_snapshot),
            None => (None, None),
        };
        (snapshot, hit_snapshot)
    }

    /// The quest callback already holds the Channel turn; never reacquire runtime here.
    pub(super) fn refresh_grizzly_kill_snapshot_in_turn(
        &self,
        runtime: &ChannelRuntimeV1,
        session: GameSessionId,
    ) -> bool {
        let (snapshot, hit_snapshot) = self.grizzly_kill_snapshots_in_turn(runtime, session);
        let Some(snapshot) = snapshot else {
            return false;
        };
        let Ok(attack) = self.states.attack.try_lock() else {
            return false;
        };
        let Ok(mut cache) = attack.kills().1.lock() else {
            return false;
        };
        if !cache.hits.install(runtime, session, hit_snapshot) {
            return false;
        }
        cache.sessions.retain(|old| {
            old.session != session && runtime.player_control_facts(old.actor, old.session).is_ok()
        });
        cache.sessions.push(snapshot);
        true
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
    if !super::quest_kills::settle(admission, entry).await
        || !admission.credit_grizzly_death(entry).await
    {
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

/// Quest eligibility frozen by the existing admitted encounter owner, before its
/// entry publishes. Social-party membership does not choose encounter recipients.
#[derive(Debug, Clone)]
pub(super) struct EncounterCredit {
    actor: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease_generation: u64,
    observed_revision: u64,
    counters: Vec<&'static str>,
    source: PartySnapshot,
}

pub(super) fn capture_encounter_admission(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    players: &[(ExactActorRef, GameSessionId)],
) -> Result<Vec<EncounterCredit>, crate::foundation::CarrierError> {
    use crate::foundation::CarrierError;
    owner
        .quest_catalogue
        .as_deref()
        .ok_or(CarrierError::PlanConflict)?;
    let sessions = owner
        .states
        .quest_sessions
        .lock()
        .map_err(|_| CarrierError::PlanConflict)?;
    let mut credits = Vec::new();
    credits
        .try_reserve(players.len())
        .map_err(|_| CarrierError::AllocationFailed)?;
    for &(actor, session) in players {
        let lease = runtime.bound_attacker_lease(actor, session)?;
        let copy = sessions
            .get(&session)
            .and_then(|s| s.copy.as_ref())
            .ok_or(CarrierError::PlanConflict)?;
        let revision = copy
            .character_revision()
            .ok_or(CarrierError::PlanConflict)?
            .get();
        let source = runtime
            .cached_world_party(actor, session)?
            .filter(|s| {
                s.character == *lease.character_id().as_bytes()
                    && s.world == *runtime.binding().world_id().as_bytes()
                    && s.is_well_formed()
            })
            .cloned()
            .ok_or(CarrierError::PlanConflict)?;
        credits.push(EncounterCredit {
            actor,
            session,
            character: *lease.character_id().as_bytes(),
            lease_generation: lease.generation(),
            observed_revision: revision,
            counters: oteryn_gameplay_quests::quest::native_kills::active_encounter(copy.tracks()),
            source,
        });
    }
    Ok(credits)
}

pub(super) fn freeze_encounter_credit(
    runtime: &ChannelRuntimeV1,
    states: &super::actor_spell::ChannelSpellStates,
    creature: &str,
    admitted: &[EncounterCredit],
    shares: &[crate::foundation::RewardDamageShare],
) -> Vec<super::quest_kills::Recipient> {
    let mut recipients = Vec::new();
    for participant in admitted {
        if states.is_dead(participant.actor)
            || !shares.iter().any(|share| {
                share.total > 0
                    && share.character.as_bytes() == &participant.character
                    && share.session == Some(participant.session)
                    && share.lease_generation == Some(participant.lease_generation)
            })
            || !runtime
                .bound_attacker_lease(participant.actor, participant.session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == &participant.character
                        && lease.generation() == participant.lease_generation
                })
        {
            continue;
        }
        // This is the participant's actual World-owned party snapshot. It neither
        // fabricates an encounter as a social party nor fans credit out to its members.
        let source = runtime
            .cached_world_party(participant.actor, participant.session)
            .ok()
            .flatten()
            .filter(|s| s.character == participant.character && s.is_well_formed())
            .unwrap_or(&participant.source);
        let source = Arc::new(source.clone());
        for counter in participant
            .counters
            .iter()
            .filter_map(|key| oteryn_gameplay_quests::quest::native_kills::counter(key))
            .filter(|c| c.creature == creature)
        {
            recipients.push(super::quest_kills::Recipient {
                actor: participant.actor,
                session: participant.session,
                character: participant.character,
                lease_generation: participant.lease_generation,
                death: crate::durability::quest_state::native_kills::NativeQuestDeath {
                    transition: counter.transition.into(),
                    definition_hash: None,
                    creature: creature.into(),
                    observed_revision: participant.observed_revision,
                    source: crate::durability::quest_state::native_kills::NativeDeathSource::Party(
                        Arc::clone(&source),
                    ),
                },
            });
        }
    }
    recipients
}

pub(super) fn replace_encounter_native_credit(
    entry: &mut PendingKillSettlement,
    recipients: Vec<super::quest_kills::Recipient>,
) {
    let frozen = entry
        .grizzly
        .get_or_insert_with(|| Arc::new(FrozenGrizzlyDeath::default()));
    Arc::make_mut(frozen).native = recipients;
}

#[cfg(test)]
mod encounter_credit_tests {
    use super::*;
    use crate::foundation::*;
    use oteryn_gameplay_entities::carrier::{ActorState, LocalPosition};

    fn id(tag: u8) -> [u8; 16] {
        [1, 0, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
    }

    #[test]
    fn admitted_damage_dealers_get_credit_without_last_hit_and_outside_principal_does_not()
    -> Result<(), CarrierError> {
        let world = WorldId::decode(&id(1)).map_err(|_| CarrierError::PlanConflict)?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).map_err(|_| CarrierError::PlanConflict)?,
            NodeId::decode(&id(3)).map_err(|_| CarrierError::PlanConflict)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::test(world),
        )?;
        let mut credits = Vec::new();
        for tag in 4..=6 {
            let session =
                GameSessionId::decode(&id(tag)).map_err(|_| CarrierError::PlanConflict)?;
            let character =
                CharacterId::decode(&id(tag + 10)).map_err(|_| CarrierError::PlanConflict)?;
            let lease = crate::foundation::CharacterLease::new(character, 1)
                .map_err(|_| CarrierError::PlanConflict)?;
            let (carrier, continuity) = runtime.encounter_parts_mut();
            let actor = carrier.admit_bound_test_attacker(continuity, session, lease)?;
            runtime.initialize_pinned_test_position(
                actor,
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
            )?;
            let source = PartySnapshot {
                world: *world.as_bytes(),
                character: *character.as_bytes(),
                party_id: None,
                revision: None,
                leader: None,
                shared_xp_enabled: false,
                members: Vec::new(),
                observed_database_micros: 1,
            };
            runtime.project_world_party(actor, session, source.clone())?;
            credits.push(EncounterCredit {
                actor,
                session,
                character: *character.as_bytes(),
                lease_generation: 1,
                observed_revision: 1,
                source,
                counters: vec!["oteryn:quest-transition/native-kills/cults-essence"],
            });
        }
        let shares: Vec<_> = credits
            .iter()
            .enumerate()
            .map(|(i, p)| {
                Ok(crate::foundation::RewardDamageShare {
                    character: CharacterId::decode(&p.character)
                        .map_err(|_| CarrierError::PlanConflict)?,
                    total: (i as u64 + 1) * 10,
                    lease_generation: Some(p.lease_generation),
                    session: Some(p.session),
                    last_damage_at_ms: Some(1),
                })
            })
            .collect::<Result<Vec<_>, CarrierError>>()?;
        let target = ExactActorRef(runtime.admit_encounter_fixture_actor(
            ActorState(0),
            "oteryn:creature.essence_of_malice",
            100,
            Some(LocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            }),
        )?);
        let outsider = &credits[2];
        runtime
            .borrow_exact_actor_commit()
            .commit_damage_for_bound_attacker(
                target,
                outsider.actor,
                CommandRef::new(
                    outsider.session,
                    CommandId::new(1).map_err(|_| CarrierError::PlanConflict)?,
                ),
                0,
                OwnerDamageCommand {
                    target: b"oteryn:creature.essence_of_malice",
                    occurrence: b"phase",
                    binding: b"phase",
                    damage: 100,
                },
            )?;
        let mut death = runtime.borrow_combat_death();
        let lethal = death.committed_lethal_receipt(target)?;
        death.project_committed_lethal(lethal)?;
        let states = super::super::actor_spell::ChannelSpellStates::default();
        let result = freeze_encounter_credit(
            &runtime,
            &states,
            "oteryn:creature.essence_of_malice",
            &credits[..2],
            &shares,
        );
        assert_eq!(result.len(), 2);
        assert!(
            result
                .iter()
                .all(|recipient| recipient.character != outsider.character)
        );
        runtime.remove_test_actor(credits[0].actor)?;
        let result = freeze_encounter_credit(
            &runtime,
            &states,
            "oteryn:creature.essence_of_malice",
            &credits[..2],
            &shares,
        );
        assert_eq!(
            result.len(),
            1,
            "a retired admitted actor cannot credit its replacement"
        );
        Ok(())
    }
}
