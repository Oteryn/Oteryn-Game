//! Source door closing pushes one occupant E, S, N, W before the door CAS.
use super::*;

struct DoorDisplacement<'a> {
    cells: &'a NativeEntryMovementCells,
    doors: &'a super::super::door::DoorBook,
    key: u64,
    revision: u64,
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
}

#[allow(
    clippy::too_many_arguments,
    reason = "the sealed displacement binds native actor, selected open door and collision generation"
)]
pub fn prepare_quest_door_displacement<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    doors: &'a super::super::door::DoorBook,
    key: u64,
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
    pz_blocked: bool,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    let at = expected.position();
    for (dx, dy) in [(1, 0), (0, 1), (0, -1), (-1, 0)] {
        let (Some(x), Some(y)) = (at.x.checked_add(dx), at.y.checked_add(dy)) else {
            continue;
        };
        let proof = DoorDisplacement {
            cells,
            doors,
            key,
            revision: doors.revision(key),
            actor,
            expected,
            destination: MovementLocalPosition {
                x,
                y,
                floor: at.floor,
            },
            pz_blocked,
        };
        if proof.check(runtime).is_ok() {
            return Ok(proof);
        }
    }
    Err(MovementError::Blocked)
}

impl DoorDisplacement<'_> {
    fn check(&self, runtime: &ChannelRuntimeV1) -> Result<(), MovementError> {
        runtime.owner_fence().map_err(MovementError::Actor)?;
        let (position, _) =
            super::super::view::placement_of(self.key).ok_or(MovementError::NotQualified)?;
        let source = self.expected.position();
        if !self.doors.contains(self.key)
            || self.doors.blocks(position)
            || self.doors.revision(self.key) != self.revision
            || source.x != i32::from(position.x)
            || source.y != i32::from(position.y)
            || source.floor != -i16::from(position.floor)
            || runtime.actor_spell_reserved(self.actor)
            || self.cells.scope().world_id != runtime.binding().world_id()
            || self.cells.scope().generation_digest
                != runtime.content_pin().server_artifact_digest()
            || self.expected.context() != runtime.pinned_movement_context()
            || runtime
                .read_actor_position(self.actor)
                .map_err(MovementError::Actor)?
                != self.expected
        {
            return Err(MovementError::NotQualified);
        }
        let to = self.destination;
        let cell = (to.x, to.y, i32::from(to.floor));
        // Canary's queryAdd uses FLAG_IGNOREBLOCKCREATURE. Terrain, other
        // doors and the native protection-zone policy still qualify the tile.
        if ground_of(self.cells)(cell) != Ground::Walkable
            || !landable_of(self.cells, self.pz_blocked, &BundleDoors(self.doors))(cell)
        {
            return Err(MovementError::Blocked);
        }
        Ok(())
    }
}
impl crate::foundation::source_step_seal::Sealed for DoorDisplacement<'_> {}
impl crate::foundation::SourceStepCommitProof for DoorDisplacement<'_> {
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

struct DoorOpening<'a> {
    cells: &'a NativeEntryMovementCells,
    map: &'a crate::map::boot::BundleMap,
    key: u64,
    revision: u64,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
}

#[allow(
    clippy::too_many_arguments,
    reason = "one sealed move qualifies the exact future open state of one native door"
)]
pub fn prepare_quest_door_opening<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    map: &'a crate::map::boot::BundleMap,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    key: u64,
    pz_blocked: bool,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    let (at, _) = super::super::view::placement_of(key).ok_or(MovementError::NotQualified)?;
    let proof = DoorOpening {
        cells,
        map,
        key,
        revision: map.facts.doors().revision(key),
        actor,
        session,
        expected,
        destination: MovementLocalPosition {
            x: i32::from(at.x),
            y: i32::from(at.y),
            floor: -i16::from(at.floor),
        },
        pz_blocked,
    };
    proof.check(runtime)?;
    Ok(proof)
}
impl DoorOpening<'_> {
    fn check(&self, runtime: &ChannelRuntimeV1) -> Result<(), MovementError> {
        qualify_origin(runtime, self.cells, self.actor, self.session, self.expected)?;
        if self.map.facts.doors().revision(self.key) != self.revision
            || !self
                .map
                .facts
                .opening_door_enterable(self.map.overlay.base(), self.key)
            || runtime
                .position_occupied_by_other(self.actor, self.destination)
                .map_err(MovementError::Actor)?
        {
            return Err(MovementError::Blocked);
        }
        let origin = self.expected.position();
        if origin.floor != self.destination.floor
            || origin.x.abs_diff(self.destination.x) > 1
            || origin.y.abs_diff(self.destination.y) > 1
            || self.cells.scope().map_revision.as_str()
                != crate::map::overlay::map_revision(self.map.overlay.base())
        {
            return Err(MovementError::NotQualified);
        }
        let to = self.destination;
        if self.pz_blocked
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
                .is_ok_and(|tile| !tile.flags().protection_zone)
        {
            return Err(MovementError::Blocked);
        }
        Ok(())
    }
}
impl crate::foundation::source_step_seal::Sealed for DoorOpening<'_> {}
impl crate::foundation::SourceStepCommitProof for DoorOpening<'_> {
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
