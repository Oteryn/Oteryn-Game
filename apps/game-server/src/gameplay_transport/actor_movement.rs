//! Child of the real player-vitals owner: one player step under the actual Channel owner
//! work-item lock, with its SPEED-1 step duration for the connection's pacing clock.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::ChannelSpellStates;
use crate::content::{LogicalCell, NativeEntryMovementCells};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition, MovementPositionSnapshot,
};
use crate::movement::speed::{
    EngineeringGroundSpeed, GroundSpeedSource, player_step_duration, runtime_player_speed,
};
use crate::movement::{
    CardinalStep, MovementEngineeringSelection, MovementError, MovementOwnerTurn,
    MovementTurnOutcome,
};
use crate::spell::actor_conditions;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::time::Duration;

/// A committed step and its SPEED-1 duration (CONDITIONS-0 §4.2), or why nothing moved.
pub(crate) type StepInChannel = Result<(MovementPositionSnapshot, Duration), MovementError>;

/// The cell of a local position, for a ground speed lookup.
fn cell(position: MovementLocalPosition) -> LogicalCell {
    LogicalCell {
        x: position.x,
        y: position.y,
        z: i32::from(position.floor),
    }
}

/// The step duration onto `onto` at `speed`; `NotQualified` when it cannot be computed
/// (ground speed 0 or no verified table), so the step is refused before anything moves.
fn duration_onto(
    source: &impl GroundSpeedSource,
    onto: MovementLocalPosition,
    speed: Option<u16>,
) -> Result<Duration, MovementError> {
    speed
        .and_then(|speed| player_step_duration(source, cell(onto), speed))
        .ok_or(MovementError::NotQualified)
}

/// A player without spell state: SPEED-1's level term until a progression owner supplies it,
/// with the runtime condition owner's `SPEED` delta and no equipment term.
fn baseline_speed(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    now_us: u64,
) -> Option<u16> {
    runtime_player_speed(
        runtime,
        actor,
        session,
        crate::gameplay_transport::PLAYER_LEVEL_UNTIL_PROGRESSION_OWNER,
        SemanticTimeMicros::from_micros(now_us),
    )
}

/// An initialized player's effective speed: its base speed, the runtime condition owner's and
/// its spell conditions' `SPEED` deltas, and the equipment speed. On qualified source ground
/// the equipment speed must come from the persisted-equipment owner, so an unavailable read
/// refuses the step; elsewhere it is 0 until that owner is composed (SPEED-1 §4.1).
fn spell_state_speed(
    runtime: &ChannelRuntimeV1,
    state: &crate::spell::cast::PlayerSpellState,
    actor: ExactActorRef,
    session: GameSessionId,
    now_us: u64,
    equipment_delta: Option<i32>,
    qualified_ground: bool,
) -> Option<u16> {
    let equipment = match equipment_delta {
        Some(delta) => i64::from(delta),
        None if qualified_ground => return None,
        None => 0,
    };
    let runtime_delta = runtime
        .actor_active_speed_delta(
            actor,
            Some(session),
            SemanticTimeMicros::from_micros(now_us),
        )
        .ok()?;
    Some(actor_conditions::movement_speed_with(
        state,
        now_us,
        runtime_delta,
        equipment,
    ))
}

/// Equipment delta is supplied only by the actual persisted-equipment owner. An actor without
/// spell state keeps the established baseline Movement path. Every step is paced (SPEED-1
/// §4.3): its duration onto the destination is computed before the commit, and a step whose
/// duration cannot be computed is refused.
#[allow(
    clippy::too_many_arguments,
    reason = "the owner turn binds every independently resolved fact explicitly"
)]
pub(crate) fn step_in_channel(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
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
        now_us,
        direction,
        blocking,
        equipment_delta,
        None,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "the owner turn binds every independently resolved fact explicitly"
)]
pub(crate) fn step_in_channel_with_source_step(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    now_us: u64,
    direction: CardinalStep,
    blocking: &std::collections::BTreeSet<LogicalCell>,
    equipment_delta: Option<i32>,
    prepared_source_step: Option<crate::movement::source_floor_change::SourceStepProof<'_>>,
) -> StepInChannel {
    let failure = Err;
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
        let speed = baseline_speed(runtime, actor, session, now_us);
        if let Some(proof) = prepared_source_step {
            let position = proof.destination();
            if blocking.contains(&cell(position)) {
                return failure(MovementError::Blocked);
            }
            let duration = duration_onto(&proof, position, speed)?;
            return runtime
                .commit_source_step(proof)
                .map(|snapshot| (snapshot, duration))
                .map_err(MovementError::Actor);
        }
        return baseline_step(runtime, cells, actor, session, direction, blocking, speed);
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
    let speed = spell_state_speed(
        runtime,
        state,
        actor,
        session,
        now_us,
        equipment_delta,
        source_step.is_some(),
    );
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
    if blocking.contains(&cell(actual_destination)) {
        return failure(MovementError::Blocked);
    }
    let duration = match &source_step {
        Some(proof) => duration_onto(proof, actual_destination, speed)?,
        None => duration_onto(&EngineeringGroundSpeed, actual_destination, speed)?,
    };
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
        Ok(MovementTurnOutcome::Applied(snapshot)) => Ok((snapshot, duration)),
        Ok(MovementTurnOutcome::Deferred) => failure(MovementError::NotQualified),
        Err(error) => failure(error),
    }
}

