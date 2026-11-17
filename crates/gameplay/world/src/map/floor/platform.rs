//! A rider lands on the platform's prepared future map, before either owner commits.
use super::*;
use crate::map::{boot::BundleMap, facts::QuestMapItemMove};

struct PlatformRider<'a> {
    cells: &'a NativeEntryMovementCells,
    map: &'a BundleMap,
    batch: &'a [QuestMapItemMove],
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
}

/// Qualify the exact rider against the platform batch's future landing and current revisions.
#[allow(
    clippy::too_many_arguments,
    reason = "the sealed proof binds both owners, the exact rider and the future map batch"
)]
pub fn prepare_quest_platform_rider<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    map: &'a BundleMap,
    batch: &'a [QuestMapItemMove],
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    pz_blocked: bool,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    let first = batch.first().ok_or(MovementError::NotQualified)?;
    let proof = PlatformRider {
        cells,
        map,
        batch,
        actor,
        session,
        expected,
        destination: MovementLocalPosition {
            x: i32::from(first.to.x),
            y: i32::from(first.to.y),
            floor: -i16::from(first.to.floor),
        },
        pz_blocked,
    };
    proof.check(runtime)?;
    Ok(proof)
}

impl PlatformRider<'_> {
    fn check(&self, runtime: &ChannelRuntimeV1) -> Result<(), MovementError> {
        qualify_origin(
            runtime,
            self.cells,
            self.actor,
            self.session,
            self.expected,
            None,
        )?;
        let NativeMovementCollisionIndex::Bundle(collision) = self.cells.index() else {
            return Err(MovementError::NotQualified);
        };
        let first = self.batch.first().ok_or(MovementError::NotQualified)?;
        let origin = self.expected.position();
        let to = self.destination;
        if !std::ptr::eq(collision.map(), self.map)
            || pos(origin.x, origin.y, i32::from(origin.floor)) != Some(first.from)
            || first.from == first.to
            || first.entry.facts.appearance_id != 17938
            || first.appearance != 17938
            || !self.map.facts.quest_platform_batch_landing(
                self.map.overlay.base(),
                self.batch,
                first.to,
            )
            || (self.pz_blocked
                && !self
                    .cells
                    .spell_tiles()
                    .lookup(
                        self.cells.scope(),
                        LogicalCell {
                            x: to.x,
                            y: to.y,
                            z: i32::from(to.floor),
                        },
                    )
                    .is_ok_and(|tile| !tile.flags().protection_zone))
            || runtime
                .position_occupied_by_other(self.actor, to)
                .map_err(MovementError::Actor)?
        {
            return Err(MovementError::Blocked);
        }
        Ok(())
    }
}

impl crate::foundation::source_step_seal::Sealed for PlatformRider<'_> {}
impl crate::foundation::SourceStepCommitProof for PlatformRider<'_> {
    fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), CarrierError> {
        self.check(runtime).map_err(|_| CarrierError::PlanConflict)
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
            self.expected.facing().unwrap_or(MovementFacing::South),
        )
    }
}
