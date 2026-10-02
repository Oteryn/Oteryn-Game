//! Child of the real player-vitals owner: movement pacing stays in the existing
//! PlayerSpellState, under the actual Channel owner work-item lock.
use super::ChannelSpellStates;
use crate::content::{LogicalCell, NativeEntryMovementCells};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, ExactActorRef, GameSessionId, MovementPositionSnapshot,
};
use crate::movement::speed::{
    BufferedStep, StepAdmission, StepPacing, qualified_ground_cost, step_duration_ms,
};
use crate::movement::{
    CardinalStep, MovementEngineeringSelection, MovementError, MovementOwnerTurn,
    MovementTurnOutcome,
};
use crate::spell::actor_conditions;

pub(crate) enum StepInChannel {
    Completed(Result<MovementPositionSnapshot, MovementError>),
    Pending { ready_at: u64 },
}
/// Equipment delta is supplied only by the actual persisted-equipment owner.
/// Absent pace metadata preserves the established baseline for an actor without
/// a speed condition; it never permits a speed spell to bypass its consumer.
pub(crate) fn step_in_channel(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    command: CommandId,
    now_us: u64,
    direction: CardinalStep,
    blocking: &std::collections::BTreeSet<LogicalCell>,
    equipment_delta: Option<i32>,
) -> StepInChannel {
    step_in_channel_with_source_step(
        runtime,
        states,
        cells,
        actor,
        session,
        command,
        now_us,
        direction,
        blocking,
        equipment_delta,
        None,
    )
}

pub(crate) fn step_in_channel_with_source_step(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    command: CommandId,
    now_us: u64,
    direction: CardinalStep,
    blocking: &std::collections::BTreeSet<LogicalCell>,
    equipment_delta: Option<i32>,
    prepared_source_step: Option<crate::movement::source_floor_change::SourceStepProof<'_>>,
) -> StepInChannel {
    let failure = |error| StepInChannel::Completed(Err(error));
    if prepared_source_step
        .as_ref()
        .is_some_and(|proof| !proof.matches_request(actor, session, direction))
    {
        return failure(MovementError::NotQualified);
    }
    if !runtime
        .player_control_facts(actor, session)
        .is_ok_and(|facts| facts.control_loss.is_none())
    {
        return failure(MovementError::NotQualified);
    }
    let Some(state) = states.get_mut(runtime, actor, session) else {
        // NoVocation admission has no cast/condition owner state. Preserve its
        // established Movement owner path without creating counterfeit vitals.
        if let Some(proof) = prepared_source_step {
            let position = proof.destination();
            if blocking.contains(&LogicalCell {
                x: position.x,
                y: position.y,
                z: i32::from(position.floor),
            }) {
                return failure(MovementError::Blocked);
            }
            return StepInChannel::Completed(
                runtime
                    .commit_source_step(proof)
                    .map_err(MovementError::Actor),
            );
        }
        return StepInChannel::Completed(baseline_step(
            runtime, cells, actor, session, direction, blocking,
        ));
    };
    let expected = match runtime.borrow_movement_position().read(actor) {
        Ok(v) => v,
        Err(e) => return failure(MovementError::Actor(e)),
    };
    let scope = cells.scope();
    if scope.world_id != runtime.binding().world_id()
        || scope.generation_digest != runtime.content_pin().server_artifact_digest()
        || expected.context() != runtime.pinned_movement_context()
    {
        return failure(MovementError::ScopeMismatch);
    }
    let source_step = if let Some(proof) = prepared_source_step {
        if proof.expected() != expected || proof.validate_current(runtime).is_err() {
            return failure(MovementError::NotQualified);
        }
        Some(proof)
    } else if crate::movement::source_floor_change::is_source_profile(cells) {
        match crate::movement::source_floor_change::prepare_source_step(
            runtime, cells, actor, session, expected, direction,
        ) {
            Ok(proof) => Some(proof),
            Err(error) => return failure(error),
        }
    } else {
        None
    };
    let destination = source_step.as_ref().map(|proof| proof.destination());
    let ground = source_step
        .as_ref()
        .and_then(|proof| proof.origin_ground_speed())
        .or_else(|| qualified_ground_cost(runtime, cells, actor, expected).ok());
    let pacing_enabled = ground.is_some() && equipment_delta.is_some();
    if !pacing_enabled && actor_conditions::has_speed_condition(state, now_us) {
        return failure(MovementError::NotQualified);
    }
    let mut pacing = actor_conditions::pacing_snapshot(state);
    let input = BufferedStep {
        game_session_id: session,
        command_id: command,
        direction,
    };
    let next_deadline = if pacing_enabled {
        match pacing.admit(now_us, input) {
            Ok(StepAdmission::Buffered { .. }) => {
                let ready_at = pacing.ready_at();
                actor_conditions::replace_pacing(state, pacing);
                return StepInChannel::Pending { ready_at };
            }
            Ok(StepAdmission::Ready | StepAdmission::ReadyBuffered) => {}
            Err(_) => return failure(MovementError::NotQualified),
        }
        let Some(equipment) = equipment_delta else {
            return failure(MovementError::NotQualified);
        };
        let Some(ground) = ground else {
            return failure(MovementError::NotQualified);
        };
        let speed = actor_conditions::movement_speed(state, now_us, equipment);
        let duration = match step_duration_ms(
            speed,
            ground,
            destination.is_some_and(|p| p.floor != expected.position().floor),
            false,
        ) {
            Ok(v) => v,
            Err(_) => return failure(MovementError::NotQualified),
        };
        match StepPacing::next_deadline(now_us, duration) {
            Ok(v) => Some(v),
            Err(_) => return failure(MovementError::NotQualified),
        }
    } else {
        None
    };
    let pos = expected.position();
    let (dx, dy) = match direction {
        CardinalStep::North => (0, -1),
        CardinalStep::East => (1, 0),
        CardinalStep::South => (0, 1),
        CardinalStep::West => (-1, 0),
    };
    let Some(x) = pos.x.checked_add(dx) else {
        return failure(MovementError::CoordinateOverflow);
    };
    let Some(y) = pos.y.checked_add(dy) else {
        return failure(MovementError::CoordinateOverflow);
    };
    let actual_destination = destination.unwrap_or(crate::foundation::MovementLocalPosition {
        x,
        y,
        floor: pos.floor,
    });
    if blocking.contains(&LogicalCell {
        x: actual_destination.x,
        y: actual_destination.y,
        z: i32::from(actual_destination.floor),
    }) {
        pacing.reject_buffered(input);
        actor_conditions::replace_pacing(state, pacing);
        return failure(MovementError::Blocked);
    }
    // Validate the pacing successor before the physical movement commit. Only
    // this already-computed infallible assignment follows a successful owner turn.
    if let Some(deadline) = next_deadline {
        if pacing.commit_step(now_us, deadline, input).is_err() {
            return failure(MovementError::NotQualified);
        }
    }
    let selection = MovementEngineeringSelection {
        owner_context: runtime.pinned_movement_context(),
        content_scope: scope,
    };
    let result = if let Some(proof) = source_step {
        runtime
            .commit_source_step(proof)
            .map(MovementTurnOutcome::Applied)
            .map_err(MovementError::Actor)
    } else {
        MovementOwnerTurn::begin(runtime, std::num::NonZeroUsize::MIN).try_step(
            actor,
            expected,
            &selection,
            cells.index(),
            direction,
        )
    };
    match result {
        Ok(MovementTurnOutcome::Applied(snapshot)) => {
            actor_conditions::replace_pacing(state, pacing);
            StepInChannel::Completed(Ok(snapshot))
        }
        Ok(MovementTurnOutcome::Deferred) => failure(MovementError::NotQualified),
        Err(error) => {
            let mut original = actor_conditions::pacing_snapshot(state);
            original.reject_buffered(input);
            actor_conditions::replace_pacing(state, original);
            failure(error)
        }
    }
}

