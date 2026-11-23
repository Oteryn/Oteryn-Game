//! Explicit source-script displacement may land on a present blocked pad.
//! Ordinary callbacks delegate to the strict landing owner. Scripted landings
//! retain current actor/session, map presence, PZ, closed-door and occupancy checks.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum QuestTeleportPolicy {
    Strict,
    Scripted,
    /// An exact source callback uses Crystal's default `teleportTo` movement.
    SourceNoLimit,
}

struct ScriptedTeleport<'a> {
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
    facing: Option<MovementFacing>,
    policy: QuestTeleportPolicy,
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
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    prepare_quest_teleport_with_policy(
        runtime,
        cells,
        actor,
        session,
        expected,
        destination,
        pz_blocked,
        closed,
        facing,
        if scripted {
            QuestTeleportPolicy::Scripted
        } else {
            QuestTeleportPolicy::Strict
        },
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "the sealed proof binds the current actor and the exact source landing policy"
)]
pub fn prepare_quest_teleport_with_policy<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
    facing: Option<MovementFacing>,
    policy: QuestTeleportPolicy,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    let proof = ScriptedTeleport {
        cells,
        actor,
        session,
        expected,
        destination,
        pz_blocked,
        closed,
        facing,
        policy,
    };
    proof.check(runtime)?;
    Ok(proof)
}

impl ScriptedTeleport<'_> {
    fn check(&self, runtime: &ChannelRuntimeV1) -> Result<(), MovementError> {
        if self.policy == QuestTeleportPolicy::Strict {
            return prepare_quest_callback_teleport_facing(
                runtime,
                self.cells,
                self.actor,
                self.session,
                self.expected,
                self.destination,
                self.pz_blocked,
                self.closed,
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
        // Crystal game.cpp:3167–3195 and tile.cpp:601–608 require a present tile,
        // then FLAG_NOLIMIT skips its ordinary admission checks.
        if self.policy == QuestTeleportPolicy::Scripted
            && (!landable_of(self.cells, self.pz_blocked, self.closed)(cell)
                || runtime
                    .position_occupied_by_other(self.actor, to)
                    .map_err(MovementError::Actor)?)
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
        let departure = self.expected.position();
        let facing =
            if self.policy == QuestTeleportPolicy::SourceNoLimit && departure != self.destination {
                crate::movement::monster_map::source_teleport_facing(departure, self.destination)
            } else {
                self.facing
                    .or(self.expected.facing())
                    .unwrap_or(MovementFacing::South)
            };
        (self.actor, self.expected, self.destination, facing)
    }
}
