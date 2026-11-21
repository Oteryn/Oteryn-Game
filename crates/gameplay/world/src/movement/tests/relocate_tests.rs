use super::*;

#[test]
fn relocation_carrier_refuses_players_and_stale_snapshots_without_mutation()
-> Result<(), Box<dyn Error>> {
    let mut runtime = runtime()?;
    let (player, before) = admitted_at(&mut runtime, 30, 4)?;
    assert!(
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(
                player,
                before,
                MovementLocalPosition {
                    x: 100,
                    y: 5,
                    floor: 7
                }
            )
            .is_err()
    );
    assert_eq!(runtime.read_actor_position(player)?, before);
    let creature = runtime.admit_pinned_test_creature(MovementLocalPosition {
        x: 6,
        y: 5,
        floor: 7,
    })?;
    let before = runtime.read_actor_position(creature)?;
    let next = MovementLocalPosition {
        x: 57,
        y: 5,
        floor: 7,
    };
    let after = runtime
        .borrow_movement_position()
        .commit_creature_relocation(creature, before, next)?;
    assert_eq!(after.position(), next);
    assert_eq!(after.revision(), before.revision() + 1);
    assert!(
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(creature, before, next)
            .is_err()
    );
    assert_eq!(runtime.read_actor_position(creature)?, after);
    Ok(())
}

#[test]
fn bundle_step_rechecks_creature_occupancy() -> Result<(), Box<dyn Error>> {
    let world = WorldId::decode(&uuid_v7(20))?;
    let room = crate::content::qualify_native_entry_room(world)?;
    let cells = crate::map::spell_tiles::tests::movement_arena::world()
        .movement_cells(room.movement_cells())?;
    let pin =
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::from_activation(
            world,
            1,
            cells.scope().generation_digest,
            [2; 32],
            [3; 32],
            [4; 32],
            (4, 5, 7),
        );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&uuid_v7(21))?,
        NodeId::decode(&uuid_v7(22))?,
        7,
        3,
        11,
        "runtime-scope-assignment:11",
        3,
        pin,
    )?;
    let session = GameSessionId::decode(&uuid_v7(30))?;
    let reserved = runtime.reserve_fresh_session(session)?;
    let actor = runtime.commit_fresh_session(reserved)?;
    runtime.initialize_first_entry_position(actor)?;
    let before = runtime.read_actor_position(actor)?;
    let closed = std::collections::BTreeSet::new();
    assert!(!runtime.actor_spell_reserved(actor));
    assert_eq!(
        cells.scope().generation_digest,
        runtime.content_pin().server_artifact_digest()
    );
    assert_eq!(before.context(), runtime.pinned_movement_context());
    assert!(
        runtime
            .player_control_facts(actor, session)
            .is_ok_and(|f| f.control_loss.is_none())
    );
    let proof = crate::map::floor::prepare_bundle_step(
        &runtime,
        &cells,
        actor,
        session,
        before,
        CardinalStep::East,
        false,
        &closed,
        |_, _| false,
    )?;
    runtime.admit_pinned_test_creature(proof.destination())?;
    assert!(runtime.commit_source_step(proof).is_err());
    assert_eq!(runtime.read_actor_position(actor)?, before);
    assert!(matches!(
        crate::map::floor::prepare_bundle_step(
            &runtime,
            &cells,
            actor,
            session,
            before,
            CardinalStep::East,
            false,
            &closed,
            |_, _| false,
        ),
        Err(MovementError::Blocked)
    ));
    Ok(())
}

#[test]
fn bundle_step_can_walk_through_a_player_on_a_protection_zone_tile() -> Result<(), Box<dyn Error>> {
    use crate::map::boot::tests::{floor_items, floor_pins, teleport_bundle};

    let world = WorldId::decode(&uuid_v7(20))?;
    let channel = ChannelId::decode(&uuid_v7(21))?;
    let room = crate::content::qualify_native_entry_room(world)?;
    let (bytes, pins) = teleport_bundle();
    let booted = crate::map::boot::boot(&bytes, &floor_pins(pins), world, channel, floor_items)?;
    let cells = booted.movement_cells(room.movement_cells())?;
    let pin =
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::from_activation(
            world,
            1,
            cells.scope().generation_digest,
            [2; 32],
            [3; 32],
            [4; 32],
            (5, 0, 8),
        );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        NodeId::decode(&uuid_v7(22))?,
        7,
        3,
        11,
        "runtime-scope-assignment:11",
        2,
        pin,
    )?;
    let session = GameSessionId::decode(&uuid_v7(30))?;
    let reserved = runtime.reserve_fresh_session(session)?;
    let actor = runtime.commit_fresh_session(reserved)?;
    runtime.initialize_first_entry_position(actor)?;
    let before = runtime.read_actor_position(actor)?;
    let peer_session = GameSessionId::decode(&uuid_v7(31))?;
    let reserved = runtime.reserve_fresh_session(peer_session)?;
    let peer = runtime.commit_fresh_session(reserved)?;
    let destination = MovementLocalPosition {
        x: 5,
        y: 1,
        floor: 8,
    };
    let peer_before = runtime.initialize_pinned_test_position(peer, destination)?;
    let closed = std::collections::BTreeSet::new();

    let proof = crate::map::floor::prepare_bundle_step(
        &runtime,
        &cells,
        actor,
        session,
        before,
        CardinalStep::South,
        false,
        &closed,
        |_, _| false,
    );
    assert!(proof.is_ok(), "player STEP refused: {:?}", proof.err());
    let after = runtime.commit_source_step(proof?)?;
    assert_eq!(after.position(), destination);
    assert_eq!(runtime.read_actor_position(peer)?, peer_before);
    Ok(())
}
