//! MAP-BUNDLE-1b: placements checked against the Transition.Teleport and House families, the
//! teleport split of ruling #162 5910173902 Q3 and the parity report.

use std::error::Error as StdError;

use oteryn_world_bundle_compiler::Error;
use oteryn_world_bundle_compiler::bundle::{
    self, BuildClass, Extent, Family, Identity, PaletteEntry,
};
use oteryn_world_bundle_compiler::compile::{
    Equivalence, Input, KeyResolver, Resolution, compile, equivalence, parity,
};
use oteryn_world_bundle_compiler::project::{self, Families};
use oteryn_world_bundle_compiler::sector::{self, Attrs, Item, Tile};

type TestResult = Result<(), Box<dyn StdError>>;

struct Resolver;

impl KeyResolver for Resolver {
    fn resolve(&self, key: &str) -> Resolution {
        match key {
            "item:teleport" => Resolution::Resolved(Family::Item, 1),
            "item:bag" => Resolution::Resolved(Family::Item, 2),
            "item:extra" => Resolution::Resolved(Family::Item, 3),
            "donor:99" => Resolution::Provisional,
            _ => Resolution::Unknown,
        }
    }
}

fn keys() -> Vec<String> {
    ["item:teleport", "item:bag", "donor:99"]
        .map(String::from)
        .to_vec()
}

fn item(palette: u32, depth: u8, teleport: Option<(u16, u16, u8)>) -> Item {
    Item {
        palette,
        depth,
        attrs: Attrs {
            teleport,
            ..Attrs::default()
        },
    }
}

fn tile(x: u16, y: u16, house: u32, items: Vec<Item>) -> Tile {
    Tile {
        x,
        y,
        flags: 0,
        house,
        zones: Vec::new(),
        items,
    }
}

/// A one-sector B3 region file of legacy floor 7 at region (0, 0).
fn region(tiles: Vec<Tile>) -> Result<Vec<u8>, Box<dyn StdError>> {
    let frame = zstd::bulk::compress(&sector::encode(&tiles)?, 3)?;
    let mut out = b"OTRB".to_vec();
    out.extend_from_slice(&[1, 7]);
    for value in [0u16, 0, 1] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.push(0);
    out.extend_from_slice(&21u32.to_le_bytes());
    out.extend_from_slice(&(frame.len() as u32).to_le_bytes());
    out.extend_from_slice(&frame);
    Ok(out)
}

fn build(
    tiles: Vec<Tile>,
    families: &Families,
) -> Result<oteryn_world_bundle_compiler::compile::Compiled, Error> {
    let regions = [region(tiles).map_err(|e| Error::Format(e.to_string()))?];
    let palette = keys();
    compile(&input(&regions, &palette, families, &world()), &Resolver)
}

fn input<'a>(
    regions: &'a [Vec<u8>],
    palette: &'a [String],
    families: &'a Families,
    world: &Extent,
) -> Input<'a> {
    Input {
        regions,
        palette,
        identity: Identity::default(),
        world: world.clone(),
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families,
    }
}

/// [`equivalence`] against the input of [`build`] with these parts replaced.
fn prove(
    regions: &[Vec<u8>],
    palette: &[String],
    resolver: &dyn KeyResolver,
    families: &Families,
    world: &Extent,
    bundle: &[u8],
) -> Result<Equivalence, Error> {
    equivalence(&input(regions, palette, families, world), resolver, bundle)
}

fn world() -> Extent {
    Extent {
        min_x: 0,
        min_y: 0,
        max_x: 256,
        max_y: 256,
        floors: vec![-7, -6],
    }
}

fn families() -> Families {
    let mut families = Families::default();
    families.teleports.insert((3, 1, 7), (9, 9, 6));
    families.houses.insert(40);
    families
}

/// The map of the tests: a matched teleport, a zero-destination teleport as the second
/// top-level entry, one inside a bag, and one inside a skipped provisional entry.
fn map() -> Vec<Tile> {
    vec![
        tile(1, 1, 0, vec![item(1, 0, None), item(0, 0, Some((0, 0, 0)))]),
        tile(
            2,
            1,
            40,
            vec![item(1, 0, None), item(0, 1, Some((0, 0, 0)))],
        ),
        tile(3, 1, 0, vec![item(0, 0, Some((9, 9, 6)))]),
        tile(4, 1, 0, vec![item(2, 0, None), item(0, 1, Some((0, 0, 0)))]),
    ]
}

