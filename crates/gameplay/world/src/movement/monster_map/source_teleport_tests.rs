//! The current-map transport primitive, not a callback/source-room authority grant.
use super::*;
use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
fn debug(value: impl std::fmt::Debug) -> String {
    format!("{value:?}")
}
fn id(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}
pub(super) fn fixture() -> Result<
    (ChannelRuntimeV1, MonsterMap, ExactActorRef, ExactActorRef),
    Box<dyn std::error::Error + Send + Sync>,
> {
    let world = WorldId::decode(&id(21)).map_err(debug)?;
    let pin = ChannelContentPin::test(world);
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(22)).map_err(debug)?,
        NodeId::decode(&id(23)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        pin,
    )
    .map_err(debug)?;
    let first = runtime
        .admit_pinned_monster_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 14,
            },
            "test:source-teleport",
            100,
        )
        .map_err(debug)?;
    let second = runtime
        .admit_pinned_monster_lab_creature(
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 13,
            },
            "test:source-reserved-form",
            100,
        )
        .map_err(debug)?;
    let pin = runtime.content_pin();
    let mut map = MonsterMap {
        valid: true,
        scope: runtime.binding().runtime_scope(),
        generation: runtime.binding().scope_generation(),
        context: runtime.pinned_movement_context(),
        activation: pin.activation_sequence(),
        server: pin.server_artifact_digest(),
        client: pin.client_artifact_digest(),
        frame: pin.frame_binding_digest(),
        map: pin.map_revision_digest(),
        tiles: BTreeMap::new(),
    };
    map.tiles.insert(
        (101, 100, 13),
        Some(Tile {
            admits: false,
            solid: true,
            source_spawn_admits: false,
            projectile: true,
            speed: 150,
            field: None,
            source_placement: SourcePlacementTerrain {
                exists: true,
                ground: false,
                immovable_solid: true,
                protection_zone: false,
                floor_change: false,
                teleport: false,
            },
            source_placement_proven: true,
        }),
    );
    Ok((runtime, map, first, second))
}

#[test]
fn srx3_source_teleport_uses_known_tile_and_canonical_stack_instead_of_walk_policy() -> TestResult {
    let (mut runtime, map, actor, reserved) = fixture()?;
    let destination = runtime
        .read_actor_position(reserved)
        .map_err(debug)?
        .position();
    assert!(!map.admits(destination));
    assert!(
        runtime
            .position_occupied_by_other(actor, destination)
            .map_err(debug)?
    );
    // Crystal game.cpp:3167-3195 uses FLAG_NOLIMIT; tile.cpp:601-608
    // accepts it before movement blockers, ground, occupants and summon policy.
    map.relocate_source_creature(&mut runtime, actor, destination)
        .map_err(debug)?;
    assert_eq!(
        runtime
            .current_top_actor_on_cell(destination)
            .map_err(debug)?,
        Some(actor)
    );
    assert_eq!(
        runtime
            .read_actor_position(reserved)
            .map_err(debug)?
            .position(),
        destination
    );
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    let same = map
        .relocate_source_creature(&mut runtime, actor, destination)
        .map_err(debug)?;
    assert_eq!(
        same.position(),
        before.position(),
        "literal no-op preserves coordinates"
    );
    assert_eq!(
        same.last_translation_revision(),
        before.last_translation_revision()
    );
    assert_eq!(
        same.facing(),
        Some(crate::foundation::MovementFacing::North),
        "default source teleport still turns north"
    );
    assert_eq!(
        runtime
            .current_top_actor_on_cell(destination)
            .map_err(debug)?,
        Some(actor)
    );
    Ok(())
}

#[test]
fn srx3_source_teleport_refuses_unknown_tile_and_stale_map_without_position_change() -> TestResult {
    let (mut runtime, mut map, actor, _) = fixture()?;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    assert!(
        map.relocate_source_creature(
            &mut runtime,
            actor,
            MovementLocalPosition {
                x: 999,
                y: 100,
                floor: 14
            }
        )
        .is_err()
    );
    assert_eq!(runtime.read_actor_position(actor).map_err(debug)?, before);
    let mut absent = map
        .tile(MovementLocalPosition {
            x: 101,
            y: 100,
            floor: 13,
        })
        .ok_or("cached tile")?;
    absent.source_placement.exists = false;
    map.tiles.insert((102, 100, 13), Some(absent));
    assert!(
        map.relocate_source_creature(
            &mut runtime,
            actor,
            MovementLocalPosition {
                x: 102,
                y: 100,
                floor: 13
            }
        )
        .is_err(),
        "a cached proof of ABSENT Tile is not source teleport admission"
    );
    assert_eq!(runtime.read_actor_position(actor).map_err(debug)?, before);
    map.valid = false;
    assert!(
        map.relocate_source_creature(
            &mut runtime,
            actor,
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 13
            }
        )
        .is_err()
    );
    assert_eq!(runtime.read_actor_position(actor).map_err(debug)?, before);
    Ok(())
}

#[test]
fn srx3_source_player_teleport_uses_exact_session_pose_and_current_map() -> TestResult {
    let (mut runtime, map, _, _) = fixture()?;
    let session = crate::foundation::GameSessionId::decode(&id(25)).map_err(debug)?;
    let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
    let initial = MovementLocalPosition {
        x: 100,
        y: 100,
        floor: 14,
    };
    runtime
        .initialize_movement_test_position(actor, initial)
        .map_err(debug)?;
    let destination = MovementLocalPosition {
        x: 101,
        y: 100,
        floor: 13,
    };
    let wrong = crate::foundation::GameSessionId::decode(&id(26)).map_err(debug)?;
    assert!(
        map.relocate_source_player(&mut runtime, actor, wrong, destination)
            .is_err()
    );
    assert_eq!(
        runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        initial
    );
    let moved = map
        .relocate_source_player(&mut runtime, actor, session, destination)
        .map_err(debug)?;
    assert_eq!(moved.position(), destination);
    assert_eq!(
        moved.facing(),
        Some(crate::foundation::MovementFacing::East)
    );
    assert!(
        map.relocate_source_player(
            &mut runtime,
            actor,
            session,
            MovementLocalPosition {
                x: 200,
                y: 200,
                floor: 13
            }
        )
        .is_err()
    );
    assert_eq!(
        runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        destination
    );
    Ok(())
}
