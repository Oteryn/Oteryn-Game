use super::*;

#[test]
fn admit_cap_2_bundle_zone_shapes_keep_distinct_retained_flags_through_boot() {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    let cases = [
        (0, (false, false)),
        (4, (true, false)),
        (16, (false, true)),
        (20, (true, false)),
        (21, (false, false)),
    ];
    let tiles = cases
        .iter()
        .enumerate()
        .map(|(index, (flags, _))| Tile {
            x: u16::try_from(index + 1).unwrap(),
            y: 0,
            flags: *flags,
            house: 0,
            zones: Vec::new(),
            items: vec![Item {
                palette: 0,
                depth: 0,
                attrs: Attrs::default(),
            }],
        })
        .collect();
    let world = world_from_bundle(bundle(Some(vec![oteryn_world_bundle::writer::Sector {
        floor: -7,
        sx: 0,
        sy: 0,
        tiles,
    }])));
    let (_, cells) = cells(&world);
    for (index, (_, expected)) in cases.iter().enumerate() {
        let tile = cells
            .spell_tiles()
            .lookup(cells.scope(), at(i32::try_from(index + 1).unwrap()))
            .unwrap();
        assert_eq!(tile.no_pvp_zone(), Some(expected.0));
        assert_eq!(tile.pvp_zone(), Some(expected.1));
        assert_eq!(tile.source_step(), None);
    }
}

#[test]
fn admit_cap_2_bundle_zone_flags_follow_the_pinned_source_loader_priority() {
    for (flags, expected) in [
        (0, (false, false)),
        (1 << 2, (true, false)),
        (1 << 4, (false, true)),
        ((1 << 2) | (1 << 4), (true, false)),
        (1 | (1 << 2) | (1 << 4), (false, false)),
        (1 << 3, (false, false)), // NOLOGOUT grants no PvP semantics.
    ] {
        assert_eq!(pvp_zones(flags), expected);
    }
}

#[test]
fn admit_cap_2_unqualified_bundle_zone_facts_stay_unknown() {
    let unknown =
        QualifiedSpellTile::from_bundle(None, Vec::new(), flags(false, false, false, false), None);
    assert_eq!(unknown.no_pvp_zone(), None);
    assert_eq!(unknown.pvp_zone(), None);
    assert_eq!(unknown.source_step(), None);
}
