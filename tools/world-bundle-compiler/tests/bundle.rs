//! Synthetic-fixture tests of the MAP-BUNDLE-1a compiler skeleton and bundle reader.

use std::error::Error as StdError;

use oteryn_world_bundle_compiler::Error;
use oteryn_world_bundle_compiler::bundle::{self, BuildClass, Extent, Family, Identity, Manifest};
use oteryn_world_bundle_compiler::compile::{Compiled, Input, KeyResolver, Resolution, compile};
use oteryn_world_bundle_compiler::project::Families;
use oteryn_world_bundle_compiler::sector::{self, Attrs, Item, Tile};

type TestResult = Result<(), Box<dyn StdError>>;

fn budget() -> sector::Budget {
    bundle::BUNDLE_BUDGET
}

struct Resolver;

impl KeyResolver for Resolver {
    fn resolve(&self, key: &str) -> Resolution {
        match key {
            "item:gold-coin" => Resolution::Resolved(Family::Item, 7),
            "item:chest" => Resolution::Resolved(Family::Item, 9),
            "terrain:grass" => Resolution::Resolved(Family::Terrain, 3),
            "donor:99" => Resolution::Provisional,
            _ => Resolution::Unknown,
        }
    }
}

fn palette() -> Vec<String> {
    [
        "terrain:grass",
        "item:chest",
        "item:gold-coin",
        "donor:99",
        "mystery",
    ]
    .map(String::from)
    .to_vec()
}

fn item(palette: u32, depth: u8) -> Item {
    Item {
        palette,
        depth,
        attrs: Attrs::default(),
    }
}

fn tile(x: u16, y: u16, items: Vec<Item>) -> Tile {
    Tile {
        x,
        y,
        flags: 0,
        house: 0,
        zones: Vec::new(),
        items,
    }
}

/// A B3 region file built the way `world_region_codec.encode_region` builds one.
fn region(
    z: u8,
    rx: u16,
    ry: u16,
    sectors: &[(u8, Vec<Tile>)],
) -> Result<Vec<u8>, Box<dyn StdError>> {
    let mut frames = Vec::new();
    for (_, tiles) in sectors {
        frames.push(zstd::bulk::compress(&sector::encode(tiles)?, 3)?);
    }
    let mut out = b"OTRB".to_vec();
    out.extend_from_slice(&[1, z]);
    for value in [rx, ry, sectors.len() as u16] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    let mut offset = 12 + 9 * sectors.len();
    for ((local, _), frame) in sectors.iter().zip(&frames) {
        out.push(*local);
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        out.extend_from_slice(&(frame.len() as u32).to_le_bytes());
        offset += frame.len();
    }
    frames.iter().for_each(|frame| out.extend_from_slice(frame));
    Ok(out)
}

fn fixture() -> Result<Vec<Vec<u8>>, Box<dyn StdError>> {
    let mut chest = item(1, 0);
    chest.attrs.action = Some(0);
    let mut teleport = item(0, 0);
    teleport.attrs.teleport = Some((300, 40, 6));
    teleport.attrs.text = Some("to the tower".into());
    let mut housed = tile(33, 1, vec![item(0, 0)]);
    housed.house = 12;
    housed.zones = vec![4, 5];
    let ground = vec![
        (
            0,
            vec![
                tile(1, 2, vec![item(0, 0), chest, item(2, 1), item(2, 1)]),
                tile(5, 2, vec![teleport]),
            ],
        ),
        (1, vec![housed]),
    ];
    let upper = vec![(
        9,
        vec![tile(
            290,
            40,
            vec![item(0, 0), item(3, 0), item(2, 1), item(2, 0)],
        )],
    )];
    Ok(vec![region(7, 0, 0, &ground)?, region(6, 1, 0, &upper)?])
}

/// The families the fixture agrees with: its one teleport and its one house.
fn families() -> Families {
    let mut families = Families::default();
    families.teleports.insert((5, 2, 7), (300, 40, 6));
    families.houses.insert(12);
    families
}