fn baseline_step(
    runtime: &mut ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    direction: CardinalStep,
    blocking: &std::collections::BTreeSet<LogicalCell>,
) -> Result<MovementPositionSnapshot, MovementError> {
    let expected = runtime
        .borrow_movement_position()
        .read(actor)
        .map_err(MovementError::Actor)?;
    let scope = cells.scope();
    let owner_context = runtime.pinned_movement_context();
    if scope.world_id != runtime.binding().world_id()
        || scope.generation_digest != runtime.content_pin().server_artifact_digest()
        || expected.context() != owner_context
    {
        return Err(MovementError::ScopeMismatch);
    }
    if crate::movement::source_floor_change::is_source_profile(cells) {
        let proof = crate::movement::source_floor_change::prepare_source_step(
            runtime, cells, actor, session, expected, direction,
        )?;
        let p = proof.destination();
        if blocking.contains(&LogicalCell {
            x: p.x,
            y: p.y,
            z: i32::from(p.floor),
        }) {
            return Err(MovementError::Blocked);
        }
        return runtime
            .commit_source_step(proof)
            .map_err(MovementError::Actor);
    }
    let pos = expected.position();
    let (dx, dy) = match direction {
        CardinalStep::North => (0, -1),
        CardinalStep::East => (1, 0),
        CardinalStep::South => (0, 1),
        CardinalStep::West => (-1, 0),
    };
    let x = pos
        .x
        .checked_add(dx)
        .ok_or(MovementError::CoordinateOverflow)?;
    let y = pos
        .y
        .checked_add(dy)
        .ok_or(MovementError::CoordinateOverflow)?;
    if blocking.contains(&LogicalCell {
        x,
        y,
        z: i32::from(pos.floor),
    }) {
        return Err(MovementError::Blocked);
    }
    let selection = MovementEngineeringSelection {
        owner_context,
        content_scope: scope,
    };
    match MovementOwnerTurn::begin(runtime, std::num::NonZeroUsize::MIN).try_step(
        actor,
        expected,
        &selection,
        cells.index(),
        direction,
    )? {
        MovementTurnOutcome::Applied(snapshot) => Ok(snapshot),
        MovementTurnOutcome::Deferred => Err(MovementError::NotQualified),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};
    use crate::spell::cast::CharacterCastFacts;
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros,
    };
    fn id(n: u8) -> [u8; 16] {
        [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n]
    }
    fn owner() -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        crate::content::QualifiedNativeEntryRoom,
        ExactActorRef,
        GameSessionId,
    ) {
        let world = WorldId::decode(&id(1)).unwrap();
        let room = crate::content::qualify_native_entry_room(world).unwrap();
        let start = room.entry_start();
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).unwrap(),
            NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .unwrap();
        let session = GameSessionId::decode(&id(4)).unwrap();
        let reservation = runtime.reserve_fresh_session(session).unwrap();
        let actor = runtime.commit_fresh_session(reservation).unwrap();
        runtime.initialize_first_entry_position(actor).unwrap();
        (runtime, ChannelSpellStates::default(), room, actor, session)
    }
    #[test]
    fn real_first_entry_without_spell_state_keeps_baseline_and_door_blocking() {
        let (mut runtime, mut states, room, actor, session) = owner();
        let cmd = CommandId::new(1).unwrap();
        let blocked = std::collections::BTreeSet::from([LogicalCell { x: 1, y: 0, z: 0 }]);
        assert!(matches!(
            step_in_channel(
                &mut runtime,
                &mut states,
                room.movement_cells(),
                actor,
                session,
                cmd,
                0,
                CardinalStep::East,
                &blocked,
                None
            ),
            StepInChannel::Completed(Err(MovementError::Blocked))
        ));
        let empty = std::collections::BTreeSet::new();
        assert!(matches!(
            step_in_channel(
                &mut runtime,
                &mut states,
                room.movement_cells(),
                actor,
                session,
                cmd,
                0,
                CardinalStep::East,
                &empty,
                None
            ),
            StepInChannel::Completed(Ok(_))
        ));
        assert!(states.actors.is_empty());
        assert_eq!(
            runtime
                .borrow_movement_position()
                .read(actor)
                .unwrap()
                .position()
                .x,
            1
        );
    }
    #[test]
    fn actual_speed_condition_cannot_move_without_qualified_pacing_consumer() {
        let (mut runtime, mut states, room, actor, session) = owner();
        let facts = CharacterCastFacts {
            vocation: crate::spell::Vocation::Knight,
            level: 291,
            magic_level: 10,
            max_health: 1000,
            max_mana: 500,
            max_soul: 100,
        };
        states
            .initialize(
                &runtime,
                actor,
                session,
                facts,
                (0, 0),
                SemanticTimeMicros::from_micros(0),
            )
            .unwrap();
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
        ))
        .unwrap();
        let row = document["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == "Haste")
            .unwrap();
        let spell = crate::spell::native::spell_from_bundle(
            &serde_json::json!({"spell":row["spell"]}),
            &row["dependencies"],
        )
        .unwrap();
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let application = crate::ability::condition::ApplicationFacts {
            now: 0,
            base_speed: 400,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
        };
        crate::spell::companion_execution::stage_haste(
            states.get_mut(&runtime, actor, session).unwrap(),
            &spell,
            &[],
            actor,
            &application,
        )
        .unwrap();
        let before = runtime.borrow_movement_position().read(actor).unwrap();
        assert!(matches!(
            step_in_channel(
                &mut runtime,
                &mut states,
                room.movement_cells(),
                actor,
                session,
                CommandId::new(1).unwrap(),
                1,
                CardinalStep::East,
                &std::collections::BTreeSet::new(),
                None
            ),
            StepInChannel::Completed(Err(MovementError::NotQualified))
        ));
        assert_eq!(
            runtime.borrow_movement_position().read(actor).unwrap(),
            before
        );
    }
}

/// Used before accepting a speed spell. A runtime snapshot's presence alone
/// cannot claim equipment authority: its value must come from that owner.
pub(crate) fn pacing_available(
    runtime: &mut ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    equipment_delta: Option<i32>,
) -> bool {
    if equipment_delta.is_none()
        || !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
    {
        return false;
    }
    let Ok(expected) = runtime.borrow_movement_position().read(actor) else {
        return false;
    };
    qualified_ground_cost(runtime, cells, actor, expected).is_ok()
}
