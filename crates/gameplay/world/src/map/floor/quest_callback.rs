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
    reject_occupied: bool,
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
        reject_occupied: true,
    };
    proof.check(runtime)?;
    Ok(proof)
}

impl CallbackTeleport<'_> {
    fn check(&self, runtime: &ChannelRuntimeV1) -> Result<(), MovementError> {
        qualify_origin(runtime, self.cells, self.actor, self.session, self.expected)?;
        let to = self.destination;
        let cell = (to.x, to.y, i32::from(to.floor));
        if ground_of(self.cells)(cell) != Ground::Walkable
            || !landable_of(self.cells, self.pz_blocked, self.closed)(cell)
            || (self.reject_occupied
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
            self.facing
                .or(self.expected.facing())
                .unwrap_or(MovementFacing::South),
        )
    }
}

/// Closed ordinary-World Halloween transfer. This does not allocate an Instance,
/// displace another actor, search for a destination or confer reward authority.
#[derive(Clone, Copy)]
pub enum PumpkinRoomMove {
    Entry,
    Return,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_pumpkin_room_transfer<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    kind: PumpkinRoomMove,
    closed: &'a dyn CellBlocker,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    // The derived layout is a real, current B3 binding. A bare coordinate never
    // substitutes for its base reward object or exact source temple transition.
    let table = table_of(cells).ok_or(MovementError::NotQualified)?;
    if table.teleport(TilePos {
        x: 33317,
        y: 32419,
        floor: -15,
    }) != Some(TilePos {
        x: 33213,
        y: 32454,
        floor: -1,
    }) || !cells
        .spell_tiles()
        .lookup(
            cells.scope(),
            LogicalCell {
                x: 33317,
                y: 32414,
                z: 15,
            },
        )
        .is_ok_and(|tile| tile.top_source_ids().contains(&12816))
        || !cells
            .spell_tiles()
            .lookup(
                cells.scope(),
                LogicalCell {
                    x: 33317,
                    y: 32415,
                    z: 7,
                },
            )
            .is_ok_and(|tile| tile.top_source_ids().contains(&1949))
    {
        return Err(MovementError::NotQualified);
    }
    let from = expected.position();
    let destination = match kind {
        PumpkinRoomMove::Entry if (from.x, from.y, from.floor) == (33317, 32415, 7) => {
            MovementLocalPosition {
                x: 33317,
                y: 32417,
                floor: 15,
            }
        }
        PumpkinRoomMove::Return
            if from.floor == 15
                && (33312..=33322).contains(&from.x)
                && (32410..=32420).contains(&from.y) =>
        {
            MovementLocalPosition {
                x: 33213,
                y: 32454,
                floor: 1,
            }
        }
        _ => return Err(MovementError::NotQualified),
    };
    // Normal Bundle teleports already allow players on one landing tile. This
    // forced, source-fixed event return uses that same stacking behavior; every
    // ordinary callback caller retains its stricter occupancy check above.
    let proof = CallbackTeleport {
        cells,
        actor,
        session,
        expected,
        destination,
        pz_blocked: false,
        closed,
        facing: None,
        reject_occupied: false,
    };
    proof.check(runtime)?;
    Ok(proof)
}