fn input<'a>(
    regions: &'a [Vec<u8>],
    palette: &'a [String],
    class: BuildClass,
    families: &'a Families,
) -> Input<'a> {
    Input {
        regions,
        palette,
        identity: Identity {
            content_revision: "rev-1".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 512,
            max_y: 256,
            floors: vec![-7, -6],
        },
        build_class: class,
        draft_areas: Vec::new(),
        families,
    }
}

fn build(class: BuildClass) -> Result<Compiled, Error> {
    let (regions, palette) = (
        fixture().map_err(|e| Error::Format(e.to_string()))?,
        palette(),
    );
    compile(&input(&regions, &palette, class, &families()), &Resolver)
}

#[test]
fn compilation_is_byte_identical_and_equivalent_to_the_source() -> TestResult {
    let first = build(BuildClass::NonProduction)?;
    assert_eq!(first.bytes, build(BuildClass::NonProduction)?.bytes);
    let read = bundle::read(&first.bytes)?;
    assert_eq!(read.digest, first.digest);
    let keys: Vec<&str> = read
        .manifest
        .palette
        .iter()
        .map(|p| p.key.as_str())
        .collect();
    assert_eq!(keys, ["terrain:grass", "item:chest", "item:gold-coin"]);
    assert_eq!(read.manifest.skipped_provisional_keys, ["donor:99"]);
    // Source to bundle, tile by tile: floor = -z, the provisional entry and its contents skipped.
    let (regions, palette) = (fixture()?, palette());
    let mut source = Vec::new();
    for data in &regions {
        let region = oteryn_world_bundle_compiler::b3::decode_region(
            data,
            bundle::TILE_LIMITS,
            &mut budget(),
        )?;
        for (_, _, tiles) in region.sectors {
            source.extend(tiles.into_iter().map(|t| (-(region.z as i8), t)));
        }
    }
    let bundled: Vec<(i8, &Tile)> = read
        .sectors
        .iter()
        .flat_map(|s| s.tiles.iter().map(move |t| (s.floor, t)))
        .collect();
    assert_eq!(bundled.len(), source.len());
    for (floor, tile) in bundled {
        let (_, original) = source
            .iter()
            .find(|(f, t)| *f == floor && (t.x, t.y) == (tile.x, tile.y))
            .ok_or("tile")?;
        assert_eq!(
            (tile.flags, tile.house, &tile.zones),
            (original.flags, original.house, &original.zones)
        );
        let kept: Vec<&Item> = original.items.iter().filter(|i| i.palette != 3).collect();
        let kept = if floor == -6 {
            vec![kept[0], kept[2]]
        } else {
            kept
        };
        assert_eq!(tile.items.len(), kept.len());
        for (got, want) in tile.items.iter().zip(kept) {
            assert_eq!(
                read.manifest.palette[got.palette as usize].key,
                palette[want.palette as usize]
            );
            assert_eq!(got.depth, want.depth);
            let teleport = want
                .attrs
                .teleport
                .map(|(x, y, z)| (x, y, (-(z as i8)) as u8));
            assert_eq!(
                got.attrs,
                Attrs {
                    teleport,
                    ..want.attrs.clone()
                }
            );
        }
    }
    Ok(())
}

#[test]
fn provisional_keys_fail_production_and_unknown_keys_always_fail() -> TestResult {
    assert!(matches!(build(BuildClass::Production), Err(Error::Key(_))));
    let diagnostics = build(BuildClass::NonProduction)?.diagnostics;
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        (diagnostics[0].x, diagnostics[0].y, diagnostics[0].floor),
        (290, 40, -6)
    );
    let regions = vec![region(7, 0, 0, &[(0, vec![tile(1, 1, vec![item(4, 0)])])])?];
    let palette = palette();
    let fams = Families::default();
    let result = compile(
        &input(&regions, &palette, BuildClass::NonProduction, &fams),
        &Resolver,
    );
    assert!(matches!(result, Err(Error::Key(_))));
    Ok(())
}

