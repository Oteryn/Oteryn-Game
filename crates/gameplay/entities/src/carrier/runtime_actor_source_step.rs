//! A source-map STEP remains an actual movement-owner operation. A private
//! qualified movement proof is distinct from a spell's relocation authority.
use super::*;

pub mod source_step_seal {
    pub trait Sealed {}
}
pub trait SourceStepCommitProof: source_step_seal::Sealed {
    fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), CarrierError>;
    fn parts(
        &self,
    ) -> (
        ExactActorRef,
        MovementPositionSnapshot,
        MovementLocalPosition,
        MovementFacing,
    );
}
impl ChannelRuntimeV1 {
    /// Publish one sealed move together with a final projection CAS. Refusal
    /// restores the full native carrier before the exclusive owner turn ends.
    pub fn commit_source_step_and_project<P: SourceStepCommitProof>(
        &mut self,
        proof: P,
        project: impl FnOnce() -> bool,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        proof.validate_current(self)?;
        let before = self.carrier.clone();
        let result = (|| {
            let snapshot = self.commit_source_step(proof)?;
            if !project() {
                return Err(CarrierError::PlanConflict);
            }
            Ok(snapshot)
        })();
        if result.is_err() {
            self.carrier = before;
        }
        result
    }

    pub fn commit_source_step<P: SourceStepCommitProof>(
        &mut self,
        proof: P,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        proof.validate_current(self)?;
        let (actor, expected, next, facing) = proof.parts();
        if actor.0 != expected.0.actor_ref {
            return Err(
                oteryn_gameplay_domain::domain::carrier_values::CarrierError::PositionSnapshotMismatch,
            );
        }
        // Obtain the exact slot before committing. No fallible lookup follows
        // the physical mutation; the uninterrupted owner turn owns this slot.
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let mut committed = self.carrier.compare_commit_position(
            &self.continuity,
            expected.0,
            expected.0.version.context,
            LocalPosition {
                x: next.x,
                y: next.y,
                floor: next.floor,
            },
        )?;
        // Source floor changes preserve the actual input direction rather than
        // deriving a new facing from the final stair/hole coordinate offset.
        committed.version.facing = Some(facing);
        match &mut self.carrier.slots[index] {
            Slot::Occupied { position, .. } | Slot::CreatureOccupied { position, .. } => {
                *position = Some(committed.version);
            }
            _ => unreachable!("position commit proved occupied exact slot"),
        }
        Ok(MovementPositionSnapshot(committed))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
    use oteryn_gameplay_foundation::foundation::{ChannelId, NodeId, WorldId};

    fn id(tag: u8) -> [u8; 16] {
        [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
    }

    struct Proof {
        actor: ExactActorRef,
        expected: MovementPositionSnapshot,
        destination: MovementLocalPosition,
        guard: Option<(ExactActorRef, MovementPositionSnapshot)>,
    }
    impl source_step_seal::Sealed for Proof {}
    impl SourceStepCommitProof for Proof {
        fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), CarrierError> {
            runtime.owner_fence()?;
            if runtime.read_actor_position(self.actor)? != self.expected
                || self.guard.is_some_and(|(actor, expected)| {
                    runtime.read_actor_position(actor) != Ok(expected)
                })
            {
                return Err(CarrierError::PositionSnapshotMismatch);
            }
            Ok(())
        }
        fn parts(
            &self,
        ) -> (
            ExactActorRef,
            MovementPositionSnapshot,
            MovementLocalPosition,
            MovementFacing,
        ) {
            (
                self.actor,
                self.expected,
                self.destination,
                MovementFacing::South,
            )
        }
    }

    fn fixture() -> (ChannelRuntimeV1, [ExactActorRef; 2]) {
        let world = WorldId::decode(&id(1)).unwrap();
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).unwrap(),
            NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::test(world),
        )
        .unwrap();
        let actors = [4, 5].map(|tag| {
            let reserved = runtime
                .reserve_fresh_session(GameSessionId::decode(&id(tag)).unwrap())
                .unwrap();
            let actor = runtime.commit_fresh_session(reserved).unwrap();
            runtime
                .initialize_pinned_test_position(
                    actor,
                    MovementLocalPosition {
                        x: i32::from(tag),
                        y: 0,
                        floor: 0,
                    },
                )
                .unwrap();
            actor
        });
        (runtime, actors)
    }

    fn proofs(runtime: &ChannelRuntimeV1, actors: [ExactActorRef; 2]) -> Vec<Proof> {
        actors
            .into_iter()
            .enumerate()
            .map(|(i, actor)| Proof {
                actor,
                expected: runtime.read_actor_position(actor).unwrap(),
                destination: MovementLocalPosition {
                    x: 10 + i32::try_from(i).unwrap(),
                    y: 0,
                    floor: 0,
                },
                guard: None,
            })
            .collect()
    }

    #[test]
    fn source_step_projection_refusal_restores_the_entire_carrier() {
        let (mut runtime, actors) = fixture();
        let before = runtime.carrier.clone();
        let initial = runtime.read_actor_position(actors[0]).unwrap();
        let proof = proofs(&runtime, actors).remove(0);
        assert!(
            runtime
                .commit_source_step_and_project(proof, || false)
                .is_err()
        );
        assert_eq!(runtime.carrier, before);
        let proof = proofs(&runtime, actors).remove(0);
        let snapshot = runtime
            .commit_source_step_and_project(proof, || true)
            .unwrap();
        assert_eq!(snapshot.position().x, 10);
        assert_eq!(snapshot.revision(), initial.revision() + 1);
        let mut projected = false;
        assert_eq!(
            runtime.commit_source_step_and_project(
                Proof {
                    actor: actors[0],
                    expected: initial,
                    destination: snapshot.position(),
                    guard: None,
                },
                || {
                    projected = true;
                    true
                },
            ),
            Err(CarrierError::PositionSnapshotMismatch)
        );
        assert!(!projected);
        assert_eq!(runtime.read_actor_position(actors[0]).unwrap(), snapshot);
        assert_eq!(
            runtime.read_actor_position(actors[1]).unwrap().position().x,
            5
        );
    }

    #[test]
    fn source_group_projection_refusal_restores_every_member_and_retries_once() {
        let (mut runtime, actors) = fixture();
        let before = runtime.carrier.clone();
        let initial = actors.map(|actor| runtime.read_actor_position(actor).unwrap());
        assert_eq!(
            runtime.commit_source_steps_and_project(proofs(&runtime, actors), || false),
            Err(CarrierError::PlanConflict)
        );
        assert_eq!(runtime.carrier, before);
        let snapshots = runtime
            .commit_source_steps_and_project(proofs(&runtime, actors), || true)
            .unwrap();
        for ((actor, initial), committed) in actors.into_iter().zip(initial).zip(snapshots) {
            assert_eq!(committed.revision(), initial.revision() + 1);
            assert_eq!(runtime.read_actor_position(actor).unwrap(), committed);
        }
        let mut projected = false;
        let stale = Proof {
            actor: actors[0],
            expected: initial[0],
            destination: initial[0].position(),
            guard: None,
        };
        assert_eq!(
            runtime.commit_source_steps_and_project(vec![stale], || {
                projected = true;
                true
            }),
            Err(CarrierError::PositionSnapshotMismatch)
        );
        assert!(!projected);
        assert_eq!(
            runtime.commit_quest_encounter_entry(proofs(&runtime, actors), &[], || true),
            Err(CarrierError::PlanConflict)
        );
    }
}