#[test]
fn zero_destination_teleports_are_dropped_and_keyed() -> TestResult {
    let compiled = build(map(), &families())?;
    assert_eq!(
        compiled.dropped_teleports,
        [(1, 1, -7), (2, 1, -7), (4, 1, -7)]
    );
    let read = bundle::read(&compiled.bytes)?;
    // The skipped provisional entry has no key; the bag's key covers its contents.
    let expected = [
        bundle::placement_key(-7, 1, 1, 1).ok_or("key")?,
        bundle::placement_key(-7, 2, 1, 0).ok_or("key")?,
    ];
    assert_eq!(read.manifest.dropped_teleports, expected);
    let tiles = &read.sectors[0].tiles;
    assert_eq!(tiles[0].items[1].attrs.teleport, None);
    assert_eq!(tiles[1].items[1].attrs.teleport, None);
    // A matched teleport keeps its destination, at the native floor.
    assert_eq!(tiles[2].items[0].attrs.teleport, Some((9, 9, (-6i8) as u8)));
    // A key that names no top-level entry is refused, and so is an unsorted list.
    let mut manifest = read.manifest.clone();
    manifest.dropped_teleports = vec![bundle::placement_key(-7, 3, 1, 1).ok_or("key")?];
    assert!(matches!(
        bundle::write(&manifest, &read.sectors),
        Err(Error::Format(_))
    ));
    manifest.dropped_teleports = expected.iter().rev().copied().collect();
    assert!(matches!(
        bundle::write(&manifest, &read.sectors),
        Err(Error::Format(_))
    ));
    Ok(())
}

#[test]
fn teleports_and_houses_disagreeing_with_their_families_fail() -> TestResult {
    let family = |result: Result<_, Error>| matches!(result, Err(Error::Family(_)));
    // A real destination without a record, and a record to another destination.
    let orphan = vec![tile(5, 1, 0, vec![item(0, 0, Some((9, 9, 6)))])];
    let mut only_orphan = families();
    only_orphan.teleports.clear();
    assert!(family(build(orphan.clone(), &only_orphan)));
    let mut elsewhere = families();
    elsewhere.teleports.insert((3, 1, 7), (9, 8, 6));
    assert!(family(build(map(), &elsewhere)));
    // A record whose tile carries no teleport attribute.
    let mut unused = families();
    unused.teleports.insert((7, 7, 7), (9, 9, 6));
    assert!(family(build(map(), &unused)));
    // A (0,0,0) attribute on a tile that has a Transition record is a mismatch, not a drop.
    let zero_on_record = vec![tile(3, 1, 0, vec![item(0, 0, Some((0, 0, 0)))])];
    assert!(family(build(zero_on_record.clone(), &families())));
    let report = parity(&[region(zero_on_record)?], &families())?;
    assert!(report.zero_destination.is_empty());
    assert_eq!(report.mismatched, [((3, 1, 7), (0, 0, 0))]);
    // A house id the House family does not have.
    let mut no_house = families();
    no_house.houses.clear();
    assert!(family(build(map(), &no_house)));
    Ok(())
}

#[test]
fn parity_reports_every_class_at_once() -> TestResult {
    let mut tiles = map();
    tiles.push(tile(5, 1, 77, vec![item(0, 0, Some((9, 9, 5)))]));
    let mut families = families();
    families.teleports.insert((8, 8, 7), (1, 1, 7));
    let report = parity(&[region(tiles)?], &families)?;
    assert_eq!(report.tiles, 5);
    assert_eq!(report.teleport_attributes, 5);
    assert_eq!(report.teleports_matched, 1);
    assert_eq!(report.zero_destination, [(1, 1, 7), (2, 1, 7), (4, 1, 7)]);
    assert_eq!(report.no_record, [((5, 1, 7), (9, 9, 5))]);
    assert!(report.mismatched.is_empty());
    assert_eq!(report.records_without_attribute, [(8, 8, 7)]);
    assert_eq!(report.unknown_houses.get(&77), Some(&1));
    assert_eq!(report.max_tile_top_level_entries, 2);
    Ok(())
}

