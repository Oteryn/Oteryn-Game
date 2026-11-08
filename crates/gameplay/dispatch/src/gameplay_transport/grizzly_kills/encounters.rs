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
}
