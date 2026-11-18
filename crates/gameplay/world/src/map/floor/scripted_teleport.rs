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
    allow_control_lost: bool,
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
        allow_control_lost: false,
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
    let proof = present_tile(cells, actor, session, expected, destination, facing, false);
    proof.check(runtime)?;
    Ok(proof)
}

/// Wanted and Defiances timeout scripts can move positioned challengers after
/// control loss. Crystal spectators.lua:113-132 calls Creature:teleportTo.
pub fn prepare_quest_timeout_ejection<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
) -> Result<ScriptedTeleport<'a>, MovementError> {
    let proof = present_tile(cells, actor, session, expected, destination, None, true);
    proof.check(runtime)?;
    Ok(proof)
}

fn present_tile(
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    facing: Option<MovementFacing>,
    allow_control_lost: bool,
) -> ScriptedTeleport<'_> {
    let facing = facing.unwrap_or_else(|| destination_facing(expected.position(), destination));
    ScriptedTeleport {
        cells,
        actor,
        session,
        expected,
        destination,
        facing: Some(facing),
        allow_control_lost,
        landing_checks: None,
    }
}

pub(super) fn destination_facing(
    origin: MovementLocalPosition,
    destination: MovementLocalPosition,
) -> MovementFacing {
    // Crystal creature_functions.cpp:895-906 gives horizontal displacement priority.
    match destination.x.cmp(&origin.x) {
        std::cmp::Ordering::Less => MovementFacing::West,
        std::cmp::Ordering::Greater => MovementFacing::East,
        std::cmp::Ordering::Equal if destination.y > origin.y => MovementFacing::South,
        std::cmp::Ordering::Equal => MovementFacing::North,
    }
}

pub(super) fn qualify_present_tile_destination(
    cells: &NativeEntryMovementCells,
    origin: MovementLocalPosition,
    destination: MovementLocalPosition,
) -> Result<(), MovementError> {
    let cell = (destination.x, destination.y, i32::from(destination.floor));
    if origin == destination || ground_of(cells)(cell) == Ground::Absent {
        return Err(MovementError::Blocked);
    }
    Ok(())
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
        let Some(checks) = &self.landing_checks else {
            return qualify_present_tile_destination(self.cells, self.expected.position(), to);
        };
        let cell = (to.x, to.y, i32::from(to.floor));
        if ground_of(self.cells)(cell) == Ground::Absent
            || !landable_of(self.cells, checks.pz_blocked, checks.closed)(cell)
            || runtime
                .position_occupied_by_other(self.actor, to)
                .map_err(MovementError::Actor)?
        {
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
