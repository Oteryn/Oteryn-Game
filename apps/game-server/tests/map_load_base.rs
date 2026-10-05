//! MAP-LOAD-1 (MAP-LOAD-PACKET-1 §2.2): the World Bundle loader and the compact base model,
//! against a small fixture bundle that each test compiles with the compiler.

use std::error::Error as StdError;

use oteryn_game_server::map::{self, BundlePins, LOAD_LIMITS, LoadError, LoadLimits, WorldBase};
use oteryn_world_bundle_compiler::Error;
use oteryn_world_bundle_compiler::b3;
use oteryn_world_bundle_compiler::bundle::{
    self, BuildClass, Extent, Family, Identity, Manifest, ReadCaps, Terrain, TerrainKind,
};
use oteryn_world_bundle_compiler::compile::{Compiled, Input, KeyResolver, Resolution, compile};
use oteryn_world_bundle_compiler::project::Families;
use oteryn_world_bundle_compiler::sector::{self, Attrs, Budget, Item, Tile};
use oteryn_world_bundle_compiler::spawn;
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn StdError>>;

/// Palette keys of the fixture's B3 placements, by index.
const KEYS: [&str; 11] = [
    "terrain:grass",  // 0: ground, walkable, 150
    "terrain:mud",    // 1: ground, walkable, 300
    "terrain:border", // 2: border
    "terrain:lava",   // 3: ground, not walkable, 120
    "terrain:void",   // 4: ground, not walkable, 0
    "terrain:fast",   // 5: ground, walkable, 1,000
    "item:chest",     // 6: a plain Item (`terrain: null`)
    "item:coin",      // 7: a plain Item
    "terrain:stone",  // 8: ground, walkable, 1
    "terrain:wall",   // 9: wall
    "terrain:mosaic", // 10: common (never the ground item)
];

/// The fixture's catalogue: compact ids and terrain semantics of [`KEYS`].
fn resolved(key: &str) -> Option<(Family, u32, Option<Terrain>)> {
    let ground = |walkable, speed| {
        Some(Terrain {
            kind: TerrainKind::Ground,
            walkable: Some(walkable),
            ground_speed: Some(speed),
        })
    };
    let other = |kind| {
        Some(Terrain {
            kind,
            walkable: None,
            ground_speed: None,
        })
    };
    Some(match key {
        "terrain:grass" => (Family::Terrain, 101, ground(true, 150)),
        "terrain:mud" => (Family::Terrain, 102, ground(true, 300)),
        "terrain:border" => (Family::Terrain, 103, other(TerrainKind::Border)),
        "terrain:lava" => (Family::Terrain, 104, ground(false, 120)),
        "terrain:void" => (Family::Terrain, 105, ground(false, 0)),
        "terrain:fast" => (Family::Terrain, 106, ground(true, 1000)),
        "item:chest" => (Family::Item, 7, None),
        "item:coin" => (Family::Item, 9, None),
        "terrain:stone" => (Family::Terrain, 108, ground(true, 1)),
        "terrain:wall" => (Family::Terrain, 109, other(TerrainKind::Wall)),
        "terrain:mosaic" => (Family::Terrain, 110, other(TerrainKind::Common)),
        _ => return None,
    })
}

struct Resolver;

impl KeyResolver for Resolver {
    fn resolve(&self, key: &str) -> Resolution {
        resolved(key).map_or(Resolution::Unknown, |(family, id, _)| {
            Resolution::Resolved(family, id)
        })
    }

    fn terrain(&self, key: &str) -> Result<Option<Terrain>, Error> {
        Ok(resolved(key).and_then(|(_, _, terrain)| terrain))
    }

