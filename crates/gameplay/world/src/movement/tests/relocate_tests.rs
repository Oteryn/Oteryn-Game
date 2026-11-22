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
        ),
        Err(MovementError::Blocked)
    ));
    Ok(())
}

#[test]
fn bundle_step_walkthrough_only_for_voluntary_players_in_protection_zone()
-> Result<(), Box<dyn Error>> {
    use crate::foundation::{CharacterId, CharacterLease, MovementFacing, NpcPlacement};
    use crate::map::boot::tests::{floor_items, floor_pins, teleport_bundle};
    use crate::map::floor::prepare_bundle_step;

    let world = WorldId::decode(&uuid_v7(20))?;
    let channel = ChannelId::decode(&uuid_v7(21))?;
    let room = crate::content::qualify_native_entry_room(world)?;
    let (bytes, pins) = teleport_bundle();
    let booted = crate::map::boot::boot(&bytes, &floor_pins(pins), world, channel, floor_items)?;
    let cells = booted.movement_cells(room.movement_cells())?;
    let closed = std::collections::BTreeSet::new();
    let step = |runtime: &ChannelRuntimeV1, actor, session, direction| {
        prepare_bundle_step(
            runtime,
            &cells,
            actor,
            session,
            runtime
                .read_actor_position(actor)
                .map_err(MovementError::Actor)?,
            direction,
            false,
            &closed,
        )
    };
    let admit = |runtime: &mut ChannelRuntimeV1, id| -> Result<_, Box<dyn Error>> {
        let session = GameSessionId::decode(&uuid_v7(id))?;
        let reservation = runtime.reserve_fresh_session(session)?;
        let actor = runtime.commit_fresh_session(reservation)?;
        runtime.initialize_first_entry_position(actor)?;
        runtime.bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(CharacterId::decode(&uuid_v7(id))?, 1)?,
        )?;
        Ok((actor, session))
    };

    for (occupant, in_pz, forced) in [
        ("player", true, true),
        ("player", false, false),
        ("monster", true, false),
        ("npc", true, false),
        ("player", true, false),
    ] {
        let pin = oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::from_activation(
            world, 1, cells.scope().generation_digest, [2; 32], [3; 32], [4; 32], (0, 0, 8),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            NodeId::decode(&uuid_v7(22))?,
            7,
            3,
            11,
            "runtime-scope-assignment:11",
            3,
            pin,
        )?;
        let (actor, session) = admit(&mut runtime, 30)?;
        let teleport = step(&runtime, actor, session, CardinalStep::South)?;
        let pz = runtime.commit_source_step(teleport)?.position();
        let north = step(&runtime, actor, session, CardinalStep::North)?;
        let outside = runtime.commit_source_step(north)?.position();
        let destination = if in_pz { pz } else { outside };
        let guard = match occupant {
            "player" => {
                let (other, other_session) = admit(&mut runtime, 31)?;
                let teleport = step(&runtime, other, other_session, CardinalStep::South)?;
                let other_at = runtime.commit_source_step(teleport)?;
                if !in_pz {
                    // Swap the roles to test the step north into the occupied non-PZ tile.
                    assert!(matches!(
                        step(&runtime, other, other_session, CardinalStep::North),
                        Err(MovementError::Blocked)
                    ));
                    continue;
                }
                if forced {
                    let actor_at = runtime.read_actor_position(actor)?;
                    Some(runtime.prepare_player_push(
                        (other, other_session, other_at),
                        (actor, session, actor_at),
                        destination,
                    )?)
                } else {
                    None
                }
            }
            "monster" => {
                runtime.admit_pinned_test_creature(destination)?;
                None
            }
            _ => {
                runtime
                    .place_dynamic_npc(
                        "oteryn:npc.walkthrough-test",
                        NpcPlacement {
                            floor: -8,
                            y: 1,
                            x: 5,
                            direction: MovementFacing::North,
                        },
                    )
                    .map_err(|error| format!("place blocking NPC: {error:?}"))?;
                None
            }
        };
        let result = step(&runtime, actor, session, CardinalStep::South);
        if occupant == "player" && !forced {
            assert_eq!(runtime.commit_source_step(result?)?.position(), destination);
        } else {
            assert!(
                matches!(result, Err(MovementError::Blocked)),
                "{occupant}, forced={forced}"
            );
        }
        drop(guard);
    }
    Ok(())
}
