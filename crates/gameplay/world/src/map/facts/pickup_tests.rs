//! MAP-PICKUP-1: the boot-time pickup decision stays separate from the view normalization.

#![allow(clippy::expect_used)]

use super::*;
use crate::map::boot::tests as fixture;

#[test]
fn map_pickup_preserves_wrapper_refusals_and_original_view_normalization() {
    use oteryn_world_bundle::sector::Item;
    let (bytes, _) = fixture::bundle();
    let bundle = oteryn_world_bundle::bundle::read(&bytes).expect("bundle");
    let mut sector = bundle.sectors[0].clone();
    let source = sector
        .tiles
        .iter()
        .find(|tile| tile.x == 7)
        .expect("coin tile")
        .clone();
    let ordinal = 1;
    let palette = source.items[1].palette;
    let pos = TilePos {
        x: source.x,
        y: source.y,
        floor: sector.floor,
    };
    let cases: Vec<(Attrs, u32, Option<pickup::Ineligible>)> = vec![
        (Attrs::default(), 0, None),
        (
            Attrs {
                count: Some(7),
                ..Attrs::default()
            },
            0,
            None,
        ),
        (
            Attrs {
                count: Some(0),
                ..Attrs::default()
            },
            0,
            Some(pickup::Ineligible::Quantity),
        ),
        (
            Attrs {
                count: Some(101),
                ..Attrs::default()
            },
            0,
            Some(pickup::Ineligible::Quantity),
        ),
        (
            Attrs {
                charges: Some(0),
                ..Attrs::default()
            },
            0,
            Some(pickup::Ineligible::Charges),
        ),
        (
            Attrs {
                charges: Some(3),
                ..Attrs::default()
            },
            0,
            Some(pickup::Ineligible::Charges),
        ),
        (
            Attrs {
                action: Some(1),
                ..Attrs::default()
            },
            0,
            Some(pickup::Ineligible::Action),
        ),
        (
            Attrs {
                unique: Some(1),
                ..Attrs::default()
            },
            0,
            Some(pickup::Ineligible::Unique),
        ),
        (
            Attrs {
                door: Some(1),
                ..Attrs::default()
            },
            0,
            Some(pickup::Ineligible::Door),
        ),
        (
            Attrs {
                depot: Some(1),
                ..Attrs::default()
            },
            0,
            Some(pickup::Ineligible::Depot),
        ),
        (
            Attrs {
                teleport: Some((1, 1, 7)),
                ..Attrs::default()
            },
            0,
            Some(pickup::Ineligible::Teleport),
        ),
        (
            Attrs {
                text: Some("text".into()),
                ..Attrs::default()
            },
            0,
            Some(pickup::Ineligible::Text),
        ),
        (
            Attrs {
                description: Some("description".into()),
                ..Attrs::default()
            },
            0,
            Some(pickup::Ineligible::Description),
        ),
        (Attrs::default(), 1, Some(pickup::Ineligible::House)),
    ];
    for (attrs, house, refusal) in cases {
        let mut tile = source.clone();
        tile.house = house;
        tile.items[1].attrs = attrs.clone();
        for contents in [false, true] {
            let mut tile = tile.clone();
            if contents {
                tile.items.push(Item {
                    palette,
                    depth: 1,
                    attrs: Attrs::default(),
                });
            }
            for dropped in [false, true] {
                let mut manifest = bundle.manifest.clone();
                if dropped {
                    manifest
                        .dropped_teleports
                        .push(placement_key(pos, ordinal).expect("key"));
                }
                sector.tiles = vec![tile.clone()];
                let mut facts = BundleFacts::palette(&manifest, fixture::items).expect("facts");
                facts.push(&manifest, &sector, &mut BTreeSet::new());
                facts.finish();
                let wire = facts.base_entry(pos, ordinal, 1).expect("wire");
                assert_eq!(
                    wire.count,
                    attrs
                        .count
                        .filter(|count| (1..=100).contains(count))
                        .unwrap_or(1) as u32
                );
                assert_eq!(wire.sub_type, u32::from(attrs.charges.unwrap_or(0)));
                assert_eq!(wire.bound, dropped || bound(&attrs));
                let exact = pickup::eligibility(
                    &tile,
                    sector.floor,
                    ordinal,
                    true,
                    &manifest.dropped_teleports,
                );
                assert_eq!(facts.pickup_entry(pos, ordinal, 1), exact);
                if !contents && !dropped {
                    assert_eq!(exact.err(), refusal);
                }
            }
        }
    }
}