/// One exact actor-position successor in a bounded source scene.
#[derive(Clone, Copy)]
pub struct SourceSceneMove {
    pub actor: ExactActorRef,
    pub expected: MovementPositionSnapshot,
    pub destination: MovementLocalPosition,
}

/// World owns eligibility, landing and the declared scene cells. The carrier
/// independently rechecks exact identity, current position and movement authority.
pub trait SourceSceneRelocationProof: source_step_seal::Sealed {
    fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), CarrierError>;
    fn moves(&self) -> &[SourceSceneMove];
}

impl ChannelRuntimeV1 {
    /// Commit at most32 source moves and one all-or-nothing scene projection.
    /// Existing movement and summon lifecycle checks remain authoritative. The
    /// private rollback snapshot is discarded before this owner turn is released.
    /// A rejected actor or projection leaves every carrier-owned store unchanged.
    pub fn commit_source_scene_relocations<P, F>(
        &mut self,
        proof: P,
        project: F,
    ) -> Result<Vec<MovementPositionSnapshot>, CarrierError>
    where
        P: SourceSceneRelocationProof,
        F: FnOnce() -> bool,
    {
        self.carrier.validate_current_continuity(&self.continuity)?;
        self.owner_fence()?;
        proof.validate_current(self)?;
        let moves = proof.moves();
        if moves.len() > 32
            || moves.iter().enumerate().any(|(index, change)| {
                moves[..index]
                    .iter()
                    .any(|before| before.actor == change.actor)
            })
        {
            return Err(CarrierError::PlanConflict);
        }
        // Same stores as compare_commit_position's summon-removal rollback.
        // No second position authority is published or retained.
        let before = (
            self.carrier.slots.clone(),
            self.carrier.free_head,
            self.carrier.occupied.clone(),
            self.carrier.native_summons.clone(),
            self.carrier.native_summon_deaths,
            self.carrier.native_summon_death_cursor,
            self.carrier.attackers.clone(),
        );
        let outcome = (|| {
            let mut result = Vec::with_capacity(moves.len());
            for change in moves {
                if change.actor.0 != change.expected.0.actor_ref {
                    return Err(CarrierError::PositionSnapshotMismatch);
                }
                let index = self
                    .carrier
                    .validate_ref(&self.continuity, change.actor.0)?;
                if self
                    .carrier
                    .read_movement_position(&self.continuity, change.actor.0)?
                    != change.expected
                {
                    return Err(CarrierError::PositionSnapshotMismatch);
                }
                if let Slot::NpcOccupied {
                    position,
                    hireling: None,
                    ..
                } = &mut self.carrier.slots[index]
                {
                    let revision = position
                        .revision
                        .checked_add(1)
                        .ok_or(CarrierError::PositionRevisionExhausted)?;
                    *position = VersionedPosition {
                        position: LocalPosition {
                            x: change.destination.x,
                            y: change.destination.y,
                            floor: change.destination.floor,
                        },
                        revision,
                        last_translation_revision: revision,
                        ..*position
                    };
                    result.push(MovementPositionSnapshot(PositionSnapshot {
                        actor_ref: change.actor.0,
                        version: *position,
                    }));
                    continue;
                }
                let mut committed = self.carrier.compare_commit_position(
                    &self.continuity,
                    change.expected.0,
                    change.expected.0.version.context,
                    LocalPosition {
                        x: change.destination.x,
                        y: change.destination.y,
                        floor: change.destination.floor,
                    },
                )?;
                // Forced source transport preserves the actual facing.
                committed.version.facing = change.expected.facing();
                match &mut self.carrier.slots[index] {
                    Slot::Occupied { position, .. } | Slot::CreatureOccupied { position, .. } => {
                        *position = Some(committed.version);
                    }
                    _ => return Err(CarrierError::PositionSnapshotMismatch),
                }
                result.push(MovementPositionSnapshot(committed));
            }
            if !project() {
                return Err(CarrierError::PlanConflict);
            }
            Ok(result)
        })();
        if outcome.is_err() {
            let (slots, free, occupied, summons, deaths, cursor, attackers) = before;
            self.carrier.slots = slots;
            self.carrier.free_head = free;
            self.carrier.occupied = occupied;
            self.carrier.native_summons = summons;
            self.carrier.native_summon_deaths = deaths;
            self.carrier.native_summon_death_cursor = cursor;
            self.carrier.attackers = attackers;
        }
        outcome
    }
}
