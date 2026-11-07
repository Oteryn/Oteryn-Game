//! Callback relocation uses the same sealed movement commit as stairs and rope.
use super::*;

struct CallbackTeleport<'a> {
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: Option<GameSessionId>,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
    forced: bool,
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
    let proof = CallbackTeleport {
        cells,
        actor,
        session: Some(session),
        expected,
        destination,
        pz_blocked,
        closed,
        forced: false,
    };
    proof.check(runtime)?;
    Ok(proof)
}

/// A declared source-world ejection may stack its occupants at one safe landing.
/// It retains scope, actor generation, position, spell reservation and PZ checks.
#[allow(
    clippy::too_many_arguments,
    reason = "one proof binds the exact actor, session, position and destination world policy"
)]
pub fn prepare_quest_callback_ejection<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: Option<GameSessionId>,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    let proof = CallbackTeleport {
        cells,
        actor,
        session,
        expected,
        destination,
        pz_blocked,
        closed,
        forced: true,
    };
    proof.check(runtime)?;
    Ok(proof)
}

impl CallbackTeleport<'_> {
    fn check(&self, runtime: &ChannelRuntimeV1) -> Result<(), MovementError> {
        if self.forced {
            if runtime.actor_spell_reserved(self.actor)
                || self.cells.scope().world_id != runtime.binding().world_id()
                || self.cells.scope().generation_digest
                    != runtime.content_pin().server_artifact_digest()
                || self.expected.context() != runtime.pinned_movement_context()
                || runtime
                    .read_actor_position(self.actor)
                    .map_err(MovementError::Actor)?
                    != self.expected
                || match self.session {
                    Some(session) => !runtime
                        .player_control_facts(self.actor, session)
                        .is_ok_and(|facts| facts.control_loss.is_none()),
                    None => !runtime.contains_live_creature(self.actor),
                }
            {
                return Err(MovementError::NotQualified);
            }
        } else {
            qualify_origin(
                runtime,
                self.cells,
                self.actor,
                self.session.ok_or(MovementError::NotQualified)?,
                self.expected,
            )?;
        }
        let to = self.destination;
        let cell = (to.x, to.y, i32::from(to.floor));
        if ground_of(self.cells)(cell) != Ground::Walkable
            || !landable_of(self.cells, self.pz_blocked, self.closed)(cell)
            || (!self.forced
                && runtime
                    .position_occupied_by_other(self.actor, to)
                    .map_err(MovementError::Actor)?)
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
            self.expected.facing().unwrap_or(MovementFacing::South),
        )
    }
}
