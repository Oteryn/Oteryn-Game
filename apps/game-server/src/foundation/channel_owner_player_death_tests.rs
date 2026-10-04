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