#[test]
fn limits_bounds_and_production_gate_fail_closed() -> TestResult {
    let palette = palette();
    let fams = Families::default();
    let crowded = vec![region(
        7,
        0,
        0,
        &[(0, vec![tile(1, 1, vec![item(0, 0); 65])])],
    )?];
    let result = compile(
        &input(&crowded, &palette, BuildClass::NonProduction, &fams),
        &Resolver,
    );
    assert!(matches!(result, Err(Error::Limit(_))));
    let deep = vec![region(5, 0, 0, &[(0, vec![tile(1, 1, vec![item(0, 0)])])])?];
    let result = compile(
        &input(&deep, &palette, BuildClass::NonProduction, &fams),
        &Resolver,
    );
    assert!(matches!(result, Err(Error::Bounds(_))));
    let fine = vec![region(
        7,
        0,
        0,
        &[(0, vec![tile(1, 1, vec![item(0, 0); 64])])],
    )?];
    let mut drafts = input(&fine, &palette, BuildClass::Production, &fams);
    assert!(compile(&drafts, &Resolver).is_ok());
    drafts.draft_areas = vec!["area:temple-of-light".into()];
    assert!(matches!(compile(&drafts, &Resolver), Err(Error::Format(_))));
    Ok(())
}

#[test]
fn reader_rejects_corrupt_and_truncated_bundles() -> TestResult {
    let bytes = build(BuildClass::NonProduction)?.bytes;
    let mut flipped = bytes.clone();
    let at = flipped.len() - 40;
    flipped[at] ^= 1;
    assert!(matches!(bundle::read(&flipped), Err(Error::Format(_))));
    assert!(bundle::read(&bytes[..bytes.len() - 1]).is_err());
    // A frame changed and the digest recomputed still fails its sector checksum.
    let flipped = reseal(flipped);
    let error = bundle::read(&flipped)
        .err()
        .ok_or("corrupt frame accepted")?;
    assert_eq!(error, Error::Format("sector checksum".into()));
    Ok(())
}

#[test]
fn build_class_fails_closed_and_placement_keys_are_positional() -> TestResult {
    let manifest = bundle::read(&build(BuildClass::NonProduction)?.bytes)?.manifest;
    let mut json: serde_json::Value = serde_json::to_value(&manifest)?;
    json["build_class"] = "staging".into();
    assert!(!serde_json::from_value::<Manifest>(json.clone())?.is_production());
    // Only a string is a build class: an object, null or a number is malformed, not production.
    for bad in [
        serde_json::json!({"production": null}),
        serde_json::json!({"non-production": null}),
        serde_json::Value::Null,
        serde_json::json!(1),
        serde_json::json!(["production"]),
    ] {
        json["build_class"] = bad;
        assert!(serde_json::from_value::<Manifest>(json.clone()).is_err());
    }
    json["build_class"] = "production".into();
    assert!(serde_json::from_value::<Manifest>(json.clone())?.is_production());
    // The same holds for `family`: a string naming a known family, nothing else.
    assert!(json["palette"][0]["family"].is_string());
    for bad in [
        serde_json::json!({"item": null}),
        serde_json::json!({"terrain": null}),
        serde_json::Value::Null,
        serde_json::json!("world-object"),
    ] {
        json["palette"][0]["family"] = bad;
        assert!(serde_json::from_value::<Manifest>(json.clone()).is_err());
    }
    json["palette"][0]["family"] = "terrain".into();
    assert!(serde_json::from_value::<Manifest>(json.clone()).is_ok());
    json.as_object_mut().ok_or("object")?.remove("build_class");
    assert!(!serde_json::from_value::<Manifest>(json)?.is_production());
    assert_eq!(
        bundle::placement_key(-7, 1, 2, 1),
        Some(1 << 32 | 2 << 16 | 7 << 8 | 1)
    );
    assert_ne!(
        bundle::placement_key(-6, 1, 2, 1),
        bundle::placement_key(-7, 1, 2, 1)
    );
    assert_eq!(bundle::placement_key(5, 1, 2, 1), None);
    assert_eq!(bundle::placement_key(-5, 1, 2, 64), None);
    assert!(manifest.compiler_version.contains(" zstd/1."));
    Ok(())
}

