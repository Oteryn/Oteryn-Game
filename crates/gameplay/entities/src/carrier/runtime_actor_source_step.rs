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
    /// Publish a sealed relocation and its world projection in one owner turn.
    /// Movement can also retire separated summons, so a refused projection restores
    /// the complete carrier, including those actors and their ownership stores.
    pub fn commit_source_step_with_projection<P: SourceStepCommitProof>(
        &mut self,
        proof: P,
        project: impl FnOnce() -> bool,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        proof.validate_current(self)?;
        let before = self.carrier.clone();
        let result = self.commit_source_step(proof).and_then(|snapshot| {
            if project() {
                Ok(snapshot)
            } else {
                Err(CarrierError::PlanConflict)
            }
        });
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
    use super::*;
    use crate::carrier::tests::{runtime, session};

    // Only the carrier transaction is under test; production supplies a World proof.
    struct Proof(
        ExactActorRef,
        MovementPositionSnapshot,
        MovementLocalPosition,
    );
    impl source_step_seal::Sealed for Proof {}
    impl SourceStepCommitProof for Proof {
        fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), CarrierError> {
            if runtime.read_actor_position(self.0)? == self.1 {
                Ok(())
            } else {
                Err(CarrierError::PositionSnapshotMismatch)
            }
        }

        fn parts(
            &self,
        ) -> (
            ExactActorRef,
            MovementPositionSnapshot,
            MovementLocalPosition,
            MovementFacing,
        ) {
            (self.0, self.1, self.2, MovementFacing::South)
        }
    }

    #[test]
    fn source_step_projection_refusal_restores_position_and_retry_is_exact()
    -> Result<(), CarrierError> {
        let mut runtime = runtime(4);
        let reservation = runtime.reserve_fresh_session(session(170))?;
        let actor = runtime.commit_fresh_session(reservation)?;
        let initial = runtime.initialize_pinned_test_position(
            actor,
            MovementLocalPosition {
                x: 7,
                y: 0,
                floor: 7,
            },
        )?;
        let destination = MovementLocalPosition {
            x: 6,
            y: 0,
            floor: 7,
        };
        let before = runtime.carrier.slots.clone();
        assert_eq!(
            runtime
                .commit_source_step_with_projection(Proof(actor, initial, destination), || false),
            Err(CarrierError::PlanConflict)
        );
        assert_eq!(runtime.carrier.slots, before);
        assert_eq!(runtime.read_actor_position(actor)?, initial);
        let committed = runtime
            .commit_source_step_with_projection(Proof(actor, initial, destination), || true)?;
        assert_eq!(committed.position(), destination);
        assert_eq!(committed.revision(), initial.revision() + 1);
        let mut projected = false;
        assert_eq!(
            runtime.commit_source_step_with_projection(Proof(actor, initial, destination), || {
                projected = true;
                true
            }),
            Err(CarrierError::PositionSnapshotMismatch)
        );
        assert!(!projected);
        assert_eq!(runtime.read_actor_position(actor)?, committed);
        Ok(())
    }
}