    fn floor_change(&self, _: &str) -> bool {
        false
    }
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

/// The fixture map: legacy `z` 7 (native floor -7) and `z` 6 (native floor -6).
fn regions() -> Result<Vec<Vec<u8>>, Box<dyn StdError>> {
    let mut chest = item(6, 0);
    chest.attrs.action = Some(0);
    let ground = vec![
        (
            0,
            vec![
                // A grass ground with a chest holding a coin.
                tile(1, 2, vec![item(0, 0), chest, item(7, 1)]),
                // A border first and a ground second: the second is the ground item.
                tile(2, 2, vec![item(2, 0), item(1, 0)]),
                // A non-walkable ground storing 120.
                tile(3, 2, vec![item(3, 0)]),
                // No ground item: a plain Item only.
                tile(4, 2, vec![item(6, 0)]),
                // A non-walkable ground storing 0.
                tile(6, 2, vec![item(4, 0)]),
                // The maximum ground speed.
                tile(7, 2, vec![item(5, 0)]),
                // Container contents do not count as top-level ordinals.
                tile(
                    8,
                    2,
                    vec![item(6, 0), item(7, 1), item(7, 1), item(9, 0), item(0, 0)],
                ),
                // MAP-KIND-CLASS-0 R4: a `common` entry is never the ground item. Alone it leaves
                // the tile with no ground, not walkable, ground speed 0; above a ground it does
                // not change the ground item.
                tile(10, 2, vec![item(10, 0)]),
                tile(11, 2, vec![item(10, 0), item(0, 0)]),
                // Two grounds: the first wins.
                tile(9, 3, vec![item(1, 0), item(0, 0)]),
            ],
        ),
        (1, vec![tile(33, 1, vec![item(8, 0)])]),
    ];
    let upper = vec![(9, vec![tile(290, 40, vec![item(0, 0), item(7, 0)])])];
    Ok(vec![region(7, 0, 0, &ground)?, region(6, 1, 0, &upper)?])
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
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
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

fn palette() -> Vec<String> {
    KEYS.map(String::from).to_vec()
}

fn build(class: BuildClass) -> Result<Compiled, Box<dyn StdError>> {
    let (regions, palette) = (regions()?, palette());
    Ok(compile(
        &input(&regions, &palette, class, &Families::default()),
        &Resolver,
    )?)
}

/// The pins that accept `compiled` in a World of the given class.
fn pins(compiled: &Compiled, production: bool) -> BundlePins {
    BundlePins {
        digest: compiled.digest,
        project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
        world_schema_version: "world-schema-1".into(),
        content_revision: "rev-1".into(),
        production,
    }
}

fn resealed(mut bytes: Vec<u8>) -> (Vec<u8>, [u8; 32]) {
    let body = bytes.len() - 32;
    let digest: [u8; 32] = Sha256::new()
        .chain_update(b"OTERYN_WORLD_BUNDLE/v3\0")
        .chain_update(&bytes[..body])
        .finalize()
        .into();
    bytes[body..].copy_from_slice(&digest);
    (bytes, digest)
}

fn word(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize
}

/// A bundle assembled byte by byte like the writer does, but without its validation, so a test
/// can hand the reader what the compiler never writes. `sectors` are `(floor, sx, sy, payload)`.
fn assemble(
    manifest: &Manifest,
    sectors: &[(i8, u16, u16, Vec<u8>)],
    spawn_raw: &[u8],
) -> Result<(Vec<u8>, [u8; 32]), Box<dyn StdError>> {
    let json = serde_json::to_vec(manifest)?;
    let mut frames = Vec::new();
    for (_, _, _, raw) in sectors {
        frames.push((raw.len(), compress(raw)?));
    }
    let spawn_frame = compress(spawn_raw)?;
    let u32_of = |value: usize| (value as u32).to_le_bytes();
    let mut out = b"OTWB".to_vec();
    out.extend_from_slice(&3u16.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&u32_of(json.len()));
    out.extend_from_slice(&u32_of(sectors.len()));
    out.extend_from_slice(&json);
    let mut offset = out.len() + 50 * sectors.len() + 44;
    for ((floor, sx, sy, _), (raw_length, frame)) in sectors.iter().zip(&frames) {
        out.extend_from_slice(&[*floor as u8, 0]);
        out.extend_from_slice(&sx.to_le_bytes());
        out.extend_from_slice(&sy.to_le_bytes());
        out.extend_from_slice(&u32_of(offset));
        out.extend_from_slice(&u32_of(frame.len()));
        out.extend_from_slice(&u32_of(*raw_length));
        out.extend_from_slice(&Sha256::digest(frame));
        offset += frame.len();
    }
    out.extend_from_slice(&u32_of(offset));
    out.extend_from_slice(&u32_of(spawn_frame.len()));
    out.extend_from_slice(&u32_of(spawn_raw.len()));
    out.extend_from_slice(&Sha256::digest(&spawn_frame));
    frames
        .iter()
        .for_each(|(_, frame)| out.extend_from_slice(frame));
    out.extend_from_slice(&spawn_frame);
    out.extend_from_slice(&[0; 32]);
    Ok(resealed(out))
}

fn compress(raw: &[u8]) -> Result<Vec<u8>, Box<dyn StdError>> {
    let mut compressor = zstd::bulk::Compressor::new(3)?;
    compressor.include_checksum(true)?;
    compressor.include_contentsize(true)?;
    Ok(compressor.compress(raw)?)
}

/// The fixture's manifest, and `pins` for any bundle a test assembles with it.
fn manifest() -> Result<Manifest, Box<dyn StdError>> {
    Ok(bundle::read(&build(BuildClass::NonProduction)?.bytes)?.manifest)
}

fn pinned(digest: [u8; 32]) -> BundlePins {
    BundlePins {
        digest,
        project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
        world_schema_version: "world-schema-1".into(),
        content_revision: "rev-1".into(),
        production: false,
    }
}

fn spawn_raw() -> Vec<u8> {
    spawn::encode(&spawn::Table::default())
}

fn refused_by_reader(result: Result<WorldBase, LoadError>) -> bool {
    matches!(result, Err(LoadError::Bundle(_)))
}

fn refused_at_limit(result: Result<WorldBase, LoadError>) -> bool {
    matches!(result, Err(LoadError::Bundle(Error::Limit(_))))
}

/// The ground item, walkable flag and stored ground speed of `items` as §1.3 defines them, from
/// the fixture catalogue alone: the first top-level entry whose terrain kind is `ground`.
fn expected_ground(items: &[Item]) -> Option<(u8, u32, bool, u16)> {
    items
        .iter()
        .filter(|item| item.depth == 0)
        .enumerate()
        .find_map(|(ordinal, item)| {
            let (_, id, terrain) = resolved(KEYS[item.palette as usize])?;
            let terrain = terrain.filter(|terrain| terrain.kind == TerrainKind::Ground)?;
            Some((ordinal as u8, id, terrain.walkable?, terrain.ground_speed?))
        })
}

#[test]
fn map_load_the_base_agrees_with_the_compiler_input_and_the_bundle_tile_by_tile() -> TestResult {
    let compiled = build(BuildClass::NonProduction)?;
    let base = map::load(&compiled.bytes, &pins(&compiled, false))?;
    assert_eq!(base.digest(), compiled.digest);
    assert_eq!(base.content_revision(), "rev-1");
    // Compiler input: the B3 regions, decoded independently, at native floor -z.
    let mut source = Vec::new();
    for data in &regions()? {
        let region = b3::decode_region(
            data,
            bundle::TILE_LIMITS,
            &mut bundle::BUNDLE_BUDGET.clone(),
        )?;
        for (_, _, tiles) in region.sectors {
            source.extend(tiles.into_iter().map(|tile| (-(region.z as i8), tile)));
        }
    }
    let read = bundle::read(&compiled.bytes)?;
    let bundled: Vec<(i8, &Tile)> = read
        .sectors
        .iter()
        .flat_map(|sector| sector.tiles.iter().map(move |tile| (sector.floor, tile)))
        .collect();
    let tiles: Vec<_> = base.tiles().collect();
    assert_eq!(tiles.len(), source.len());
    assert_eq!(tiles.len(), bundled.len());
    assert_eq!(base.tile_count(), source.len());
    assert_eq!(
        base.entry_count(),
        source
            .iter()
            .map(|(_, tile)| tile.items.len())
            .sum::<usize>()
    );
    for ((floor, x, y, view), (bundle_floor, bundled)) in tiles.iter().zip(&bundled) {
        assert_eq!((*floor, *x, *y), (*bundle_floor, bundled.x, bundled.y));
        let (_, original) = source
            .iter()
            .find(|(f, t)| f == floor && (t.x, t.y) == (*x, *y))
            .ok_or("tile missing from the source")?;
        let ids: Vec<u32> = original
            .items
            .iter()
            .map(|item| resolved(KEYS[item.palette as usize]).map(|(_, id, _)| id))
            .collect::<Option<_>>()
            .ok_or("fixture key")?;
        let depths: Vec<u8> = original.items.iter().map(|item| item.depth).collect();
        assert_eq!(view.ids(), ids, "{floor} {x} {y}");
        assert_eq!(view.depths(), depths, "{floor} {x} {y}");
        let bundled_ids: Vec<u32> = bundled
            .items
            .iter()
            .map(|item| read.manifest.palette[item.palette as usize].id)
            .collect();
        assert_eq!(view.ids(), bundled_ids);
        let ground = expected_ground(&original.items);
        assert_eq!(view.ground(), ground.map(|(ordinal, id, ..)| (ordinal, id)));
        assert_eq!(
            view.walkable(),
            ground.is_some_and(|(_, _, walkable, _)| walkable)
        );
        assert_eq!(
            view.stored_ground_speed(),
            ground.map_or(0, |(.., speed)| speed)
        );
        // The same tile by position.
        let by_position = base.tile(*x, *y, *floor).ok_or("lookup")?;
        assert_eq!(by_position.ids(), view.ids());
    }
    // Positions the bundle holds no tile at.
    assert!(base.tile(5, 2, -7).is_none());
    assert!(base.tile(1, 2, -6).is_none());
    assert!(base.tile(1, 2, -8).is_none());
    assert!(base.tile(4000, 4000, -7).is_none());
    Ok(())
}

#[test]
fn map_load_a_tile_whose_only_entry_is_common_has_no_ground_and_speed_0() -> TestResult {
    let compiled = build(BuildClass::NonProduction)?;
    let base = map::load(&compiled.bytes, &pins(&compiled, false))?;
    let alone = base.tile(10, 2, -7).ok_or("common-only tile")?;
    assert_eq!(alone.ground(), None);
    assert!(!alone.walkable());
    assert_eq!(base.ground_speed(10, 2, -7), 0);
    let above = base.tile(11, 2, -7).ok_or("common above a ground")?;
    assert_eq!(above.ground(), Some((1, 101)));
    Ok(())
}

#[test]
fn map_load_terrain_comes_from_the_bundle_palette_only() -> TestResult {
    let compiled = build(BuildClass::NonProduction)?;
    let base = map::load(&compiled.bytes, &pins(&compiled, false))?;
    let at = |x, y, floor| base.tile(x, y, floor).ok_or("tile");
    // Grass, under a chest holding a coin.
    let grass = at(1, 2, -7)?;
    assert_eq!(
        (grass.ground(), grass.walkable(), grass.ground_speed()),
        (Some((0, 101)), true, 150)
    );
    // A border first and a ground second: the second.
    let mud = at(2, 2, -7)?;
    assert_eq!((mud.ground(), mud.ground_speed()), (Some((1, 102)), 300));
    // A non-walkable ground with a stored 120 loads, and paces at 0.
    let lava = at(3, 2, -7)?;
    assert_eq!(
        (
            lava.ground(),
            lava.walkable(),
            lava.stored_ground_speed(),
            lava.ground_speed()
        ),
        (Some((0, 104)), false, 120, 0)
    );
    // A `null` terrain entry is not ground.
    let chest = at(4, 2, -7)?;
    assert_eq!(
        (chest.ground(), chest.walkable(), chest.ground_speed()),
        (None, false, 0)
    );
    // A non-walkable ground with 0 loads.
    let void = at(6, 2, -7)?;
    assert_eq!(
        (void.ground(), void.walkable(), void.ground_speed()),
        (Some((0, 105)), false, 0)
    );
    assert_eq!(at(7, 2, -7)?.ground_speed(), 1000);
    // Contents are not top-level: the grass is the third top-level entry.
    assert_eq!(at(8, 2, -7)?.ground(), Some((2, 101)));
    assert_eq!(at(9, 3, -7)?.ground(), Some((0, 102)));
    assert_eq!(at(33, 1, -7)?.ground_speed(), 1);
    assert_eq!(base.ground_speed(290, 40, -6), 150);
    assert_eq!(base.ground_speed(5, 2, -7), 0);
    Ok(())
}

#[test]
fn map_load_opens_no_file_besides_the_bundle() -> TestResult {
    let compiled = build(BuildClass::Production)?;
    let empty = std::env::temp_dir().join(format!("map-load-no-content-{}", std::process::id()));
    std::fs::create_dir_all(&empty)?;
    let previous = std::env::current_dir()?;
    std::env::set_current_dir(&empty)?;
    assert!(!std::path::Path::new("content").exists());
    let loaded = map::load(&compiled.bytes, &pins(&compiled, true));
    std::env::set_current_dir(previous)?;
    std::fs::remove_dir(&empty)?;
    assert_eq!(loaded?.tile_count(), 12);
    Ok(())
}

#[test]
fn map_load_refuses_a_bundle_that_does_not_match_its_pins() -> TestResult {
    let compiled = build(BuildClass::NonProduction)?;
    let good = pins(&compiled, false);
    assert!(map::load(&compiled.bytes, &good).is_ok());
    let mut wrong = good.clone();
    wrong.digest[0] ^= 1;
    assert_eq!(
        map::load(&compiled.bytes, &wrong).err(),
        Some(LoadError::Digest)
    );
    let wrong = BundlePins {
        content_revision: "rev-2".into(),
        ..good.clone()
    };
    assert_eq!(
        map::load(&compiled.bytes, &wrong).err(),
        Some(LoadError::ContentRevision)
    );
    let wrong = BundlePins {
        world_schema_version: "world-schema-2".into(),
        ..good.clone()
    };
    assert_eq!(
        map::load(&compiled.bytes, &wrong).err(),
        Some(LoadError::SchemaVersion)
    );
    let wrong = BundlePins {
        project_format_version: "OTERYN_WORLD_PROJECT/v3".into(),
        ..good.clone()
    };
    assert_eq!(
        map::load(&compiled.bytes, &wrong).err(),
        Some(LoadError::SchemaVersion)
    );
    // A production World refuses a non-production bundle, and a missing build class.
    let production = BundlePins {
        production: true,
        ..good.clone()
    };
    assert_eq!(
        map::load(&compiled.bytes, &production).err(),
        Some(LoadError::NonProductionBundle)
    );
    let mut json = serde_json::to_value(manifest()?)?;
    json.as_object_mut()
        .ok_or("manifest object")?
        .remove("build_class");
    let missing: Manifest = serde_json::from_value(json)?;
    let (bytes, digest) = assemble(&missing, &[], &spawn_raw())?;
    assert!(map::load(&bytes, &pinned(digest)).is_ok());
    let production = BundlePins {
        production: true,
        ..pinned(digest)
    };
    assert_eq!(
        map::load(&bytes, &production).err(),
        Some(LoadError::NonProductionBundle)
    );
    let compiled = build(BuildClass::Production)?;
    assert!(map::load(&compiled.bytes, &pins(&compiled, true)).is_ok());
    // Too short to hold a digest.
    assert!(refused_by_reader(map::load(
        &compiled.bytes[..31],
        &pins(&compiled, true)
    )));
    Ok(())
}

#[test]
fn map_load_refuses_corrupt_and_unknown_content_whole() -> TestResult {
    let compiled = build(BuildClass::NonProduction)?;
    let bytes = compiled.bytes.clone();
    // A corrupt sector frame with a recomputed digest fails its sector checksum.
    let mut corrupt = bytes.clone();
    let first = word(&bytes, 16 + word(&bytes, 8) + 6);
    corrupt[first] ^= 1;
    let (corrupt, digest) = resealed(corrupt);
    assert_eq!(
        map::load(&corrupt, &pinned(digest)).err(),
        Some(LoadError::Bundle(Error::Format("sector checksum".into())))
    );
    // The same bytes without the reseal claim another digest than the pinned one.
    let mut flipped = bytes.clone();
    flipped[first] ^= 1;
    assert!(map::load(&flipped, &pins(&compiled, false)).is_err());
    // An unknown top-level manifest key.
    let mut json = serde_json::to_value(manifest()?)?;
    json["surprise"] = true.into();
    let text = serde_json::to_vec(&json)?;
    let (unknown, digest) = with_manifest(&bytes, &text)?;
    assert!(refused_by_reader(map::load(&unknown, &pinned(digest))));
    // The same splice with the original manifest loads, so only the key refused it.
    let (same, digest) = with_manifest(&bytes, &serde_json::to_vec(&manifest()?)?)?;
    assert!(map::load(&same, &pinned(digest)).is_ok());
    // A palette index past the palette.
    let stray = sector::encode(&[tile(1, 1, vec![item(11, 0)])])?;
    let (bytes, digest) = assemble(&manifest()?, &[(-7, 0, 0, stray)], &spawn_raw())?;
    assert!(refused_by_reader(map::load(&bytes, &pinned(digest))));
    Ok(())
}

/// `bytes` with its manifest replaced by `json`: the header length and every table offset
/// moved, and the digest recomputed.
fn with_manifest(bytes: &[u8], json: &[u8]) -> Result<(Vec<u8>, [u8; 32]), Box<dyn StdError>> {
    let (length, count) = (word(bytes, 8), word(bytes, 12));
    let table = 16 + length;
    let delta = json.len() as i64 - length as i64;
    let mut out = bytes[..8].to_vec();
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&bytes[12..16]);
    out.extend_from_slice(json);
    let rows = &bytes[table..table + 50 * count + 44];
    let mut rows = rows.to_vec();
    for at in (0..count).map(|i| 50 * i + 6).chain([50 * count]) {
        let moved = (word(&rows, at) as i64 + delta) as u32;
        rows[at..at + 4].copy_from_slice(&moved.to_le_bytes());
    }
    out.extend_from_slice(&rows);
    out.extend_from_slice(&bytes[table + 50 * count + 44..]);
    Ok(resealed(out))
}

/// A manifest whose `ground_speed` of `key` is replaced by `speed` (and `walkable` by
/// `walkable`), assembled around the fixture's own sectors.
fn with_terrain(
    key: &str,
    walkable: bool,
    speed: u64,
) -> Result<(Vec<u8>, [u8; 32]), Box<dyn StdError>> {
    let bytes = build(BuildClass::NonProduction)?.bytes;
    let mut json = serde_json::to_value(manifest()?)?;
    let palette = json["palette"].as_array_mut().ok_or("palette")?;
    let entry = palette
        .iter_mut()
        .find(|entry| entry["key"] == key)
        .ok_or("palette key")?;
    entry["terrain"]["walkable"] = walkable.into();
    entry["terrain"]["ground_speed"] = speed.into();
    with_manifest(&bytes, &serde_json::to_vec(&json)?)
}

#[test]
fn map_load_ground_speed_bounds_are_enforced_by_the_reader() -> TestResult {
    // 0 non-walkable accepted; 0 walkable refused.
    let (bytes, digest) = with_terrain("terrain:void", false, 0)?;
    assert!(map::load(&bytes, &pinned(digest)).is_ok());
    let (bytes, digest) = with_terrain("terrain:void", true, 0)?;
    assert!(refused_by_reader(map::load(&bytes, &pinned(digest))));
    // 1,000 accepted; 1,001 refused, walkable or not.
    let (bytes, digest) = with_terrain("terrain:fast", true, 1000)?;
    assert_eq!(
        map::load(&bytes, &pinned(digest))?.ground_speed(7, 2, -7),
        1000
    );
    for walkable in [true, false] {
        let (bytes, digest) = with_terrain("terrain:fast", walkable, 1001)?;
        assert!(refused_by_reader(map::load(&bytes, &pinned(digest))));
    }
    // A non-walkable ground storing 120 loads; it paces at 0.
    let (bytes, digest) = with_terrain("terrain:lava", false, 120)?;
    let base = map::load(&bytes, &pinned(digest))?;
    assert_eq!(
        base.tile(3, 2, -7).ok_or("lava")?.stored_ground_speed(),
        120
    );
    assert_eq!(base.ground_speed(3, 2, -7), 0);
    Ok(())
}

/// `MAP01-TILE-BASE-ENTRIES` (64 top-level), `MAP01-TILE-ENTRIES` (4,096) and
/// `MAP01-TILE-TEXT-BYTES` (4,096) at the maximum and one more, at their real values.
#[test]
fn map_load_tile_caps_accept_the_maximum_and_refuse_one_more() -> TestResult {
    let manifest = manifest()?;
    let load = |payload: Vec<u8>| -> Result<Result<WorldBase, LoadError>, Box<dyn StdError>> {
        let (bytes, digest) = assemble(&manifest, &[(-7, 0, 0, payload)], &spawn_raw())?;
        Ok(map::load(&bytes, &pinned(digest)))
    };
    let top = load(sector::encode(&[tile(1, 1, vec![item(0, 0); 64])])?)?;
    let top = top?;
    assert_eq!(top.tile(1, 1, -7).ok_or("tile")?.ids().len(), 64);
    assert!(refused_at_limit(load(sector::encode(&[tile(
        1,
        1,
        vec![item(0, 0); 65]
    )])?)?));
    // One container holding 4,095 items: 4,096 entries, one of them top-level.
    let mut full = vec![item(6, 0)];
    full.extend(std::iter::repeat_n(item(7, 1), 4095));
    let base = load(sector::encode(&[tile(1, 1, full.clone())])?)?;
    assert_eq!(base?.entry_count(), 4096);
    full.push(item(7, 1));
    assert!(refused_at_limit(load(sector::encode(&[tile(
        1, 1, full
    )])?)?));
    // Text: exactly 4,096 bytes, and one more.
    let mut texted = item(6, 0);
    texted.attrs.text = Some("a".repeat(4096));
    assert!(load(sector::encode(&[tile(1, 1, vec![texted.clone()])])?)?.is_ok());
    texted.attrs.text = Some("a".repeat(4097));
    assert!(refused_at_limit(load(sector::encode(&[tile(
        1,
        1,
        vec![texted]
    )])?)?));
    Ok(())
}

/// The `MAP01-BUNDLE-*` caps, hit with reduced maxima on the fixture (the registered values are
/// the defaults, asserted below): exactly the maximum loads, one more is a `Limit`.
#[test]
fn map_load_bundle_caps_accept_the_maximum_and_refuse_one_more() -> TestResult {
    let compiled = build(BuildClass::NonProduction)?;
    let (bytes, pins) = (&compiled.bytes, pins(&compiled, false));
    let (count, table) = (word(bytes, 12), 16 + word(bytes, 8));
    let raws: Vec<usize> = (0..count)
        .map(|i| word(bytes, table + 50 * i + 14))
        .collect();
    let spawn = word(bytes, table + 50 * count + 8);
    let largest = *raws.iter().max().ok_or("rows")?;
    let total = raws.iter().sum::<usize>() + spawn;
    let base = map::load(bytes, &pins)?;
    let (tiles, entries) = (base.tile_count(), base.entry_count());
    let exact = LoadLimits {
        caps: ReadCaps {
            file_bytes: bytes.len(),
            sectors: count,
            sector_raw_bytes: largest,
            total_raw_bytes: total,
        },
        budget: Budget { tiles, entries },
    };
    assert!(map::load_with(bytes, &pins, exact).is_ok());
    let over = |limits| refused_at_limit(map::load_with(bytes, &pins, limits));
    let caps = exact.caps;
    assert!(over(LoadLimits {
        caps: ReadCaps {
            file_bytes: bytes.len() - 1,
            ..caps
        },
        ..exact
    }));
    assert!(over(LoadLimits {
        caps: ReadCaps {
            sectors: count - 1,
            ..caps
        },
        ..exact
    }));
    assert!(over(LoadLimits {
        caps: ReadCaps {
            sector_raw_bytes: largest - 1,
            ..caps
        },
        ..exact
    }));
    assert!(over(LoadLimits {
        caps: ReadCaps {
            total_raw_bytes: total - 1,
            ..caps
        },
        ..exact
    }));
    assert!(over(LoadLimits {
        budget: Budget {
            tiles: tiles - 1,
            entries
        },
        ..exact
    }));
    assert!(over(LoadLimits {
        budget: Budget {
            tiles,
            entries: entries - 1
        },
        ..exact
    }));
    // The manifest maximum (16 MiB) and the sector-count maximum (2^20), from the header fields.
    let patched = |at: usize, value: u32| {
        let mut out = bytes.clone();
        out[at..at + 4].copy_from_slice(&value.to_le_bytes());
        resealed(out)
    };
    let (manifest, digest) = patched(8, (16 << 20) + 1);
    assert!(refused_at_limit(map::load(&manifest, &pinned(digest))));
    let (sectors, digest) = patched(12, (1 << 20) + 1);
    assert!(refused_at_limit(map::load(&sectors, &pinned(digest))));
    // The ratio maximum (1,024): a raw length past it is refused before decompression.
    let compressed = word(bytes, table + 10);
    let (ratio, digest) = patched(table + 14, (compressed * 1024 + 1) as u32);
    assert!(refused_at_limit(map::load(&ratio, &pinned(digest))));
    // A file over the real 1 GiB maximum is refused before anything is parsed.
    let mut huge = vec![0u8; bundle::MAX_FILE_BYTES + 1];
    let at = huge.len() - 32;
    huge[at..].copy_from_slice(&compiled.digest);
    assert!(refused_at_limit(map::load(&huge, &pins)));
    // The defaults are the registered maxima.
    assert_eq!(LOAD_LIMITS.caps.file_bytes, 1 << 30);
    assert_eq!(LOAD_LIMITS.caps.sectors, 1 << 20);
    assert_eq!(LOAD_LIMITS.caps.sector_raw_bytes, 16 << 20);
    assert_eq!(LOAD_LIMITS.caps.total_raw_bytes, 1 << 30);
    assert_eq!(
        (LOAD_LIMITS.budget.tiles, LOAD_LIMITS.budget.entries),
        (1 << 25, 1 << 26)
    );
    Ok(())
}

/// A bounded property test over the reader (ADR-0021 §4.8): every resealed one-byte change of
/// the fixture, and seeded multi-byte changes of its header and table set to boundary values,
/// load or are refused with a typed error and never panic. Allocation stays bounded because
/// every declared length is checked against its cap before anything is allocated for it
/// (asserted at the caps by the two cap tests above).
#[test]
fn map_load_reader_property_never_panics() -> TestResult {
    let compiled = build(BuildClass::NonProduction)?;
    let bytes = compiled.bytes.clone();
    let check = |candidate: Vec<u8>| {
        let (candidate, digest) = resealed(candidate);
        let _ = map::load(&candidate, &pinned(digest));
    };
    let body = bytes.len() - 32;
    for at in 0..body {
        for mask in [0x01u8, 0x80, 0xFF] {
            let mut flipped = bytes.clone();
            flipped[at] ^= mask;
            check(flipped);
        }
    }
    // Header and table words set to boundary values, deterministically seeded.
    let table = 16 + word(&bytes, 8);
    let rows = table + 50 * word(&bytes, 12) + 44;
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let values = [
        0u32,
        1,
        0xFF,
        0xFFFF,
        1 << 20,
        (1 << 20) + 1,
        16 << 20,
        (16 << 20) + 1,
        u32::MAX,
    ];
    for _ in 0..2000 {
        let mut mutated = bytes.clone();
        for _ in 0..1 + next() % 3 {
            let at = (next() as usize) % (rows - 3);
            let value = values[(next() as usize) % values.len()];
            mutated[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        check(mutated);
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// ADR-0021 §4.8 budget measurement (MAP-LOAD-1 evidence). Not part of the gate: both tests are
// ignored and run by hand, in release mode, on the checked-in World Project:
//
//   MAP_LOAD_BUDGET_BUNDLE=/tmp/real.otwb cargo test --locked --release -p oteryn-game-server \
//       --test map_load_base -- --ignored --exact map_load_budget_compile_the_real_map
//   MAP_LOAD_BUDGET_BUNDLE=/tmp/real.otwb cargo test --locked --release -p oteryn-game-server \
//       --test map_load_base -- --ignored --exact map_load_budget_measure
//
// The first compiles every placement region of `content/world/placements` with a measurement
// resolver (below); the second, in a fresh process, loads that bundle and reports the load time,
// the resident memory of the base and the 18x14 viewport p99.

/// A resolver for the measurement only: every palette key of the real map resolves, so every
/// one of its tiles and entries is in the bundle. A key whose Terrain record has a known kind
/// takes that kind (a ground its known walkability and ground speed, else walkable at 150); every
/// other key, provisional keys and the Terrain records whose kind is still `UNKNOWN` included,
/// is a plain Item.
struct Measurement {
    terrain: std::collections::HashMap<String, Terrain>,
}

impl Measurement {
    fn new(root: &std::path::Path) -> Result<Self, Box<dyn StdError>> {
        let mut terrain = std::collections::HashMap::new();
        for entry in std::fs::read_dir(root.join("content/world/terrain"))? {
            let path = entry?.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if !name.starts_with("terrain-") {
                continue;
            }
            let shard: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
            for record in shard["records"].as_array().ok_or("records")? {
                let known = |field: &str| {
                    (record[field]["state"] == "KNOWN").then(|| record[field]["value"].clone())
                };
                let kind = match known("kind").as_ref().and_then(|v| v.as_str()) {
                    Some("ground") => TerrainKind::Ground,
                    Some("border") => TerrainKind::Border,
                    Some("wall") => TerrainKind::Wall,
                    Some("roof") => TerrainKind::Roof,
                    Some("field") => TerrainKind::Field,
                    Some("common") => TerrainKind::Common,
                    _ => continue,
                };
                let item = record["provenance"]["item_pointer"]["key"]
                    .as_str()
                    .ok_or("item pointer")?;
                let (walkable, ground_speed) = if kind == TerrainKind::Ground {
                    let walkable = known("walkable").and_then(|v| v.as_bool()).unwrap_or(true);
                    let speed = known("ground_speed")
                        .and_then(|v| v.as_u64())
                        .and_then(|v| u16::try_from(v).ok())
                        .filter(|v| *v <= 1000 && (*v >= 1 || !walkable))
                        .unwrap_or(150);
                    (Some(walkable), Some(speed))
                } else {
                    (None, None)
                };
                terrain.insert(
                    item.to_owned(),
                    Terrain {
                        kind,
                        walkable,
                        ground_speed,
                    },
                );
            }
        }
        Ok(Self { terrain })
    }

    fn id(key: &str) -> u32 {
        let digest = Sha256::digest(key.as_bytes());
        u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]) & 0x7FFF_FFFF
    }
}

impl KeyResolver for Measurement {
    fn resolve(&self, key: &str) -> Resolution {
        let family = if self.terrain.contains_key(key) {
            Family::Terrain
        } else {
            Family::Item
        };
        Resolution::Resolved(family, Self::id(key))
    }

    fn terrain(&self, key: &str) -> Result<Option<Terrain>, Error> {
        Ok(self.terrain.get(key).copied())
    }

    fn floor_change(&self, _: &str) -> bool {
        false
    }
}

fn budget_path() -> Result<std::path::PathBuf, Box<dyn StdError>> {
    Ok(std::env::var_os("MAP_LOAD_BUDGET_BUNDLE")
        .ok_or("set MAP_LOAD_BUDGET_BUNDLE to the bundle path")?
        .into())
}

#[test]
#[ignore = "budget measurement (ADR-0021 §4.8), run by hand in release mode"]
fn map_load_budget_compile_the_real_map() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/placements/index.json"),
    )?)?;
    let regions = index["shards"]
        .as_array()
        .ok_or("shards")?
        .iter()
        .map(|path| {
            Ok(std::fs::read(
                root.join(path.as_str().ok_or("shard path")?),
            )?)
        })
        .collect::<Result<Vec<_>, Box<dyn StdError>>>()?;
    let palette: Vec<String> = index["palette"]
        .as_array()
        .ok_or("palette")?
        .iter()
        .map(|entry| entry["key"].as_str().map(String::from).ok_or("key"))
        .collect::<Result<_, _>>()?;
    let world = oteryn_world_bundle_compiler::project::world_extent(&std::fs::read(
        root.join("content/world/worlds/worlds-00000-00000.json"),
    )?)?;
    let resolver = Measurement::new(&root)?;
    // The checked families, as the compiler binary reads them.
    let shards = |directory: &str, prefix: &str| -> Result<Vec<Vec<u8>>, Box<dyn StdError>> {
        let mut paths: Vec<_> = std::fs::read_dir(root.join(directory))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<_, _>>()?;
        paths.retain(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".json"))
        });
        paths.sort();
        Ok(paths.iter().map(std::fs::read).collect::<Result<_, _>>()?)
    };
    let mut families = Families::default();
    for shard in shards("content/world/transitions", "teleports-")? {
        families.add_teleports(&shard)?;
    }
    for shard in shards("content/houses", "houses-")? {
        families.add_houses(&shard)?;
    }
    for shard in shards("content/creatures/definitions", "creatures-")? {
        families.add_creatures(&shard)?;
    }
    for shard in shards("content/world/spawns", "spawns-")? {
        families.add_spawns(&shard)?;
    }
    let mut input = input(&regions, &palette, BuildClass::Production, &families);
    input.world = world;
    let compiled = compile(&input, &resolver)?;
    std::fs::write(budget_path()?, &compiled.bytes)?;
    println!(
        "bundle bytes {} digest {}",
        compiled.bytes.len(),
        compiled
            .digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    Ok(())
}

