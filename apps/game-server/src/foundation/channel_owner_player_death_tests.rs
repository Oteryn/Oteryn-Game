//! DEATH-2 (first player death decision §4.5): the Channel owner places a respawning player at
//! its admitted entry spawn, or at the recorded respawn position an admission consumes, keeping
//! the actor reference.
use super::*;

fn uuid_v7(tag: u8) -> [u8; 16] {
    [
        0x01, 0x90, 0x00, 0x00, 0x00, tag, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, tag,
    ]
}

/// A runtime whose pinned entry start is (3, 4, 7), with one player positioned at (10, 10, 7).
fn owner() -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
    let world = WorldId::decode(&uuid_v7(0x60)).expect("world");
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&uuid_v7(0x61)).expect("channel"),
        NodeId::decode(&uuid_v7(0x62)).expect("node"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        ChannelContentPin::from_activation(world, 1, [1; 32], [2; 32], [3; 32], [4; 32], (3, 4, 7)),
    )
    .expect("runtime");
    let session = GameSessionId::decode(&uuid_v7(0x21)).expect("session");
    let reservation = runtime.reserve_fresh_session(session).expect("reserve");
    let player = runtime.commit_fresh_session(reservation).expect("commit");
    runtime
        .initialize_movement_test_position(
            player,
            MovementLocalPosition {
                x: 10,
                y: 10,
                floor: 7,
            },
        )
        .expect("player position");
    (runtime, player, session)
}

#[test]
fn a_respawn_places_the_same_actor_at_the_entry_spawn_as_one_position_successor() {
    let (mut runtime, player, session) = owner();
    let entry = MovementLocalPosition {
        x: 3,
        y: 4,
        floor: 7,
    };
    assert_eq!(runtime.respawn_position(), entry);
    assert_eq!(runtime.map_revision_digest(), [4; 32]);
    let before = runtime.read_actor_position(player).expect("position");
    let placed = runtime
        .place_respawned_player(player, session, runtime.respawn_position())
        .expect("respawn placement");
    assert_eq!(placed.position(), entry);
    assert_eq!(placed.context(), before.context());
    let after = runtime.read_actor_position(player).expect("same actor");
    assert_eq!(after.position(), entry);
    assert_eq!(after.0.version.revision, before.0.version.revision + 1);
    assert!(runtime.player_control_facts(player, session).is_ok());
}

#[test]
fn a_respawn_of_another_session_or_a_stale_actor_moves_nothing() {
    let (mut runtime, player, _) = owner();
    let before = runtime.read_actor_position(player).expect("position");
    let other = GameSessionId::decode(&uuid_v7(0x22)).expect("session");
    let entry = runtime.respawn_position();
    assert!(
        runtime
            .place_respawned_player(player, other, entry)
            .is_err()
    );
    let unpositioned_session = GameSessionId::decode(&uuid_v7(0x23)).expect("session");
    let reservation = runtime
        .reserve_fresh_session(unpositioned_session)
        .expect("reserve");
    let unpositioned = runtime.commit_fresh_session(reservation).expect("commit");
    assert!(
        runtime
            .place_respawned_player(unpositioned, unpositioned_session, entry)
            .is_err()
    );
    assert_eq!(
        runtime.read_actor_position(player).expect("position"),
        before
    );
}

/// DEATH-2b: a fresh admission places its actor at the recorded respawn position it consumes,
/// which need not be the current entry start, as the same kind of position successor.
#[test]
fn an_admission_places_the_actor_at_its_recorded_respawn_position() {
    let (mut runtime, player, session) = owner();
    let recorded = MovementLocalPosition {
        x: 5,
        y: 6,
        floor: 7,
    };
    let before = runtime.read_actor_position(player).expect("position");
    let placed = runtime
        .place_respawned_player(player, session, recorded)
        .expect("recorded placement");
    assert_eq!(placed.position(), recorded);
    let after = runtime.read_actor_position(player).expect("same actor");
    assert_eq!(after.position(), recorded);
    assert_eq!(after.0.version.revision, before.0.version.revision + 1);
}

fn at(x: i32, y: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor: 7 }
}

