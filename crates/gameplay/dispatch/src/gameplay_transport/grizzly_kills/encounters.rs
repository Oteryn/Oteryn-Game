//! Native room admissions share the existing death capture owner and its lock order.
use super::*;
use oteryn_gameplay_entities::carrier::encounter_access::EncounterDefenders;

const MAX_RUNS: usize = 64;

#[derive(Debug)]
pub(super) struct Participant {
    pub(super) actor: ExactActorRef,
    pub(super) session: GameSessionId,
    pub(super) character: [u8; 16],
    pub(super) lease_generation: u64,
    pub(super) admission_revision: u64,
}
#[derive(Debug)]
pub(super) struct EncounterRun {
    pub(super) source: String,
    pub(super) definition_hash: [u8; 32],
    pub(super) participants: Vec<Participant>,
    pub(super) roster: EncounterDefenders,
}

pub(in crate::gameplay_transport) struct PreparedEncounter {
    source: String,
    definition_hash: [u8; 32],
    participants: Vec<Participant>,
}
impl GrizzlyCaptureCache {
    /// Every allocation, lease and active source check precedes physical publication.
    pub(in crate::gameplay_transport) fn prepare_encounter(
        &mut self,
        runtime: &ChannelRuntimeV1,
        source: &str,
        definition_hash: [u8; 32],
        selected: &[(ExactActorRef, GameSessionId)],
    ) -> Result<PreparedEncounter, ()> {
        self.encounters.retain(|run| {
            run.participants.iter().any(|participant| {
                runtime
                    .bound_attacker_lease(participant.actor, participant.session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == &participant.character
                            && lease.generation() == participant.lease_generation
                    })
            }) && run.roster.defenders().iter().any(|actor| {
                runtime.contains_live_creature(*actor) || runtime.retains_corpse_projection(*actor)
            })
        });
        if self.encounters.len() >= MAX_RUNS
            || !(1..=8).contains(&selected.len())
            || self.encounters.iter().any(|run| {
                run.source == source
                    && run
                        .roster
                        .defenders()
                        .iter()
                        .any(|actor| runtime.contains_live_creature(*actor))
            })
        {
            return Err(());
        }
        self.encounters.try_reserve(1).map_err(|_| ())?;
        let mut participants = Vec::new();
        participants.try_reserve(selected.len()).map_err(|_| ())?;
        for &(actor, session) in selected {
            let lease = runtime
                .bound_attacker_lease(actor, session)
                .map_err(|_| ())?;
            let snapshot = self
                .sessions
                .iter()
                .find(|snapshot| {
                    snapshot.actor == actor
                        && snapshot.session == session
                        && snapshot.character == *lease.character_id().as_bytes()
                        && snapshot.lease_generation == lease.generation()
                        && snapshot.observed_revision > 0
                        && snapshot.native_counters.iter().any(|counter| {
                            counter.encounter_source.as_deref() == Some(source)
                                && counter.definition_hash == Some(definition_hash)
                        })
                })
                .ok_or(())?;
            participants.push(Participant {
                actor,
                session,
                character: snapshot.character,
                lease_generation: snapshot.lease_generation,
                admission_revision: snapshot.observed_revision,
            });
        }
        Ok(PreparedEncounter {
            source: source.into(),
            definition_hash,
            participants,
        })
    }
    /// Capacity was reserved by prepare_encounter under this same cache guard.
    pub(in crate::gameplay_transport) fn publish_encounter(
        &mut self,
        prepared: PreparedEncounter,
        roster: EncounterDefenders,
    ) {
        self.encounters.push(EncounterRun {
            source: prepared.source,
            definition_hash: prepared.definition_hash,
            participants: prepared.participants,
            roster,
        });
    }
}

impl GrizzlyCaptureCache {
    /// A retained unfinished admission vetoes its door even if an earlier run left durable credit.
    /// An absent admission lets the durable goal owner handle reconnect recovery.
    pub(in crate::gameplay_transport) fn encounter_door_allowed(
        &self,
        source: &str,
        hash: [u8; 32],
        actor: ExactActorRef,
        session: GameSessionId,
        lease_generation: u64,
    ) -> bool {
        self.encounters
            .iter()
            .filter(|run| run.source == source)
            .all(|run| {
                let Some(participant) = run.participants.iter().find(|participant| {
                    participant.actor == actor && participant.session == session
                }) else {
                    return true;
                };
                run.definition_hash == hash
                    && participant.admission_revision > 0
                    && participant.lease_generation == lease_generation
                    && run.roster.complete_for(actor)
            })
    }
}

