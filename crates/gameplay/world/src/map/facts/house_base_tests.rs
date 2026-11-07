#![allow(clippy::expect_used)]
use super::*;

#[test]
fn w19_house_base_facts_exclude_channel_door_rotation_and_callback_overrides() {
    let pos = TilePos {
        x: 6,
        y: 0,
        floor: -7,
    };
    let key = placement_key(pos, 0).expect("placement");
    let before = EntryFacts {
        definition: MapDefinition::Item(NonZeroU32::MIN),
        terrain_kind: None,
        appearance_id: 2358,
        blocks_projectile: false,
        pickupable: false,
        bound: false,
        count: 1,
        sub_type: 0,
    };
    let mut facts = BundleFacts::default();
    facts.palette.push(PaletteFacts {
        static_surface: false,
        family: Family::Item,
        id: 17,
        facts: before,
        solid: false,
        door: PaletteDoor::No,
    });
    facts.placements.push((
        key,
        Placement {
            palette: 0,
            bound: false,
            dropped: false,
            count: 1,
            sub_type: 0,
            attrs: Attrs::default(),
            pickup: Err(pickup::Ineligible::NotPickupable),
        },
    ));
    let opened = EntryFacts {
        appearance_id: 2359,
        ..before
    };
    facts
        .doors
        .push(key, true, false, opened, false, None, None, None);
    facts.doors.finish();
    assert_eq!(
        facts.doors.toggle(key, 0, false, 1, true),
        Some(door::DoorUse::Toggled {
            revision: 1,
            open: true
        })
    );
    assert_eq!(facts.pinned().base_entry(pos, 0, 17), Some(opened));
    assert_eq!(facts.pinned().object_revision(key), 1);
    let check_base = |facts: &BundleFacts| {
        let base = facts.base_only();
        assert_eq!(base.base_entry(pos, 0, 17), Some(before));
        assert!(!base.base_hidden(key));
        assert_eq!(base.object_revision(key), 0);
    };
    check_base(&facts);
    let rotated = EntryFacts {
        appearance_id: 2360,
        ..before
    };
    Arc::make_mut(&mut facts.rotations.lock().expect("rotations")).insert(key, (2, rotated));
    assert_eq!(facts.pinned().base_entry(pos, 0, 17), Some(rotated));
    assert_eq!(facts.pinned().object_revision(key), 2);
    check_base(&facts);
    let changed = EntryFacts {
        appearance_id: 2361,
        ..before
    };
    {
        let mut callbacks = facts.callbacks.lock().expect("callbacks");
        let callbacks = Arc::make_mut(&mut callbacks);
        callbacks.changed.insert(key, (3, changed));
        callbacks.hidden.insert(key);
    }
    assert_eq!(facts.pinned().base_entry(pos, 0, 17), Some(changed));
    assert!(facts.pinned().base_hidden(key));
    assert_eq!(facts.pinned().object_revision(key), 3);
    check_base(&facts);
}