#[test]
fn family_shards_load_in_the_native_frame_and_fail_closed() -> TestResult {
    let world = br#"{"family":"World","records":[{"declaration":{"bounds":{"min_x":10,
        "min_y":20,"max_x_exclusive":30,"max_y_exclusive":40},
        "coordinate_frame":"global-target-2026-09-27","floors":[7,0,15]}}]}"#;
    let extent = project::world_extent(world)?;
    assert_eq!(
        (extent.min_x, extent.max_x, extent.floors),
        (10, 30, vec![-15, -7, 0])
    );
    let other_frame = String::from_utf8(world.to_vec())?.replace("2026-09-27", "2026-01-01");
    assert!(project::world_extent(other_frame.as_bytes()).is_err());
    let teleports = br#"{"family":"Transition.Teleport","records":[{"declaration":{
        "from":{"coordinate_frame":"global-target-2026-09-27","floor":7,"x":1,"y":2},
        "to":{"coordinate_frame":"global-target-2026-09-27","floor":6,"x":3,"y":4},
        "transition_kind":"teleport"}}]}"#;
    let mut families = Families::default();
    families.add_teleports(teleports)?;
    assert_eq!(families.teleports.get(&(1, 2, 7)), Some(&(3, 4, 6)));
    assert!(families.add_teleports(teleports).is_err());
    let houses = br#"{"houses":[{"provenance":{"engine_house":{"house_id":3437}}}]}"#;
    families.add_houses(houses)?;
    assert!(families.houses.contains(&3437));
    assert!(families.add_houses(houses).is_err());
    let index: serde_json::Value = serde_json::from_str(
        r#"{"source":{"minimap_draft":{"areas":[{"name":"nargor"},{"name":"blue-valley"}]}}}"#,
    )?;
    assert_eq!(project::draft_areas(&index)?, ["blue-valley", "nargor"]);
    let twice: serde_json::Value = serde_json::from_str(
        r#"{"source":{"minimap_draft":{"areas":[{"name":"nargor"},{"name":"nargor"}]}}}"#,
    )?;
    assert!(project::draft_areas(&twice).is_err());
    Ok(())
}