/// `fixtures/codec-region-z07-x000-y000.b3` was written by `world_region_codec.encode_region`
/// (#1170 head `fde85fa8`) from synthetic tiles; the Rust reader must decode it exactly.
#[test]
fn reader_matches_the_python_b3_codec() -> TestResult {
    let data = include_bytes!("fixtures/codec-region-z07-x000-y000.b3");
    let region =
        oteryn_world_bundle_compiler::b3::decode_region(data, bundle::TILE_LIMITS, &mut budget())?;
    assert_eq!(
        (region.z, region.rx, region.ry, region.sectors.len()),
        (7, 0, 0, 2)
    );
    let (sx, sy, tiles) = &region.sectors[0];
    assert_eq!((*sx, *sy, tiles.len()), (0, 0, 3));
    assert_eq!(tiles[0], tile(1, 0, vec![item(0, 0)]));
    let rich = &tiles[1];
    assert_eq!(
        (rich.x, rich.y, rich.flags, rich.house, &rich.zones),
        (3, 2, 0x10, 77, &vec![1, 300])
    );
    let first = Attrs {
        count: Some(0),
        action: Some(1000),
        unique: Some(60000),
        door: Some(3),
        ..Attrs::default()
    };
    let second = Attrs {
        text: Some("ząb".into()),
        charges: Some(12),
        description: Some(String::new()),
        ..Attrs::default()
    };
    let third = Attrs {
        teleport: Some((32000, 31000, 7)),
        depot: Some(4),
        ..Attrs::default()
    };
    let expected = vec![
        Item {
            palette: 5,
            depth: 0,
            attrs: first,
        },
        Item {
            palette: 6,
            depth: 1,
            attrs: second,
        },
        Item {
            palette: 200,
            depth: 2,
            attrs: third,
        },
    ];
    assert_eq!(rich.items, expected);
    assert_eq!(tiles[2], tile(31, 31, Vec::new()));
    let (sx, sy, tiles) = &region.sectors[1];
    let mut flagged = tile(34, 36, vec![item(1, 0), item(1, 0)]);
    flagged.flags = 1;
    assert_eq!((*sx, *sy, tiles.as_slice()), (1, 1, [flagged].as_slice()));
    // The shared grammar re-encodes the codec's payload byte for byte.
    let offset = u32::from_le_bytes([data[13], data[14], data[15], data[16]]) as usize;
    let length = u32::from_le_bytes([data[17], data[18], data[19], data[20]]) as usize;
    let payload = zstd::bulk::decompress(&data[offset..offset + length], 1 << 20)?;
    assert_eq!(sector::encode(&region.sectors[0].2)?, payload);
    Ok(())
}

fn reseal(mut bytes: Vec<u8>) -> Vec<u8> {
    let body = bytes.len() - 32;
    let mut hash = <sha2::Sha256 as sha2::Digest>::new();
    sha2::Digest::update(&mut hash, b"OTERYN_WORLD_BUNDLE/v1\0");
    sha2::Digest::update(&mut hash, &bytes[..body]);
    let digest: [u8; 32] = sha2::Digest::finalize(hash).into();
    bytes[body..].copy_from_slice(&digest);
    bytes
}

fn patched(bytes: &[u8], at: usize, value: u32) -> Vec<u8> {
    let mut out = bytes.to_vec();
    out[at..at + 4].copy_from_slice(&value.to_le_bytes());
    reseal(out)
}