/// Freeze only the exact native source run; social-party membership cannot add recipients.
pub(super) fn capture(
    runtime: &ChannelRuntimeV1,
    entry: &PendingKillSettlement,
    snapshots: &[TaskSnapshot],
    runs: &[EncounterRun],
    principal_present: bool,
) -> Vec<crate::gameplay_transport::quest_kills::Recipient> {
    use crate::durability::quest_state::{
        encounter_deaths::EncounterDeathContext,
        native_kills::{NativeDeathSource, NativeQuestDeath},
    };
    if !principal_present {
        return Vec::new();
    }
    let principal = entry.facts.principal;
    let (scope, generation) = runtime.encounter_owner_scope();
    let mut recipients = Vec::new();
    for run in runs {
        if !run.roster.defenders().contains(&entry.target)
            || !run.participants.iter().any(|participant| {
                participant.actor == principal.actor
                    && participant.session == principal.session
                    && participant.character == *principal.character.as_bytes()
                    && participant.lease_generation == principal.lease_generation
            })
        {
            continue;
        }
        for participant in &run.participants {
            let Some(snapshot) = snapshots.iter().find(|snapshot| {
                snapshot.actor == participant.actor
                    && snapshot.session == participant.session
                    && snapshot.character == participant.character
                    && snapshot.lease_generation == participant.lease_generation
                    && snapshot.observed_revision >= participant.admission_revision
            }) else {
                continue;
            };
            for counter in &snapshot.native_counters {
                if counter.creature != entry.creature
                    || counter.encounter_source.as_deref() != Some(run.source.as_str())
                    || counter.definition_hash != Some(run.definition_hash)
                {
                    continue;
                }
                recipients.push(crate::gameplay_transport::quest_kills::Recipient {
                    actor: participant.actor,
                    session: participant.session,
                    character: participant.character,
                    lease_generation: participant.lease_generation,
                    death: NativeQuestDeath {
                        transition: counter.transition.clone(),
                        creature: entry.creature.clone(),
                        observed_revision: participant.admission_revision,
                        definition_hash: Some(run.definition_hash),
                        source: NativeDeathSource::Encounter(EncounterDeathContext {
                            source: run.source.clone(),
                            scope,
                            scope_generation: generation.get(),
                            character: participant.character,
                            session: participant.session,
                            lease_generation: participant.lease_generation,
                            participant: participant.actor.placement_identity(),
                            defender: entry.target.corpse_identity(),
                        }),
                    },
                });
            }
        }
    }
    recipients
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{ChannelId, CombatDeathFixture, ScopeOwnershipGeneration, WorldId};

    #[test]
    fn engenc_room_door_requires_its_exact_committed_defenders() -> Result<(), String> {
        fn checked(error: impl std::fmt::Debug) -> String {
            format!("{error:?}")
        }
        let id = |seed| {
            [
                seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
            ]
        };
        let mut fixture = CombatDeathFixture::new(
            WorldId::decode(&id(1)).map_err(checked)?,
            ChannelId::decode(&id(2)).map_err(checked)?,
            ScopeOwnershipGeneration::new(1).map_err(checked)?,
        )
        .map_err(checked)?;
        let target = fixture.actor();
        let mut participant = target;
        participant.0.actor_local_id.0 += 1;
        let session = GameSessionId::decode(&id(3)).map_err(checked)?;
        let hash = [4; 32];
        let mut cache = GrizzlyCaptureCache::default();
        cache.encounters.push(EncounterRun {
            source: "source-room".into(),
            definition_hash: hash,
            participants: vec![Participant {
                actor: participant,
                session,
                character: id(5),
                lease_generation: 2,
                admission_revision: 10,
            }],
            roster: EncounterDefenders::new(&[participant], &[target]).map_err(checked)?,
        });
        assert!(!cache.encounter_door_allowed("source-room", hash, participant, session, 2));
        assert!(cache.encounter_door_allowed("other-room", hash, participant, session, 2));
        fixture
            .strike("engenc-room-defender", CombatDeathFixture::HEALTH)
            .map_err(checked)?;
        let (death, _) = fixture.project_death().map_err(checked)?;
        assert_eq!(cache.encounters[0].roster.record_death(death), Some(0));
        assert!(cache.encounter_door_allowed("source-room", hash, participant, session, 2));
        assert!(!cache.encounter_door_allowed("source-room", [6; 32], participant, session, 2));
        assert!(!cache.encounter_door_allowed("source-room", hash, participant, session, 3));
        assert_eq!(cache.encounters[0].roster.record_death(death), None);
        // Any unfinished replay/admission vetoes; an old completed run cannot bless it.
        cache.encounters.push(EncounterRun {
            source: "source-room".into(),
            definition_hash: hash,
            participants: vec![Participant {
                actor: participant,
                session,
                character: id(5),
                lease_generation: 2,
                admission_revision: 11,
            }],
            roster: EncounterDefenders::new(&[participant], &[target]).map_err(checked)?,
        });
        assert!(!cache.encounter_door_allowed("source-room", hash, participant, session, 2));
        Ok(())
    }

    /// Asura and the other native rooms credit a defender only when the most-damage
    /// principal is an exact participant of the run that owns it (TibiaWiki
    /// Asura_Palace_Quest/Spoiler: each kill is done individually).
    #[test]
    fn asuraelf_encounter_kill_credits_only_an_admitted_participant() -> Result<(), String> {
        fn checked(error: impl std::fmt::Debug) -> String {
            format!("{error:?}")
        }
        let id = |seed| {
            [
                seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
            ]
        };
        let entry = crate::gameplay_transport::kill_reward::tests::pending(2, 50, 3, 0);
        let principal = entry.facts.principal;
        let world = WorldId::decode(&id(60)).map_err(checked)?;
        let runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(61)).map_err(checked)?,
            crate::foundation::NodeId::decode(&id(62)).map_err(checked)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            3,
            oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::test(world),
        )
        .map_err(checked)?;
        let hash = [4; 32];
        let participant = |actor, session, lease_generation| Participant {
            actor,
            session,
            character: *principal.character.as_bytes(),
            lease_generation,
            admission_revision: 10,
        };
        let snapshot = |p: &Participant| TaskSnapshot {
            actor: p.actor,
            session: p.session,
            character: p.character,
            lease_generation: p.lease_generation,
            observed_revision: 10,
            active_tasks: Vec::new(),
            native_counters: vec![NativeCounterCapture {
                transition: "oteryn:quest-transition/native-kills/engenc-asura-s5".into(),
                creature: entry.creature.clone(),
                definition_hash: Some(hash),
                death_area: None,
                encounter_source: Some("source-room".into()),
            }],
            source_objectives: Vec::new(),
            active_goals: Vec::new(),
            objective_keys: Vec::new(),
        };
        let run = |p: Participant| -> Result<EncounterRun, String> {
            Ok(EncounterRun {
                source: "source-room".into(),
                definition_hash: hash,
                roster: EncounterDefenders::new(&[p.actor], &[entry.target]).map_err(checked)?,
                participants: vec![p],
            })
        };
        let admitted = participant(
            principal.actor,
            principal.session,
            principal.lease_generation,
        );
        let snapshots = [snapshot(&admitted)];
        let credited = capture(&runtime, &entry, &snapshots, &[run(admitted)?], true);
        assert_eq!(credited.len(), 1);
        assert_eq!(credited[0].session, principal.session);
        let admitted = participant(
            principal.actor,
            principal.session,
            principal.lease_generation,
        );
        assert!(capture(&runtime, &entry, &snapshots, &[run(admitted)?], false).is_empty());

        // The most-damage principal is not in the run that owns the defender: nobody,
        // including the run's own participant, is credited.
        let mut other = principal.actor;
        other.0.actor_local_id.0 += 1;
        let session = GameSessionId::decode(&id(3)).map_err(checked)?;
        let outsider = participant(other, session, principal.lease_generation);
        let snapshots = [
            snapshot(&participant(
                principal.actor,
                principal.session,
                principal.lease_generation,
            )),
            snapshot(&outsider),
        ];
        assert!(capture(&runtime, &entry, &snapshots, &[run(outsider)?], true).is_empty());

        // A stale lease of the same character is not the admitted participant either.
        let stale = participant(
            principal.actor,
            principal.session,
            principal.lease_generation + 1,
        );
        let snapshots = [snapshot(&stale)];
        assert!(capture(&runtime, &entry, &snapshots, &[run(stale)?], true).is_empty());
        Ok(())
    }
}
