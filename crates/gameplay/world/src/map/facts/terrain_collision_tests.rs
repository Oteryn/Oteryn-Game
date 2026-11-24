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
