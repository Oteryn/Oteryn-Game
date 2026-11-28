#![allow(clippy::expect_used)]
use super::*;

#[test]
fn source_bookstands_preserve_walkable_boss_staging() -> Result<(), String> {
    let reference = NonZeroU32::new(1).ok_or("Item definition reference")?;
    for (key, solid) in [
        ("oteryn:item.tibia.i28142", false),
        ("oteryn:item.tibia.i28144", false),
        ("oteryn:item.tibia.i28891", false),
        ("oteryn:item.tibia.i28143", true),
    ] {
        let entry = PaletteEntry {
            key: key.to_owned(),
            family: Family::Item,
            id: 0,
            terrain: None,
        };
        let facts = |native_solid| {
            palette_facts(&entry, &|_| {
                Some(ItemDefinition {
                    reference,
                    solid: native_solid,
                    blocks_projectile: true,
                    pickupable: false,
                    door: false,
                })
            })
            .map_err(|error| format!("{key}: {error:?}"))
        };
        assert_eq!(facts(None)?.solid, solid, "{key} source collision");
        assert!(facts(Some(true))?.solid, "{key} explicit native policy");
    }
    Ok(())
}

#[test]
fn cleared_terrain_releases_only_its_own_blocker_and_regrowth_restores_it() {
    use crate::map::boot::tests as fixture;
    for other_blocker in [false, true] {
        let (bytes, mut pins) = fixture::bundle_with_item_states(
            ("oteryn:item.tibia.i3696", 0),
            ("oteryn:item.tibia.i1772", 1),
            &[("oteryn:item.tibia.i3695", 2)],
        );
        let mut data = oteryn_world_bundle::bundle::read(&bytes).expect("fixture source");
        if other_blocker {
            let tiles = &mut data.sectors[0].tiles;
            let blocker = tiles
                .iter()
                .find(|tile| tile.x == 7)
                .expect("other source tile")
                .items[1]
                .clone();
            tiles
                .iter_mut()
                .find(|tile| tile.x == 6)
                .expect("grass tile")
                .items
                .push(blocker);
        }
        let bytes =
            oteryn_world_bundle::writer::write(&data.manifest, &data.sectors, &Default::default())
                .expect("write");
        pins.digest = oteryn_world_bundle::bundle::read(&bytes)
            .expect("read")
            .digest;
        let base = crate::map::load(&bytes, &pins).expect("base");
        let definitions = |key: &str| {
            let (id, solid) = match key {
                "oteryn:item.tibia.i3696" => (1, true),
                "oteryn:item.tibia.i1772" => (2, true),
                "oteryn:item.tibia.i3695" => (3, false),
                _ => return None,
            };
            Some(ItemDefinition {
                reference: NonZeroU32::new(id).expect("reference"),
                solid: Some(solid),
                blocks_projectile: solid,
                pickupable: false,
                door: false,
            })
        };
        let mut facts = BundleFacts::palette(&data.manifest, definitions).expect("facts");
        let mut blocked = BTreeSet::new();
        for sector in &data.sectors {
            facts.push(&data.manifest, sector, &mut blocked);
        }
        facts.finish();
        let pos = TilePos {
            x: 6,
            y: 0,
            floor: -7,
        };
        let key = placement_key(pos, 1).expect("source key");
        let before = facts.base_entry(pos, 1, 0).expect("native grass");
        let after = facts
            .movement_item_facts(3695)
            .expect("qualified cut state");
        assert!(facts.quest_callback_blocked(&base, pos, blocked.contains(&pos)));
        assert!(facts.install_terrain_state(key, 1, after));
        assert_eq!(
            facts.quest_callback_blocked(&base, pos, blocked.contains(&pos)),
            other_blocker,
            "cut grass cannot remove another solid entry"
        );
        assert!(facts.install_terrain_state(key, 2, before));
        assert!(
            facts.quest_callback_blocked(&base, pos, blocked.contains(&pos)),
            "regrown grass blocks again"
        );
        assert!(facts.install_terrain_state(key, 1, after));
        assert!(
            facts.quest_callback_blocked(&base, pos, blocked.contains(&pos)),
            "older completion cannot reopen regrown grass"
        );
        assert!(facts.install_terrain_state(
            key,
            3,
            EntryFacts {
                definition: MapDefinition::Item(NonZeroU32::new(9999).expect("unknown reference")),
                ..after
            }
        ));
        assert!(
            facts.quest_callback_blocked(&base, pos, blocked.contains(&pos)),
            "an unqualified replacement cannot clear the original blocker"
        );
    }
}

#[test]
fn crystal_map_item_collision_uses_donor_facts_without_a_native_profile() {
    use crate::content::native_gameplay::test_support::activated_with_loot_tables;
    use crate::map::boot::native_item_definition;

    let world =
        crate::foundation::WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
            .expect("world id");
    let (content, _) = activated_with_loot_tables(world, None);
    let native = content.active().and_then(|active| active.native_gameplay());
    for (id, solid, projectile) in [
        (7705, false, false),  // Oressa buttress
        (15487, false, false), // Manop's floor overlay
        (1632, true, true),    // Authored closed castle door
        (1633, false, false),  // Open castle door
    ] {
        let key = format!("oteryn:item.tibia.i{id}");
        let item = native_item_definition(native, &key).expect("item definition");
        let entry = PaletteEntry {
            key,
            family: Family::Item,
            id: item.reference.get() - 1,
            terrain: None,
        };
        let facts = palette_facts(&entry, &|key| native_item_definition(native, key))
            .expect("item palette facts");
        assert_eq!(
            (facts.solid, facts.facts.blocks_projectile),
            (solid, projectile),
            "item {id}"
        );
    }
}

#[test]
fn door_poses_preserve_independent_crystal_collision_flags() {
    let items = |key: &str| {
        let flags = oteryn_world_bundle::crystal_item_collision::for_key(key)?;
        Some(ItemDefinition {
            reference: NonZeroU32::MIN,
            solid: Some(flags.solid),
            blocks_projectile: flags.blocks_projectile,
            pickupable: false,
            door: flags.door,
        })
    };
    for (id, initially_locked, starts_open, open_solid, open_blocks_projectile) in [
        (1631, true, false, false, false),
        (1632, false, false, false, false),
        (1633, false, true, false, false),
        (5733, false, false, false, false),
        (5735, true, false, false, false),
        (8352, false, false, false, true),
        (12249, false, false, true, false),
    ] {
        let closed = crate::map::door_registry::closed_of(id).unwrap_or(id);
        let pose = palette_door(&format!("oteryn:item.tibia.i{id}"), id, closed, &items);
        assert!(
            matches!(pose, PaletteDoor::Pair { locked, starts_open: posed_open, open, open_solid: solid, key_forms, .. }
                if locked == initially_locked && posed_open == starts_open
                    && solid == open_solid && open.blocks_projectile == open_blocks_projectile
                    && key_forms.is_some() == (id != 12249)),
            "Crystal door {id} has unexpected pose {pose:?}"
        );
    }
}