/// A `kB` field of `/proc/self/status`, in bytes.
fn status_bytes(field: &str) -> Result<usize, Box<dyn StdError>> {
    let status = std::fs::read_to_string("/proc/self/status")?;
    let line = status
        .lines()
        .find(|line| line.starts_with(field))
        .ok_or("status field")?;
    let kb: usize = line
        .trim_start_matches(field)
        .trim()
        .trim_end_matches("kB")
        .trim()
        .parse()?;
    Ok(kb * 1024)
}

#[test]
#[ignore = "budget measurement (ADR-0021 §4.8), run by hand in release mode"]
fn map_load_budget_measure() -> TestResult {
    let bytes = std::fs::read(budget_path()?)?;
    let digest: [u8; 32] = bytes[bytes.len() - 32..].try_into()?;
    let pins = BundlePins {
        digest,
        project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
        world_schema_version: "world-schema-1".into(),
        content_revision: "rev-1".into(),
        production: true,
    };
    let before = status_bytes("VmRSS:")?;
    let mut runs = Vec::new();
    let mut kept = None;
    for _ in 0..5 {
        drop(kept.take());
        let start = std::time::Instant::now();
        let base = map::load(&bytes, &pins)?;
        runs.push(start.elapsed().as_millis());
        kept = Some(base);
    }
    let base = kept.ok_or("loaded")?;
    let peak = status_bytes("VmHWM:")?;
    drop(bytes);
    let resident = status_bytes("VmRSS:")?;
    println!("tiles {} entries {}", base.tile_count(), base.entry_count());
    println!("load ms {runs:?}");
    println!(
        "rss before {before} after load (bundle bytes dropped) {resident} base delta {} peak {peak}",
        resident.saturating_sub(before)
    );
    // Viewports: 18x14 around a seeded sample of real tiles on floor -7 (legacy z 7), over every
    // floor a client there sees (legacy z 7 down to 0, native -7..=0), each floor above shifted
    // by its height like the client's perspective.
    let centres: Vec<(u16, u16)> = base
        .tiles()
        .filter(|(floor, ..)| *floor == -7)
        .map(|(_, x, y, _)| (x, y))
        .collect();
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut times = Vec::with_capacity(20_000);
    let mut touched = 0u64;
    for _ in 0..20_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let (cx, cy) = centres[(state % centres.len() as u64) as usize];
        let start = std::time::Instant::now();
        for floor in -7i8..=0 {
            let shift = i32::from(floor + 7);
            for dy in -6i32..8 {
                for dx in -8i32..10 {
                    let x = i32::from(cx) + dx - shift;
                    let y = i32::from(cy) + dy - shift;
                    let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
                        continue;
                    };
                    if let Some(tile) = base.tile(x, y, floor) {
                        touched += u64::from(tile.ids().iter().fold(0, |a, id| a ^ id));
                        touched += u64::from(tile.ground_speed());
                    }
                }
            }
        }
        times.push(start.elapsed().as_nanos());
    }
    times.sort_unstable();
    let at = |q: f64| times[((times.len() as f64 - 1.0) * q) as usize];
    println!(
        "viewport ns p50 {} p99 {} max {} (checksum {touched})",
        at(0.5),
        at(0.99),
        times[times.len() - 1]
    );
    Ok(())
}
