use super::{
    TestResult, death_ml_loss_composed_fixtures,
    fixture::{debug, id},
    qc_pinned_world,
};
use crate::content::{NativeEntryMovementCells, native_cell_lookup::NativeMovementCollisionIndex};
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
    NodeId, WorldId,
};
use crate::gameplay_transport::{
    ChannelSpellStates, actor_spell::actor_movement::step_in_channel, map_door,
    paced_movement::player_bundle_doors,
};
use crate::map::{
    boot::{BundleCollisionIndexExt, BundleWorld},
    overlay::TilePos,
    view::placement_key,
};
use crate::movement::{CardinalStep, MovementError};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_simulation_determinism::SemanticTimeMicros;

const GATES: [(u16, u16, i8, u16); 8] = [
    (32212, 32435, -10, 1648),
    (32874, 31974, -12, 5102),
    (32875, 31974, -12, 5102),
    (32234, 32140, -9, 6262),
    (32554, 32860, -9, 5291),
    (32909, 31074, -7, 1680),
    (32910, 31088, -7, 1678),
    (33775, 31699, -8, 9565),
];

fn world(positions: &[(u16, u16)]) -> TestResult<BundleWorld> {
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("native Item generation")?;
    qc_pinned_world::world_near(native, positions)
}

fn target(
    world: &BundleWorld,
    tile: TilePos,
    appearance: u16,
) -> TestResult<(u64, WorldObjectTarget)> {
    let source = world
        .base()
        .tile(tile.x, tile.y, tile.floor)
        .ok_or("source gate tile")?;
    let ordinal = qc_pinned_world::ordinal(source, appearance, None)?;
    let key = placement_key(tile, ordinal).ok_or("source gate placement")?;
    assert_eq!(world.facts().action_at(key), Some(1050));
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok((
        key,
        WorldObjectTarget {
            placement,
            expected_revision: 0,
        },
    ))
}

fn local(tile: TilePos) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(tile.x),
        y: i32::from(tile.y),
        floor: -i16::from(tile.floor),
    }
}

