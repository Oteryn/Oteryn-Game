//! A source-map STEP remains an actual movement-owner operation. A private
//! qualified movement proof is distinct from a spell's relocation authority.
use super::*;

pub(crate) mod source_step_seal {
    pub(crate) trait Sealed {}
}
pub(crate) trait SourceStepCommitProof: source_step_seal::Sealed {
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
    pub(crate) fn commit_source_step<P: SourceStepCommitProof>(
        &mut self,
        proof: P,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        proof.validate_current(self)?;
        let (actor, expected, next, facing) = proof.parts();
        if actor.0 != expected.0.actor_ref {
            return Err(CarrierError::PositionSnapshotMismatch);
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
