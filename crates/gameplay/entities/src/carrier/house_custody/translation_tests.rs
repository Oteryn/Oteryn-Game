//! House exit retains the existing position revision and translation-observation contract.
use super::*;
use oteryn_gameplay_foundation::foundation::{ChannelId, CharacterId, NodeId, WorldId};

fn id(seed: u8) -> [u8; 16] {
    [
        1, 2, 3, 4, 5, seed, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

#[test]
fn house_exit_updates_translation_stamp_only_for_coordinate_or_floor_changes() -> Result<(), String>
{
    let source_position = MovementLocalPosition {
        x: 2,
        y: 3,
        floor: 7,
    };
    for destination in [
        source_position,
        MovementLocalPosition {
            x: 3,
            ..source_position
        },
        MovementLocalPosition {
            floor: 8,
            ..source_position
        },
    ] {
        let world = WorldId::decode(&id(1)).map_err(debug)?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).map_err(debug)?,
            NodeId::decode(&id(3)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::test(world),
        )
        .map_err(debug)?;
        let source_session = GameSessionId::decode(&id(4)).map_err(debug)?;
        let house_session = GameSessionId::decode(&id(5)).map_err(debug)?;
        let returned_session = GameSessionId::decode(&id(6)).map_err(debug)?;
        let character = CharacterId::decode(&id(7)).map_err(debug)?;
        let house_scope = RuntimeScopeRefV1::instance(world, id(8)).map_err(debug)?;
        let house_generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
        let reserved = runtime
            .reserve_fresh_session(source_session)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
        let admitted = runtime
            .initialize_pinned_test_position(
                actor,
                MovementLocalPosition {
                    x: 1,
                    ..source_position
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                source_session,
                CharacterLease::new(character, 1).map_err(debug)?,
            )
            .map_err(debug)?;
        let moved = runtime
            .borrow_movement_position()
            .commit_cardinal(admitted, source_position)
            .map_err(debug)?;
        let before = runtime
            .borrow_movement_position()
            .commit_turn(moved, MovementFacing::North)
            .map_err(debug)?;
        assert_eq!(before.last_translation_revision(), moved.revision());
        assert_ne!(before.last_translation_revision(), before.revision());

        runtime
            .commit_house_custody(
                actor,
                source_session,
                house_session,
                CharacterLease::new(character, 2).map_err(debug)?,
                "test:house",
                house_scope,
                house_generation,
                LocalPosition {
                    x: 100,
                    y: 100,
                    floor: 2,
                },
                false,
            )
            .map_err(debug)?;
        assert!(
            runtime.read_actor_position(actor).is_err(),
            "House custody removes Channel presence"
        );
        runtime
            .commit_house_custody(
                actor,
                house_session,
                returned_session,
                CharacterLease::new(character, 3).map_err(debug)?,
                "test:house",
                house_scope,
                house_generation,
                LocalPosition {
                    x: destination.x,
                    y: destination.y,
                    floor: destination.floor,
                },
                true,
            )
            .map_err(debug)?;

        let after = runtime.read_actor_position(actor).map_err(debug)?;
        assert_eq!(after.context(), before.context());
        assert_eq!(after.position(), destination);
        assert_eq!(after.revision(), before.revision() + 1);
        assert_eq!(
            after.last_translation_revision(),
            if destination == source_position {
                before.last_translation_revision()
            } else {
                after.revision()
            }
        );
        assert_eq!(
            runtime
                .bound_attacker_lease(actor, returned_session)
                .map_err(debug)?
                .generation(),
            3
        );
    }
    Ok(())
}