#[test]
fn registered_limits_are_checked_before_allocation() -> TestResult {
    let bytes = build(BuildClass::NonProduction)?.bytes;
    assert!(bundle::read(&reseal(bytes.clone())).is_ok());
    let limit = |result: Result<bundle::Bundle, Error>| matches!(result, Err(Error::Limit(_)));
    assert!(limit(bundle::read(&patched(&bytes, 8, (16 << 20) + 1))));
    assert!(limit(bundle::read(&patched(&bytes, 12, (1 << 20) + 1))));
    let row = 16 + u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let compressed = u32::from_le_bytes([
        bytes[row + 10],
        bytes[row + 11],
        bytes[row + 12],
        bytes[row + 13],
    ]);
    let raw = u32::from_le_bytes([
        bytes[row + 14],
        bytes[row + 15],
        bytes[row + 16],
        bytes[row + 17],
    ]);
    assert!(limit(bundle::read(&patched(
        &bytes,
        row + 14,
        (16 << 20) + 1
    ))));
    assert!(limit(bundle::read(&patched(
        &bytes,
        row + 14,
        compressed * 1024 + 1
    ))));
    assert!(matches!(
        bundle::read(&patched(&bytes, row + 14, raw + 1)),
        Err(Error::Format(_))
    ));
    // Per tile: 4,097 entries, and a 4,097-byte text, are refused from their length fields.
    let crowded = [1, 0, 0x88, 0x80, 0x02];
    assert!(matches!(
        sector::decode(&crowded, (0, 0), bundle::TILE_LIMITS, &mut budget()),
        Err(Error::Limit(_))
    ));
    let text = [1, 0, 1 << 3, 1, 1 << 5, 0x81, 0x20];
    assert!(matches!(
        sector::decode(&text, (0, 0), bundle::TILE_LIMITS, &mut budget()),
        Err(Error::Limit(_))
    ));
    let ok = [1, 0, 1 << 3, 1, 1 << 5, 0x80, 0x20];
    assert!(matches!(
        sector::decode(&ok, (0, 0), bundle::TILE_LIMITS, &mut budget()),
        Err(Error::Format(_))
    ));
    Ok(())
}

#[test]
fn skipped_containers_still_resolve_and_the_writer_round_trips() -> TestResult {
    let palette = palette();
    let fams = Families::default();
    // A provisional container (3) holding an unknown key (4) fails even outside production.
    let hidden = vec![region(
        7,
        0,
        0,
        &[(0, vec![tile(1, 1, vec![item(3, 0), item(4, 1)])])],
    )?];
    let result = compile(
        &input(&hidden, &palette, BuildClass::NonProduction, &fams),
        &Resolver,
    );
    assert!(matches!(result, Err(Error::Key(_))));
    // A tile outside the sector it is written in never reaches the file.
    let manifest = bundle::read(&build(BuildClass::NonProduction)?.bytes)?.manifest;
    let stray = bundle::Sector {
        floor: -7,
        sx: 0,
        sy: 0,
        tiles: vec![tile(40, 1, vec![item(0, 0)])],
    };
    assert!(bundle::write(&manifest, &[stray]).is_err());
    let empty = bundle::Sector {
        floor: -7,
        sx: 0,
        sy: 0,
        tiles: Vec::new(),
    };
    assert!(bundle::write(&manifest, &[empty]).is_err());
    let mut unsorted = manifest.clone();
    unsorted.skipped_provisional_keys = vec!["b".into(), "a".into()];
    assert!(bundle::write(&unsorted, &[]).is_err());
    // Non-canonical varints: a redundant zero group, and bits past 64.
    for payload in [
        &[0x81, 0x00, 0][..],
        &[
            1, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02, 0,
        ],
    ] {
        assert!(sector::decode(payload, (0, 0), bundle::TILE_LIMITS, &mut budget()).is_err());
    }
    // The per-bundle entry budget is charged before a tile's items are reserved.
    let mut small = sector::Budget {
        tiles: 10,
        entries: 1,
    };
    let two = sector::encode(&[tile(1, 1, vec![item(0, 0), item(0, 0)])])?;
    assert!(matches!(
        sector::decode(&two, (0, 0), bundle::TILE_LIMITS, &mut small),
        Err(Error::Limit(_))
    ));
    Ok(())
}

