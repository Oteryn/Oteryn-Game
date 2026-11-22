// GAP-SV8: readonly item probes qualify absent bundle air without authorizing placement.
use super::*;
use crate::durability::native_world_items::SpellGroundTarget;

#[test]
fn bundle_air_read_binds_the_generation_and_preserves_the_placement_boundary() {
    let entry = room(1);
    let world_id = entry.movement_cells().scope().world_id;
    let world = crate::map::spell_tiles::tests::vertical_movement::world(
        world_id,
        ChannelId::decode(&uuid(2)).unwrap(),
    );
    let bundle = entry
        .bundle_view(
            world.movement_cells(entry.movement_cells()).unwrap(),
            world.frame_binding_digest(world_id),
            world.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: 10,
                y: 10,
                floor: 6,
            },
        )
        .unwrap();
    let current = runtime(&bundle, false);
    let air = MovementLocalPosition {
        x: 10,
        y: 10,
        floor: 5,
    };
    let address = SpellGroundTarget::for_native_tile_read(&bundle, &current, air)
        .expect("a read of proven absent bundle air");
    assert_eq!(
        address.spatial_position,
        [
            10_i32.to_be_bytes().as_slice(),
            10_i32.to_be_bytes().as_slice(),
            5_i16.to_be_bytes().as_slice()
        ]
        .concat()
    );
    assert!(SpellGroundTarget::from_native_owner(&bundle, &current, air).is_err());
    assert!(
        SpellGroundTarget::for_native_tile_read(&bundle, &runtime(&bundle, true), air).is_err()
    );
    for position in [
        MovementLocalPosition { x: -1, ..air },
        MovementLocalPosition { x: 65536, ..air },
        MovementLocalPosition { floor: -1, ..air },
        MovementLocalPosition { floor: 16, ..air },
    ] {
        assert!(SpellGroundTarget::for_native_tile_read(&bundle, &current, position).is_err());
    }
}

#[test]
fn area_cues_qualify_known_absent_cells_without_granting_ground_placement() {
    use crate::spell::spell_presentations::QualifiedCueTile;
    let entry = room(1);
    let world_id = entry.movement_cells().scope().world_id;
    let world = crate::map::spell_tiles::tests::vertical_movement::world(
        world_id,
        ChannelId::decode(&uuid(2)).unwrap(),
    );
    let bundle = entry
        .bundle_view(
            world.movement_cells(entry.movement_cells()).unwrap(),
            world.frame_binding_digest(world_id),
            world.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: 10,
                y: 10,
                floor: 6,
            },
        )
        .unwrap();
    let current = runtime(&bundle, false);
    let air = MovementLocalPosition {
        x: 10,
        y: 10,
        floor: 5,
    };
    assert!(QualifiedCueTile::from_known_absent_cell(&bundle, &current, air).is_ok());
    assert!(QualifiedCueTile::from_owners(&bundle, &current, air).is_ok());
    assert!(SpellGroundTarget::from_native_owner(&bundle, &current, air).is_err());
    assert!(QualifiedCueTile::from_owners(&bundle, &runtime(&bundle, true), air).is_err());
    for position in [
        MovementLocalPosition { x: -1, ..air },
        MovementLocalPosition { x: 65536, ..air },
        MovementLocalPosition { floor: -1, ..air },
        MovementLocalPosition { floor: 16, ..air },
    ] {
        assert!(QualifiedCueTile::from_owners(&bundle, &current, position).is_err());
    }
    let source = source_world_room();
    let current = runtime(&source, false);
    assert!(
        QualifiedCueTile::from_owners(
            &source,
            &current,
            MovementLocalPosition {
                x: i32::MAX,
                y: i32::MAX,
                floor: 0
            }
        )
        .is_err()
    );
}
