//! Callback relocation uses the same sealed movement commit as stairs and rope.
use super::scripted_teleport::{destination_facing, qualify_present_tile_destination};
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EncounterLanding {
    Clear,
    SharePlayers,
    PresentTile,
}

struct CallbackTeleport<'a> {
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
    facing: Option<MovementFacing>,
    landing: EncounterLanding,
    allow_control_lost: bool,
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
        landing: EncounterLanding::Clear,
        allow_control_lost: false,
    };
    proof.check(runtime)?;
    Ok(proof)
}

/// The encounter binds its landing policy after qualifying the staged players.
#[allow(clippy::too_many_arguments)]
pub fn prepare_quest_encounter_arrival<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
    landing: EncounterLanding,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    let proof = CallbackTeleport {
        cells,
        actor,
        session,
        expected,
        destination,
        pz_blocked,
        closed,
        facing: (landing == EncounterLanding::PresentTile)
            .then(|| destination_facing(expected.position(), destination)),
        landing,
        allow_control_lost: false,
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
        landing: EncounterLanding::SharePlayers,
        allow_control_lost: true,
    };
    proof.check(runtime)?;
    Ok(proof)
}

impl CallbackTeleport<'_> {
    fn check(&self, runtime: &ChannelRuntimeV1) -> Result<(), MovementError> {
        // Only a forced encounter ejection moves a control-lost player.
        qualify_origin_control(
            runtime,
            self.cells,
            self.actor,
            self.session,
            self.expected,
            None,
            self.allow_control_lost,
        )?;
        let to = self.destination;
        if self.landing == EncounterLanding::PresentTile {
            return qualify_present_tile_destination(self.cells, self.expected.position(), to);
        }
        let cell = (to.x, to.y, i32::from(to.floor));
        if ground_of(self.cells)(cell) != Ground::Walkable
            || !landable_of(self.cells, self.pz_blocked, self.closed)(cell)
            || if self.landing == EncounterLanding::SharePlayers {
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