#[test]
fn a_compiled_bundle_is_equivalent_to_its_source_tile_by_tile() -> TestResult {
    let regions = [region(map())?];
    let compiled = build(map(), &families())?;
    let proof = prove(
        &regions,
        &keys(),
        &Resolver,
        &families(),
        &world(),
        &compiled.bytes,
    )?;
    // The provisional entry and its content, a dropped teleport, are left out.
    assert_eq!((proof.tiles, proof.dropped_teleports), (4, 2));
    assert_eq!((proof.entries, proof.skipped_entries), (5, 2));
    // Another source, or a palette that names other keys, is not equivalent.
    let mut other = map();
    other[0].flags = 1;
    assert!(
        prove(
            &[region(other)?],
            &keys(),
            &Resolver,
            &families(),
            &world(),
            &compiled.bytes
        )
        .is_err()
    );
    let swapped: Vec<String> = ["item:bag", "item:teleport", "donor:99"]
        .map(String::from)
        .to_vec();
    assert!(
        prove(
            &regions,
            &swapped,
            &Resolver,
            &families(),
            &world(),
            &compiled.bytes
        )
        .is_err()
    );
    let mut fewer = map();
    fewer.pop();
    assert!(
        prove(
            &[region(fewer)?],
            &keys(),
            &Resolver,
            &families(),
            &world(),
            &compiled.bytes
        )
        .is_err()
    );
    // A resolution other than the manifest's, or a provisional set the source does not give,
    // is not equivalent either: the proof reads the resolver, not the bundle's claims.
    struct Other;
    impl KeyResolver for Other {
        fn resolve(&self, key: &str) -> Resolution {
            match Resolver.resolve(key) {
                Resolution::Resolved(family, id) => Resolution::Resolved(family, id + 10),
                other => other,
            }
        }
    }
    assert!(
        prove(
            &regions,
            &keys(),
            &Other,
            &families(),
            &world(),
            &compiled.bytes
        )
        .is_err()
    );
    let read = bundle::read(&compiled.bytes)?;
    let mut manifest = read.manifest.clone();
    manifest.skipped_provisional_keys.push("donor:x".into());
    let claimed = bundle::write(&manifest, &read.sectors)?;
    assert!(
        prove(
            &regions,
            &keys(),
            &Resolver,
            &families(),
            &world(),
            &claimed
        )
        .is_err()
    );
    // A manifest that omits a dropped teleport key is not equivalent: its entry would be
    // materialized.
    let mut manifest = read.manifest.clone();
    manifest.dropped_teleports.pop();
    let omitted = bundle::write(&manifest, &read.sectors)?;
    assert!(
        prove(
            &regions,
            &keys(),
            &Resolver,
            &families(),
            &world(),
            &omitted
        )
        .is_err()
    );
    // The Transition rule is checked again from the families: a record to elsewhere, a
    // record on a (0,0,0) tile, a missing record and an unmet record all fail.
    let mut elsewhere = families();
    elsewhere.teleports.insert((3, 1, 7), (9, 8, 6));
    let mut on_zero = families();
    on_zero.teleports.insert((1, 1, 7), (0, 0, 0));
    let mut missing = families();
    missing.teleports.clear();
    let mut unmet = families();
    unmet.teleports.insert((7, 7, 7), (9, 9, 6));
    for wrong in [elsewhere, on_zero, missing, unmet] {
        assert!(
            prove(
                &regions,
                &keys(),
                &Resolver,
                &wrong,
                &world(),
                &compiled.bytes
            )
            .is_err()
        );
    }
    let fails = |result: Result<_, Error>, why: &str| matches!(result, Err(Error::Format(what)) if what.contains(why));
    // A house id is checked again against the House family.
    let mut no_house = families();
    no_house.houses.clear();
    assert!(fails(
        prove(
            &regions,
            &keys(),
            &Resolver,
            &no_house,
            &world(),
            &compiled.bytes
        ),
        "not in the House family"
    ));
    // The manifest World must be the World record.
    let mut wider = world();
    wider.max_x = 512;
    assert!(fails(
        prove(
            &regions,
            &keys(),
            &Resolver,
            &families(),
            &wider,
            &compiled.bytes
        ),
        "manifest world is not"
    ));
    // A key that resolves nowhere fails even as a descendant of a skipped provisional entry.
    let mut hidden = map();
    hidden[3].items[1].palette = 3;
    let mut unknown = keys();
    unknown.push("item:none".into());
    assert!(fails(
        prove(
            &[region(hidden)?],
            &unknown,
            &Resolver,
            &families(),
            &world(),
            &compiled.bytes
        ),
        "unresolved palette entry"
    ));
    // The whole manifest is derived from the input: another identity or other draft areas
    // than the compiler input, or a palette entry no kept source entry uses, is not equivalent.
    let fails_manifest = |input: &Input, bytes: &[u8], field: &str| {
        let why = format!("manifest {field} is not");
        fails(equivalence(input, &Resolver, bytes), &why)
    };
    let (palette, families) = (keys(), families());
    let mut other = input(&regions, &palette, &families, &world());
    other.identity.content_revision = "content-r2".into();
    assert!(fails_manifest(&other, &compiled.bytes, "identity"));
    let mut drafted = input(&regions, &palette, &families, &world());
    drafted.draft_areas = vec!["area:north".into()];
    assert!(fails_manifest(&drafted, &compiled.bytes, "draft_areas"));
    let mut manifest = read.manifest.clone();
    manifest.palette.push(PaletteEntry {
        key: "item:extra".into(),
        family: Family::Item,
        id: 3,
    });
    let unused = bundle::write(&manifest, &read.sectors)?;
    let exact = input(&regions, &palette, &families, &world());
    assert!(fails_manifest(&exact, &unused, "palette"));
    equivalence(&exact, &Resolver, &compiled.bytes)?;
    Ok(())
}