fn owner(
    world: &BundleWorld,
    start: TilePos,
) -> TestResult<(ChannelRuntimeV1, NativeEntryMovementCells)> {
    let world_id = WorldId::decode(&id(42)).map_err(debug)?;
    let room = crate::content::qualify_native_entry_room(world_id).map_err(debug)?;
    let cells = world.movement_cells(room.movement_cells()).map_err(debug)?;
    let start = local(start);
    let pin = ChannelContentPin::from_activation(
        world_id,
        1,
        room.compiled().server_digest(),
        room.compiled().client_digest(),
        world.frame_binding_digest(world_id),
        world.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let runtime = ChannelRuntimeV1::from_committed_assignment(
        world_id,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        NodeId::decode(&id(3)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        pin,
    )
    .map_err(debug)?;
    Ok((runtime, cells))
}

fn add_player(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    tag: u8,
    level: u32,
    tile: TilePos,
) -> TestResult<(ExactActorRef, GameSessionId)> {
    let session = GameSessionId::decode(&id(tag)).map_err(debug)?;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(actor, local(tile))
        .map_err(debug)?;
    states
        .initialize(
            runtime,
            actor,
            session,
            crate::spell::cast::CharacterCastFacts {
                vocation: crate::spell::Vocation::Knight,
                level,
                magic_level: 0,
                max_health: 1000,
                max_mana: 500,
                max_soul: 100,
            },
            (0, 0),
            SemanticTimeMicros::from_micros(0),
        )
        .ok_or("initialize live character level")?;
    Ok((actor, session))
}

#[test]
fn authored_level_fifty_gates_boot_and_toggle_with_pinned_item_profiles() -> TestResult {
    let world = world(&GATES.map(|(x, y, _, _)| (x, y)))?;
    for (x, y, floor, appearance) in GATES {
        let tile = TilePos { x, y, floor };
        let (key, target) = target(&world, tile, appearance)?;
        let adjacent = [-1_i16, 0, 1]
            .into_iter()
            .flat_map(|dx| {
                [-1_i16, 0, 1].map(move |dy| TilePos {
                    x: x.wrapping_add_signed(dx),
                    y: y.wrapping_add_signed(dy),
                    floor,
                })
            })
            .find(|&pos| pos != tile && world.enterable(pos))
            .ok_or("enterable adjacent tile")?;
        for (tag, level, occupied, expected, position, revision) in [
            (4, 49, false, UseDisposition::Locked, adjacent, 0),
            (5, 50, true, UseDisposition::NotPossible, adjacent, 0),
            (6, 50, false, UseDisposition::Committed, tile, 1),
        ] {
            let (mut runtime, cells) = owner(&world, adjacent)?;
            let NativeMovementCollisionIndex::Bundle(index) = cells.index() else {
                return Err("expected bundle collision index".into());
            };
            let mut states = ChannelSpellStates::default();
            let (actor, session) = add_player(&mut runtime, &mut states, tag, level, adjacent)?;
            if occupied {
                add_player(&mut runtime, &mut states, 7, level, tile)?;
            }
            let before = runtime.read_actor_position(actor).map_err(debug)?;
            let result = map_door::use_owned_door(
                index.map(),
                &cells,
                &mut runtime,
                actor,
                session,
                before,
                adjacent,
                &states,
                false,
                SemanticTimeMicros::from_micros(0),
                &target,
                || true,
            );
            assert_eq!(
                result.disposition, expected,
                "{tile:?} appearance {appearance} at level {level}"
            );
            assert_eq!(
                runtime
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                local(position)
            );
            assert_eq!(world.facts().doors().revision(key), revision);
            assert_eq!(world.enterable(tile), revision != 0, "{tile:?}");
        }
        assert!(world.enterable(tile), "{tile:?}");
    }
    Ok(())
}

#[test]
fn open_level_gate_checks_the_entering_player() -> TestResult {
    let (x, y, floor, appearance) = GATES[1];
    let world = world(&[(x, y)])?;
    let tile = TilePos { x, y, floor };
    let adjacent = TilePos { y: y - 1, ..tile };
    let opening_from = TilePos {
        x: x - 1,
        ..adjacent
    };
    let (key, target) = target(&world, tile, appearance)?;
    let (mut runtime, cells) = owner(&world, adjacent)?;
    let NativeMovementCollisionIndex::Bundle(index) = cells.index() else {
        return Err("expected bundle collision index".into());
    };
    let mut states = ChannelSpellStates::default();
    let (opener, opener_session) = add_player(&mut runtime, &mut states, 4, 50, opening_from)?;
    let (entrant, entrant_session) = add_player(&mut runtime, &mut states, 5, 49, adjacent)?;
    let before = runtime.read_actor_position(opener).map_err(debug)?;
    let result = map_door::use_owned_door(
        index.map(),
        &cells,
        &mut runtime,
        opener,
        opener_session,
        before,
        opening_from,
        &states,
        false,
        SemanticTimeMicros::from_micros(0),
        &target,
        || true,
    );
    assert_eq!(result.disposition, UseDisposition::Committed);
    assert_eq!(
        runtime
            .read_actor_position(opener)
            .map_err(debug)?
            .position(),
        local(tile)
    );

    let step = |runtime: &mut ChannelRuntimeV1,
                states: &mut ChannelSpellStates,
                actor,
                session,
                direction| {
        let blocking = player_bundle_doors(world.facts(), states, &owner.states, runtime, actor, session);
        step_in_channel(
            runtime, states, &cells, actor, session, 1, direction, &blocking, None,
        )
    };
    // Check the movement owner before the asynchronous step-out close. The
    // doorway is empty and visibly open when each entrant attempts the step.
    step(
        &mut runtime,
        &mut states,
        opener,
        opener_session,
        CardinalStep::South,
    )
    .map_err(debug)?;
    assert!(world.enterable(tile));
    let result = step(
        &mut runtime,
        &mut states,
        entrant,
        entrant_session,
        CardinalStep::South,
    );
    assert!(matches!(result, Err(MovementError::Blocked)), "{result:?}");
    assert_eq!(
        runtime
            .read_actor_position(entrant)
            .map_err(debug)?
            .position(),
        local(adjacent)
    );
    step(
        &mut runtime,
        &mut states,
        opener,
        opener_session,
        CardinalStep::North,
    )
    .map_err(debug)?;
    assert_eq!(
        runtime
            .read_actor_position(opener)
            .map_err(debug)?
            .position(),
        local(tile)
    );
    assert_eq!(world.facts().doors().revision(key), 1);
    Ok(())
}