/// NPC-0 §6.1: the recorded cell, then each ring out to Chebyshev 3 from its north cell
/// clockwise, on the same floor.
#[test]
fn the_respawn_fallback_spirals_from_north_clockwise_out_to_three() {
    let cells: Vec<_> = respawn_fallback_cells(at(10, 10)).collect();
    assert_eq!(cells.len(), 1 + 8 + 16 + 24);
    assert_eq!(
        cells[..9],
        [
            at(10, 10),
            at(10, 9),
            at(11, 9),
            at(11, 10),
            at(11, 11),
            at(10, 11),
            at(9, 11),
            at(9, 10),
            at(9, 9),
        ]
    );
    assert_eq!(cells[9], at(10, 8));
    assert_eq!(cells[48], at(9, 7));
    let unique: std::collections::BTreeSet<_> = cells.iter().map(|c| (c.x, c.y)).collect();
    assert_eq!(unique.len(), cells.len());
    assert!(
        cells
            .iter()
            .all(|c| c.floor == 7
                && (c.x - 10).abs().max((c.y - 10).abs()) <= RESPAWN_FALLBACK_RADIUS)
    );
}

/// CHAR-POSITION-0 §3.3: an admitted respawn whose recorded cell is valid and free is placed
/// there.
#[test]
fn an_admitted_respawn_uses_a_valid_free_recorded_cell() {
    let (mut runtime, player, session) = owner();
    let placed = runtime
        .place_admitted_respawn(player, session, Some(at(20, 20)), |_| true)
        .expect("placement");
    assert_eq!(placed.position(), at(20, 20));
}

/// CHAR-POSITION-0 §3.3: a blocked or removed recorded cell is not a refusal; the actor is
/// placed at the nearest valid free cell of the §6.1 spiral.
#[test]
fn a_blocked_recorded_cell_falls_back_to_the_nearest_valid_cell() {
    let (mut runtime, player, session) = owner();
    let walkable = [at(21, 20), at(20, 21)];
    let placed = runtime
        .place_admitted_respawn(player, session, Some(at(20, 20)), |cell| {
            walkable.contains(&cell)
        })
        .expect("placement");
    // East (21, 20) precedes south (20, 21) in the ring's clockwise order.
    assert_eq!(placed.position(), at(21, 20));
    assert_eq!(
        runtime
            .read_actor_position(player)
            .expect("position")
            .position(),
        at(21, 20)
    );
}

/// CHAR-POSITION-0 §3.3: a valid recorded cell another live actor occupies falls back the same
/// way, and the player's own current cell counts as free.
#[test]
fn an_occupied_recorded_cell_falls_back_and_the_own_cell_is_free() {
    let (mut runtime, player, session) = owner();
    let other_session = GameSessionId::decode(&uuid_v7(0x24)).expect("session");
    let reservation = runtime
        .reserve_fresh_session(other_session)
        .expect("reserve");
    let other = runtime.commit_fresh_session(reservation).expect("commit");
    runtime
        .initialize_movement_test_position(other, at(20, 20))
        .expect("other position");
    let placed = runtime
        .place_admitted_respawn(player, session, Some(at(20, 20)), |_| true)
        .expect("placement");
    assert_eq!(placed.position(), at(20, 19), "north of the occupied cell");
    let placed = runtime
        .place_admitted_respawn(player, session, Some(at(20, 19)), |_| true)
        .expect("placement");
    assert_eq!(placed.position(), at(20, 19), "its own cell");
}

/// CHAR-POSITION-0 §3.3 step 4: with no valid free cell within the radius, or no decodable
/// recorded cell, the actor is placed at the respawn position.
#[test]
fn no_fallback_cell_places_the_actor_at_the_respawn_position() {
    let (mut runtime, player, session) = owner();
    let far = at(20 + RESPAWN_FALLBACK_RADIUS + 1, 20);
    let placed = runtime
        .place_admitted_respawn(player, session, Some(at(20, 20)), |cell| cell == far)
        .expect("placement");
    assert_eq!(placed.position(), runtime.respawn_position());
    let (mut runtime, player, session) = owner();
    let placed = runtime
        .place_admitted_respawn(player, session, None, |_| true)
        .expect("placement");
    assert_eq!(placed.position(), runtime.respawn_position());
}

/// A stale actor or another session moves nothing, whatever the recorded cell.
#[test]
fn an_admitted_respawn_of_another_session_moves_nothing() {
    let (mut runtime, player, _) = owner();
    let before = runtime.read_actor_position(player).expect("position");
    let other = GameSessionId::decode(&uuid_v7(0x22)).expect("session");
    assert!(
        runtime
            .place_admitted_respawn(player, other, Some(at(20, 20)), |_| true)
            .is_err()
    );
    assert_eq!(
        runtime.read_actor_position(player).expect("position"),
        before
    );
}