fn baseline_step(
    runtime: &mut ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    direction: CardinalStep,
    blocking: &std::collections::BTreeSet<LogicalCell>,
    speed: Option<u16>,
) -> StepInChannel {
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
        if blocking.contains(&cell(p)) {
            return Err(MovementError::Blocked);
        }
        let duration = duration_onto(&proof, p, speed)?;
        return runtime
            .commit_source_step(proof)
            .map(|snapshot| (snapshot, duration))
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
    let onto = MovementLocalPosition {
        x,
        y,
        floor: pos.floor,
    };
    if blocking.contains(&cell(onto)) {
        return Err(MovementError::Blocked);
    }
    let duration = duration_onto(&EngineeringGroundSpeed, onto, speed)?;
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
        MovementTurnOutcome::Applied(snapshot) => Ok((snapshot, duration)),
        MovementTurnOutcome::Deferred => Err(MovementError::NotQualified),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::items_after_test_module)]
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
    pub(super) fn owner() -> (
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
        let blocked = std::collections::BTreeSet::from([LogicalCell { x: 1, y: 0, z: 0 }]);
        assert!(matches!(
            step_in_channel(
                &mut runtime,
                &mut states,
                room.movement_cells(),
                actor,
                session,
                0,
                CardinalStep::East,
                &blocked,
                None
            ),
            Err(MovementError::Blocked)
        ));
        let empty = std::collections::BTreeSet::new();
        assert!(matches!(
            step_in_channel(
                &mut runtime,
                &mut states,
                room.movement_cells(),
                actor,
                session,
                0,
                CardinalStep::East,
                &empty,
                None
            ),
            // SPEED-1: level 1 on 150 ground, 1000 × 150 / 278 → 550 ms.
            Ok((_, duration)) if duration == Duration::from_millis(550)
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
    fn initialized_player_on_engineering_ground_is_paced_from_its_base_speed() {
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
        assert!(matches!(
            step_in_channel(
                &mut runtime,
                &mut states,
                room.movement_cells(),
                actor,
                session,
                1,
                CardinalStep::East,
                &std::collections::BTreeSet::new(),
                None
            ),
            // Base speed 400 on 150 ground: 200 ms.
            Ok((_, duration)) if duration == Duration::from_millis(200)
        ));
    }
    #[test]
    fn actual_speed_condition_enters_the_effective_speed_of_the_paced_step() {
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
        let state = states.get(&runtime, actor, session).unwrap();
        let hasted = spell_state_speed(&runtime, state, actor, session, 1, None, false).unwrap();
        assert!(hasted > 400, "{hasted}");
        // On qualified source ground the equipment speed must come from its owner.
        assert_eq!(
            spell_state_speed(&runtime, state, actor, session, 1, None, true),
            None
        );
        let expected = player_step_duration(
            &EngineeringGroundSpeed,
            LogicalCell { x: 1, y: 0, z: 0 },
            hasted,
        )
        .unwrap();
        assert!(matches!(
            step_in_channel(
                &mut runtime,
                &mut states,
                room.movement_cells(),
                actor,
                session,
                1,
                CardinalStep::East,
                &std::collections::BTreeSet::new(),
                None
            ),
            Ok((_, duration)) if duration == expected
        ));
    }
}

#[cfg(test)]
mod creature_field_step_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    fn ready() -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        crate::content::QualifiedNativeEntryRoom,
        ExactActorRef,
        GameSessionId,
    ) {
        let (runtime, mut states, room, actor, session) = super::tests::owner();
        states
            .initialize(
                &runtime,
                actor,
                session,
                crate::spell::cast::CharacterCastFacts {
                    vocation: crate::spell::Vocation::Knight,
                    level: 291,
                    magic_level: 10,
                    max_health: 1000,
                    max_mana: 500,
                    max_soul: 100,
                },
                (0, 0),
                SemanticTimeMicros::from_micros(0),
            )
            .unwrap();
        (runtime, states, room, actor, session)
    }
    fn contact(runtime: &ChannelRuntimeV1) -> super::super::CreatureFieldContact {
        // Real immutable qualified field recipe, actual Player/movement owner fixture.
        let catalog: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/spell-authoring/samples/native-field-profiles.json"
        ))
        .unwrap();
        let recipe = catalog["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                serde_json::from_value::<crate::ability::field_condition::QualifiedFieldCondition>(
                    v.clone(),
                )
                .unwrap()
            })
            .find(|r| r.source.server == "canary" && r.source.server_item_id == 2118)
            .unwrap();
        let values = recipe.condition_values().unwrap().unwrap();
        super::super::CreatureFieldContact {
            source: "creature:field:owner-component-fixture".into(),
            definition: crate::ability::condition::ConditionDefinition::new(
                "native.field.i2118",
                1,
                values,
            )
            .unwrap(),
            element: recipe.element().unwrap().unwrap(),
            content: runtime.content_pin().server_artifact_digest(),
        }
    }
    #[test]
    fn actual_step_and_creature_field_initial_hp_dot_publish_together() {
        let (mut runtime, mut states, room, actor, session) = ready();
        let field = contact(&runtime);
        let before = states.get(&runtime, actor, session).unwrap().clone();
        assert!(
            states
                .step_with_creature_field_contact(
                    &mut runtime,
                    room.movement_cells(),
                    actor,
                    session,
                    0,
                    CardinalStep::East,
                    &Default::default(),
                    None,
                    None,
                    Some(field)
                )
                .is_ok()
        );
        let next = states.get(&runtime, actor, session).unwrap();
        assert!(next.vitals().health < before.vitals().health);
        assert_eq!(next.revision(), before.revision() + 1);
        assert!(
            next.owned_conditions()
                .instances()
                .iter()
                .any(|i| i.provenance().source_kind
                    == crate::ability::condition::ConditionSourceKind::Creature)
        );
        assert_eq!(runtime.read_actor_position(actor).unwrap().position().x, 1);
    }
    #[test]
    fn blocked_step_leaves_initial_hp_condition_cursor_and_position_unchanged() {
        let (mut runtime, mut states, room, actor, session) = ready();
        let field = contact(&runtime);
        let before = states.get(&runtime, actor, session).unwrap().clone();
        let position = runtime.read_actor_position(actor).unwrap();
        assert!(
            states
                .step_with_creature_field_contact(
                    &mut runtime,
                    room.movement_cells(),
                    actor,
                    session,
                    0,
                    CardinalStep::East,
                    &std::collections::BTreeSet::from([LogicalCell { x: 1, y: 0, z: 0 }]),
                    None,
                    None,
                    Some(field)
                )
                .is_err()
        );
        assert_eq!(states.get(&runtime, actor, session), Some(&before));
        assert_eq!(runtime.read_actor_position(actor).unwrap(), position);
        assert!(states.deaths.is_empty());
    }
    #[test]
    fn older_field_contact_clock_refuses_before_next_position_or_hp_publication() {
        let (mut runtime, mut states, room, actor, session) = ready();
        let field = contact(&runtime);
        assert!(
            states
                .step_with_creature_field_contact(
                    &mut runtime,
                    room.movement_cells(),
                    actor,
                    session,
                    10,
                    CardinalStep::East,
                    &Default::default(),
                    None,
                    None,
                    Some(field)
                )
                .is_ok()
        );
        let before = states.get(&runtime, actor, session).unwrap().clone();
        let position = runtime.read_actor_position(actor).unwrap();
        let field = contact(&runtime);
        assert!(
            states
                .step_with_creature_field_contact(
                    &mut runtime,
                    room.movement_cells(),
                    actor,
                    session,
                    0,
                    CardinalStep::West,
                    &Default::default(),
                    None,
                    None,
                    Some(field)
                )
                .is_err()
        );
        assert_eq!(states.get(&runtime, actor, session), Some(&before));
        assert_eq!(runtime.read_actor_position(actor).unwrap(), position);
        assert!(states.deaths.is_empty());
    }
    #[test]
    fn failed_lethal_mint_does_not_move_or_install_dot_then_retry_mints_one_actual_death() {
        let (mut runtime, mut states, room, actor, session) = ready();
        let before = states.get(&runtime, actor, session).unwrap().clone();
        let (next, _) =
            crate::spell::actor_conditions::stage_creature_hit(&before, 999, 0).unwrap();
        let index = states.index(actor, session).unwrap();
        states.actors[index].2 = next;
        let before = states.get(&runtime, actor, session).unwrap().clone();
        let position = runtime.read_actor_position(actor).unwrap();
        let mint = states.mint_death;
        states.mint_death = || None;
        let field = contact(&runtime);
        assert!(
            states
                .step_with_creature_field_contact(
                    &mut runtime,
                    room.movement_cells(),
                    actor,
                    session,
                    0,
                    CardinalStep::East,
                    &Default::default(),
                    None,
                    None,
                    Some(field)
                )
                .is_err()
        );
        assert_eq!(states.get(&runtime, actor, session), Some(&before));
        assert_eq!(runtime.read_actor_position(actor).unwrap(), position);
        assert!(states.deaths.is_empty());
        states.mint_death = mint;
        let field = contact(&runtime);
        assert!(
            states
                .step_with_creature_field_contact(
                    &mut runtime,
                    room.movement_cells(),
                    actor,
                    session,
                    0,
                    CardinalStep::East,
                    &Default::default(),
                    None,
                    None,
                    Some(field)
                )
                .is_ok()
        );
        assert_eq!(states.deaths.len(), 1);
        assert_eq!(
            states
                .get(&runtime, actor, session)
                .unwrap()
                .vitals()
                .health,
            0
        );
        assert!(
            states
                .get(&runtime, actor, session)
                .unwrap()
                .owned_conditions()
                .instances()
                .is_empty()
        );
    }
}