fn word(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize
}

/// The registry's file, sector-count, sector-raw and total-raw boundaries, hit with reduced
/// maxima on a real bundle: exactly the maximum is accepted, one more is a `Limit`.
#[test]
fn reduced_maxima_accept_the_boundary_and_refuse_one_more() -> TestResult {
    let bytes = build(BuildClass::NonProduction)?.bytes;
    let (count, table) = (word(&bytes, 12), 16 + word(&bytes, 8));
    let raws: Vec<usize> = (0..count)
        .map(|i| word(&bytes, table + 50 * i + 14))
        .collect();
    let (total, largest) = (
        raws.iter().sum::<usize>(),
        *raws.iter().max().ok_or("rows")?,
    );
    assert!(count >= 2);
    let refused = |caps| matches!(bundle::read_with(&bytes, caps), Err(Error::Limit(_)));
    let exact = bundle::ReadCaps {
        file_bytes: bytes.len(),
        sectors: count,
        sector_raw_bytes: largest,
        total_raw_bytes: total,
    };
    assert!(bundle::read_with(&bytes, exact).is_ok());
    assert!(refused(bundle::ReadCaps {
        file_bytes: bytes.len() - 1,
        ..exact
    }));
    assert!(refused(bundle::ReadCaps {
        sectors: count - 1,
        ..exact
    }));
    assert!(refused(bundle::ReadCaps {
        sector_raw_bytes: largest - 1,
        ..exact
    }));
    // The total is over only on the row that crosses it, before that row is decompressed.
    assert!(refused(bundle::ReadCaps {
        total_raw_bytes: total - 1,
        ..exact
    }));
    assert_eq!(
        bundle::READ_CAPS.total_raw_bytes,
        bundle::MAX_TOTAL_RAW_BYTES
    );
    // A file over the real 1 GiB maximum is refused before anything is parsed.
    let huge = vec![0u8; bundle::MAX_FILE_BYTES + 1];
    assert!(matches!(bundle::read(&huge), Err(Error::Limit(_))));
    Ok(())
}

/// `MAP01-BUNDLE-TILES` and `MAP01-BUNDLE-ENTRIES`: exactly the budget decodes, one less fails.
#[test]
fn bundle_tile_and_entry_budgets_are_exact() -> TestResult {
    let tiles = vec![
        tile(1, 2, vec![item(0, 0), item(1, 0), item(2, 1)]),
        tile(3, 2, vec![item(0, 0)]),
    ];
    let payload = sector::encode(&tiles)?;
    let decode = |tiles, entries| {
        let mut budget = sector::Budget { tiles, entries };
        sector::decode(&payload, (0, 0), bundle::TILE_LIMITS, &mut budget)
    };
    assert_eq!(decode(2, 4)?, tiles);
    assert!(matches!(decode(1, 4), Err(Error::Limit(_))));
    assert!(matches!(decode(2, 3), Err(Error::Limit(_))));
    Ok(())
}

/// ADR-0021 §4.8: flip every byte, recompute the digest, and the reader may refuse but must not
/// panic; without the reseal the digest itself refuses.
#[test]
fn reader_survives_every_resealed_byte_flip() -> TestResult {
    let bytes = build(BuildClass::NonProduction)?.bytes;
    let body = bytes.len() - 32;
    for at in 0..body {
        for mask in [0x01u8, 0x80, 0xFF] {
            let mut flipped = bytes.clone();
            flipped[at] ^= mask;
            assert!(bundle::read(&flipped).is_err());
            // Any outcome but a panic is acceptable for a resealed flip.
            let _ = bundle::read(&reseal(flipped));
        }
    }
    Ok(())
}
