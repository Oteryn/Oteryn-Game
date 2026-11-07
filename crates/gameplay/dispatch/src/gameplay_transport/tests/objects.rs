#[test]
fn use_object_reachable_is_chebyshev_one_same_floor_only() {
    let door =
        std::collections::BTreeSet::from([crate::content::LogicalCell { x: 1, y: -1, z: 0 }]);
    // Same cell, every adjacent (including diagonal) cell, and up to distance 1 inclusive.
    for (x, y) in [
        (1, -1),
        (0, -1),
        (2, -1),
        (1, 0),
        (1, -2),
        (0, 0),
        (2, 0),
        (0, -2),
        (2, -2),
    ] {
        assert!(
            ComposedFreshAdmission::use_object_reachable(x, y, 0, &door),
            "({x},{y}) expected reachable"
        );
    }
    // Distance 2 on either axis: TOO_FAR.
    for (x, y) in [(3, -1), (1, 1), (-1, -1), (1, -3)] {
        assert!(
            !ComposedFreshAdmission::use_object_reachable(x, y, 0, &door),
            "({x},{y}) expected too far"
        );
    }
    // The same (x, y) on a different floor is never reachable.
    assert!(!ComposedFreshAdmission::use_object_reachable(
        1, -1, 1, &door
    ));
    // No collision cell at all (an unbound/empty door) is never reachable.
    assert!(!ComposedFreshAdmission::use_object_reachable(
        1,
        -1,
        0,
        &std::collections::BTreeSet::new()
    ));
}

/// #162 5868482467 shared-lease P1 repair (r4121956127, Codex): the occupancy set
/// `use_object` builds must contain every committed actor's cell, not only the issuing
/// actor's own. Two real actors in one real `ChannelRuntimeV1`: actor A stands in the door
/// cell, actor B stands adjacent (the accepted `east` cell) and is the one attempting to
/// close the door. `ChannelRuntimeV1::committed_player_positions` (the new read this repair
/// adds, reusing the existing per-slot position store) must report both, and the real door
/// `LocalObjectRuntime` (bound exactly as `node/serve.rs` binds it) must then report
/// OCCUPIED for the close attempt, with no transition and no revision change.
#[test]
fn use_object_occupancy_includes_every_committed_actor_not_only_the_issuer()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::content::LogicalCell;
    use crate::content::accepted;
    use crate::world_runtime::{
        LocalObjectUseOutcome, ReferenceContentGeneration, ScopeContentGenerationFence,
        bind_native_entry_door,
    };
    use {
        crate::foundation::{MovementLocalPosition, NodeId},
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
    };

    let world_id = WorldId::decode(&uuid_v7(0x50)).expect("world");
    let channel_id = ChannelId::decode(&uuid_v7(0x51)).expect("channel");
    let node_id = NodeId::decode(&uuid_v7(0x52)).expect("node");
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world_id,
        channel_id,
        node_id,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        ChannelContentPin::test(world_id),
    )
    .expect("channel runtime");

    let session_a = GameSessionId::decode(&uuid_v7(0x53)).expect("session a");
    let session_b = GameSessionId::decode(&uuid_v7(0x54)).expect("session b");
    let reservation_a = runtime.reserve_fresh_session(session_a).expect("reserve a");
    let actor_a = runtime
        .commit_fresh_session(reservation_a)
        .expect("commit a");
    let reservation_b = runtime.reserve_fresh_session(session_b).expect("reserve b");
    let actor_b = runtime
        .commit_fresh_session(reservation_b)
        .expect("commit b");

    // Actor A stands in the door cell; actor B stands adjacent (accepted `east`). Both
    // start at the pinned first-entry cell (0,0,0) under the runtime's real pinned
    // context, then step by real cardinal commits — `initialize_movement_test_position`
    // deliberately uses a synthetic context that would not match the pinned one
    // `committed_player_positions` filters by.
    runtime
        .initialize_first_entry_position(actor_a)
        .expect("first entry a");
    runtime
        .initialize_first_entry_position(actor_b)
        .expect("first entry b");
    let door_cell = accepted::DOOR_CELL;
    {
        let mut position = runtime.borrow_movement_position();
        let snapshot = position.read(actor_a).expect("read a at start");
        let snapshot = position
            .commit_cardinal(
                snapshot,
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
            )
            .expect("step a east");
        position
            .commit_cardinal(
                snapshot,
                MovementLocalPosition {
                    x: door_cell.1,
                    y: door_cell.2,
                    floor: door_cell.3,
                },
            )
            .expect("step a north into the door cell");
    }
    let east = accepted::CELLS[1];
    {
        let mut position = runtime.borrow_movement_position();
        let snapshot = position.read(actor_b).expect("read b at start");
        position
            .commit_cardinal(
                snapshot,
                MovementLocalPosition {
                    x: east.1,
                    y: east.2,
                    floor: east.3,
                },
            )
            .expect("step b east");
    }

    // The exact read `use_object` performs already includes both actors, not only
    // whichever one happens to be the issuer.
    let occupied: std::collections::BTreeSet<LogicalCell> = runtime
        .committed_player_positions()
        .into_iter()
        .map(|position| LogicalCell {
            x: position.x,
            y: position.y,
            z: i32::from(position.floor),
        })
        .collect();
    assert_eq!(occupied.len(), 2);
    let door_logical_cell = LogicalCell {
        x: door_cell.1,
        y: door_cell.2,
        z: i32::from(door_cell.3),
    };
    assert!(occupied.contains(&door_logical_cell));
    assert!(occupied.contains(&LogicalCell {
        x: east.1,
        y: east.2,
        z: i32::from(east.3),
    }));

    // Bind the real door runtime from the real qualified native entry room content, exactly
    // as `node/serve.rs` does at activation, then open it (opening's target collision is
    // Absent, so it is never occupancy-checked) so a subsequent close can be.
    let room = crate::content::qualify_native_entry_room(world_id).expect("qualified room");
    let door_content = room.door().clone();
    let scope = RuntimeScopeRefV1::channel(world_id, channel_id);
    let scope_generation = ScopeOwnershipGeneration::new(1).expect("scope generation");
    let content_generation = ReferenceContentGeneration::from_content(&door_content)
        .map_err(|error| format!("{error:?}"))?;
    let fence = ScopeContentGenerationFence::for_activation(
        scope,
        scope_generation,
        content_generation,
    );
    let mut door = bind_native_entry_door(&door_content, &fence, scope, scope_generation)
        .map_err(|error| format!("{error:?}"))?;
    let empty = std::collections::BTreeSet::new();
    let opened = door
        .attempt_use(0, &empty)
        .map_err(|error| format!("{error:?}"))?;
    assert!(matches!(
        opened,
        LocalObjectUseOutcome::Committed { revision: 1, .. }
    ));

    // Actor B (adjacent, the issuer) attempts to close the door while actor A is standing
    // in the doorway: OCCUPIED, no transition, no revision change.
    assert_eq!(
        door.attempt_use(1, &occupied)
            .map_err(|error| format!("{error:?}"))?,
        LocalObjectUseOutcome::Occupied
    );
    assert_eq!(door.revision(), 1);
    assert_eq!(door.state_key().as_str(), accepted::DOOR_OPEN_STATE);
    Ok(())
}
