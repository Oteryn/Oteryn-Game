//! Callback relocation uses the same sealed movement commit as stairs and rope.
use super::*;

struct CallbackTeleport<'a> {
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
    facing: Option<MovementFacing>,
    player_stack: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_quest_callback_teleport<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    prepare_quest_callback_teleport_facing(
        runtime,
        cells,
        actor,
        session,
        expected,
        destination,
        pz_blocked,
        closed,
        None,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "the proof binds the current actor, movement scope, landing rules and facing"
)]
pub fn prepare_quest_callback_teleport_facing<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
    facing: Option<MovementFacing>,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    let proof = CallbackTeleport {
        cells,
        actor,
        session,
        expected,
        destination,
        pz_blocked,
        closed,
        facing,
        player_stack: false,
    };
    proof.check(runtime)?;
    Ok(proof)
}

/// Forced source encounter exits may gather players on one landing tile.
/// Current map, actor/session and creature occupancy remain authoritative.
pub fn prepare_quest_encounter_ejection<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    closed: &'a dyn CellBlocker,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    let proof = CallbackTeleport {
        cells,
        actor,
        session,
        expected,
        destination,
        closed,
        pz_blocked: false,
        facing: None,
        player_stack: true,
    };
    proof.check(runtime)?;
    Ok(proof)
}

impl CallbackTeleport<'_> {
    fn check(&self, runtime: &ChannelRuntimeV1) -> Result<(), MovementError> {
        qualify_origin(
            runtime,
            self.cells,
            self.actor,
            self.session,
            self.expected,
            None,
        )?;
        let to = self.destination;
        let cell = (to.x, to.y, i32::from(to.floor));
        if ground_of(self.cells)(cell) != Ground::Walkable
            || !landable_of(self.cells, self.pz_blocked, self.closed)(cell)
            || if self.player_stack {
                runtime
                    .positioned_melee_census()
                    .map_err(MovementError::Actor)?
                    .iter()
                    .any(|(actor, position, session)| {
                        *actor != self.actor && session.is_none() && position.position() == to
                    })
            } else {
                runtime
                    .position_occupied_by_other(self.actor, to)
                    .map_err(MovementError::Actor)?
            }
        {
            return Err(MovementError::Blocked);
        }
        Ok(())
    }
}

impl crate::foundation::source_step_seal::Sealed for CallbackTeleport<'_> {}
impl crate::foundation::SourceStepCommitProof for CallbackTeleport<'_> {
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
            self.facing
                .or(self.expected.facing())
                .unwrap_or(MovementFacing::South),
        )
    }
}
