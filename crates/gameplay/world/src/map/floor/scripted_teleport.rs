//! Explicit source-script displacement may land on a present blocked pad.
//! Ordinary callbacks delegate to the strict landing owner. Scripted landings
//! retain current actor/session and map presence. Each source binds its landing policy.
use super::*;

/// A sealed source movement bound to the current player and pinned map.
pub struct ScriptedTeleport<'a> {
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    facing: Option<MovementFacing>,
    landing_checks: Option<LandingChecks<'a>>,
}

struct LandingChecks<'a> {
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
    allow_blocked_ground: bool,
}

#[allow(
    clippy::too_many_arguments,
    reason = "the sealed proof binds current scope, landing facts and the authored script policy"
)]
pub fn prepare_scripted_quest_teleport_facing<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
    facing: Option<MovementFacing>,
    scripted: bool,
) -> Result<ScriptedTeleport<'a>, MovementError> {
    let proof = ScriptedTeleport {
        cells,
        actor,
        session,
        expected,
        destination,
        facing,
        landing_checks: Some(LandingChecks {
            pz_blocked,
            closed,
            allow_blocked_ground: scripted,
        }),
    };
    proof.check(runtime)?;
    Ok(proof)
}

/// Pinned Crystal game.cpp:3167-3192 uses FLAG_NOLIMIT on an existing tile.
/// Used by source callbacks that invoke Creature:teleportTo.
pub fn prepare_quest_teleport_to_present_tile<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    facing: Option<MovementFacing>,
) -> Result<ScriptedTeleport<'a>, MovementError> {
    // creature_functions.cpp:895-906 makes horizontal displacement take priority.
    let origin = expected.position();
    let facing = facing.unwrap_or_else(|| match destination.x.cmp(&origin.x) {
        std::cmp::Ordering::Less => MovementFacing::West,
        std::cmp::Ordering::Greater => MovementFacing::East,
        std::cmp::Ordering::Equal if destination.y > origin.y => MovementFacing::South,
        std::cmp::Ordering::Equal => MovementFacing::North,
    });
    let proof = ScriptedTeleport {
        cells,
        actor,
        session,
        expected,
        destination,
        facing: Some(facing),
        landing_checks: None,
    };
    proof.check(runtime)?;
    Ok(proof)
}

impl ScriptedTeleport<'_> {
    fn check(&self, runtime: &ChannelRuntimeV1) -> Result<(), MovementError> {
        if let Some(checks) = &self.landing_checks
            && !checks.allow_blocked_ground
        {
            return prepare_quest_callback_teleport_facing(
                runtime,
                self.cells,
                self.actor,
                self.session,
                self.expected,
                self.destination,
                checks.pz_blocked,
                checks.closed,
                self.facing,
            )
            .map(|_| ());
        }
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
        if ground_of(self.cells)(cell) == Ground::Absent {
            return Err(MovementError::Blocked);
        }
        if let Some(checks) = &self.landing_checks {
            if !landable_of(self.cells, checks.pz_blocked, checks.closed)(cell)
                || runtime
                    .position_occupied_by_other(self.actor, to)
                    .map_err(MovementError::Actor)?
            {
                return Err(MovementError::Blocked);
            }
        } else if self.expected.position() == to {
            return Err(MovementError::Blocked);
        }
        Ok(())
    }
}
impl crate::foundation::source_step_seal::Sealed for ScriptedTeleport<'_> {}
impl crate::foundation::SourceStepCommitProof for ScriptedTeleport<'_> {
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
