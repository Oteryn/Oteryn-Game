use super::*;
use crate::durability::item_mint::{GroundPlacement, TypedDefinitionRef};

fn uuid(n: u8) -> [u8; 16] {
    let mut value = [0; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value[15] = n;
    value
}

fn world(n: u8) -> WorldId {
    match WorldId::decode(&uuid(n)) {
        Ok(value) => value,
        Err(error) => unreachable!("valid fixture WorldId: {error:?}"),
    }
}

fn channel(n: u8) -> ChannelId {
    match ChannelId::decode(&uuid(n)) {
        Ok(value) => value,
        Err(error) => unreachable!("valid fixture ChannelId: {error:?}"),
    }
}

fn target() -> SpellGroundTarget {
    SpellGroundTarget {
        spatial_position: [
            120_i32.to_be_bytes().as_slice(),
            (-5_i32).to_be_bytes().as_slice(),
            (-1_i16).to_be_bytes().as_slice(),
        ]
        .concat(),
        map_revision: "map-a".into(),
        content_revision: "content-a".into(),
        placement_context: vec![9],
    }
}

fn root(n: u8) -> GroundItemInstance {
    let target = target();
    GroundItemInstance {
        item_instance_id: uuid(n),
        world_id: world(1),
        channel_id: channel(2),
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: "stack".into(),
            revision_ref: "revision-a".into(),
        },
        quantity: 100,
        runtime_scope_ownership_generation: 7,
        ground: GroundPlacement {
            spatial_position: target.spatial_position,
            corpse_ref: Vec::new(),
            map_revision: target.map_revision,
            content_revision: target.content_revision,
            native_room_placement_context: target.placement_context,
        },
        minted_transaction_id: uuid(n),
    }
}

fn roots(count: u8) -> Vec<GroundItemInstance> {
    (0..count).map(root).collect()
}

fn check(
    remaining: &[GroundItemInstance],
    inserted: &[GroundItemInstance],
) -> Result<(), GroundCellLimitError> {
    check_ground_cell_root_admission(world(1), channel(2), 7, &target(), remaining, inserted)
}

#[test]
fn accepts_exactly_64_physical_roots_and_refuses_the_65th() {
    let existing = roots(63);
    assert_eq!(check(&existing, &[root(63)]), Ok(()));
    assert_eq!(
        check(&roots(64), &[root(64)]),
        Err(GroundCellLimitError::NotEnoughRoom)
    );
    assert_eq!(check(&[], &roots(64)), Ok(()));
    assert_eq!(
        check(&[], &roots(65)),
        Err(GroundCellLimitError::NotEnoughRoom)
    );
    assert_eq!(
        check(&roots(65), &[]),
        Err(GroundCellLimitError::NotEnoughRoom)
    );
    assert_eq!(
        GroundCellLimitError::NotEnoughRoom.user_message(),
        Some("not enough room")
    );
    assert_eq!(GroundCellLimitError::IncompatibleInput.user_message(), None);
}

#[test]
fn a_stack_counts_once_and_native_merge_with_no_new_root_needs_no_room() {
    let mut existing = roots(64);
    existing[0].quantity = u32::MAX;
    assert_eq!(check(&existing, &[]), Ok(()));
    // Native merging changes quantity, not the number of Ground item instances.
    existing[1].quantity = 1;
    assert_eq!(check(&existing, &[]), Ok(()));
}

#[test]
fn actual_root_removal_leaves_room_for_atomic_replacement() {
    let mut remaining = roots(64);
    remaining.remove(0);
    assert_eq!(check(&remaining, &[root(64)]), Ok(()));
    assert_eq!(
        check(&remaining, &[root(64), root(65)]),
        Err(GroundCellLimitError::NotEnoughRoom)
    );
}

#[test]
fn refuses_duplicate_identity_within_or_across_native_row_sets() {
    assert_eq!(
        check(&[root(1), root(1)], &[]),
        Err(GroundCellLimitError::IncompatibleInput)
    );
    assert_eq!(
        check(&[root(1)], &[root(1)]),
        Err(GroundCellLimitError::IncompatibleInput)
    );
    assert_eq!(
        check(&[], &[root(1), root(1)]),
        Err(GroundCellLimitError::IncompatibleInput)
    );
}

#[test]
fn supplied_scope_and_generation_are_compared_independently() {
    let existing = roots(1);
    for (world, channel, generation) in [
        (world(3), channel(2), 7),
        (world(1), channel(3), 7),
        (world(1), channel(2), 8),
    ] {
        assert_eq!(
            check_ground_cell_root_admission(world, channel, generation, &target(), &existing, &[]),
            Err(GroundCellLimitError::IncompatibleInput)
        );
    }
    let mut inserted = root(2);
    inserted.channel_id = channel(3);
    assert_eq!(
        check(&existing, &[inserted]),
        Err(GroundCellLimitError::IncompatibleInput)
    );
}

#[test]
fn incompatible_tile_metadata_is_not_silently_filtered_out() {
    for mutate in [
        |item: &mut GroundItemInstance| item.ground.spatial_position[0] ^= 1,
        |item: &mut GroundItemInstance| item.ground.map_revision.push('b'),
        |item: &mut GroundItemInstance| item.ground.content_revision.push('b'),
        |item: &mut GroundItemInstance| item.ground.native_room_placement_context.push(1),
    ] {
        let mut item = root(0);
        mutate(&mut item);
        assert_eq!(
            check(&[item.clone()], &[]),
            Err(GroundCellLimitError::IncompatibleInput)
        );
        assert_eq!(
            check(&[], &[item]),
            Err(GroundCellLimitError::IncompatibleInput)
        );
    }
}

#[test]
fn opaque_or_truncated_target_is_never_an_empty_cell() {
    for length in [0, 9, 11] {
        let mut target = target();
        target.spatial_position.resize(length, 0);
        assert_eq!(
            check_ground_cell_root_admission(world(1), channel(2), 7, &target, &[], &[]),
            Err(GroundCellLimitError::IncompatibleInput)
        );
    }
}

#[test]
fn repeated_reload_evaluation_has_no_memory_or_mutation() {
    let existing = roots(63);
    let inserted = [root(63)];
    let original = (existing.clone(), inserted.clone());
    assert_eq!(check(&existing, &inserted), Ok(()));
    assert_eq!(check(&existing.clone(), &inserted.clone()), Ok(()));
    assert_eq!((existing, inserted), original);
    // A later independently read occupancy must be checked again.
    assert_eq!(
        check(&roots(64), &[root(64)]),
        Err(GroundCellLimitError::NotEnoughRoom)
    );
}
