//! MAP-BUNDLE-2: terrain semantics in the bundle (format v2, decision §1.4): the fail-closed
//! compiler resolution and the fail-closed reader.

use std::error::Error as StdError;

use oteryn_world_bundle_compiler::Error;
use oteryn_world_bundle_compiler::bundle::{
    self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Terrain, TerrainKind,
};
use oteryn_world_bundle_compiler::compile::{Input, KeyResolver, Resolution, compile};
use oteryn_world_bundle_compiler::project::Families;
use oteryn_world_bundle_compiler::resolve::Registry;
use oteryn_world_bundle_compiler::sector::{self, Attrs, Item, Tile};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn StdError>>;
type Change = Box<dyn Fn(&mut Value)>;

fn reference(family: &str, key: &str) -> Value {
    json!({"family": family, "key": key, "revision": "definition-r1"})
}

fn field(value: Option<Value>) -> Value {
    match value {
        Some(value) => json!({"state": "KNOWN", "value": value}),
        None => json!({"state": "UNKNOWN"}),
    }
}

/// A Terrain catalogue record pointing at `item` (or at no Item).
fn terrain_record(
    key: &str,
    item: Option<&str>,
    kind: Option<&str>,
    walkable: Option<bool>,
    speed: Option<u64>,
) -> Value {
    let pointer = item.map(|item| json!({"item_pointer": reference("Item", item)}));
    json!({
        "identity": reference("Terrain", key),
        "provenance": pointer.unwrap_or_else(|| json!({})),
        "kind": field(kind.map(Value::from)),
        "walkable": field(walkable.map(Value::from)),
        "ground_speed": field(speed.map(Value::from)),
    })
}

fn shard(family: &str, records: Vec<Value>) -> Vec<u8> {
    json!({"family": family, "records": records})
        .to_string()
        .into_bytes()
}

fn item_shard(keys: &[&str]) -> Vec<u8> {
    let records = keys
        .iter()
        .map(|key| json!({"definition": {"identity": reference("Item", key)}}))
        .collect();
    shard("Item", records)
}

/// A sealed registry: Items for every `item:` key below, plus the given Terrain records and a
/// WorldObject whose record has an UNKNOWN kind and a junk speed.
fn registry(terrain: Vec<Value>) -> Result<Registry, Error> {
    let mut registry = Registry::default();
    let mut items = vec!["item:plain", "item:crate"];
    items.extend(
        terrain
            .iter()
            .filter_map(|record| record["provenance"]["item_pointer"]["key"].as_str()),
    );
    // The Item shard wants distinct keys; the pointers above are distinct by construction.
    let owned: Vec<String> = items.iter().map(|key| (*key).to_owned()).collect();
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    registry.add_items(&item_shard(&refs))?;
    registry.add_catalogue("Terrain", &shard("Terrain", terrain))?;
    let object = json!({
        "identity": reference("WorldObject", "object:crate"),
        "provenance": {"item_pointer": reference("Item", "item:crate")},
        "kind": field(None),
        "walkable": field(Some(Value::from("yes"))),
        "ground_speed": field(Some(Value::from(99_999))),
    });
    registry.add_catalogue("WorldObject", &shard("WorldObject", vec![object]))?;
    registry.seal()?;
    Ok(registry)
}

fn every_kind() -> Vec<Value> {
    vec![
        terrain_record(
            "terrain:grass",
            Some("item:grass"),
            Some("ground"),
            Some(true),
            Some(150),
        ),
        terrain_record(
            "terrain:pit",
            Some("item:pit"),
            Some("ground"),
            Some(false),
            Some(0),
        ),
        terrain_record(
            "terrain:swamp",
            Some("item:swamp"),
            Some("ground"),
            Some(false),
            Some(120),
        ),
        terrain_record(
            "terrain:fast",
            Some("item:fast"),
            Some("ground"),
            Some(true),
            Some(1000),
        ),
        terrain_record(
            "terrain:edge",
            Some("item:edge"),
            Some("border"),
            Some(true),
            Some(7),
        ),
        terrain_record("terrain:direct", None, Some("wall"), None, None),
        terrain_record("terrain:roof", Some("item:roof"), Some("roof"), None, None),
        terrain_record("terrain:fire", Some("item:fire"), Some("field"), None, None),
        // A `common` record keeps its own walkable fact in the catalogue; the bundle drops it.
        terrain_record(
            "terrain:mosaic",
            Some("item:mosaic"),
            Some("common"),
            Some(true),
            None,
        ),
    ]
}

/// Placed in this order; the index is the source palette index.
const KEYS: [&str; 11] = [
    "item:plain",
    "item:crate",
    "item:grass",
    "item:pit",
    "item:swamp",
    "item:fast",
    "item:edge",
    "terrain:direct",
    "item:roof",
    "item:fire",
    "item:mosaic",
];

fn region(z: u8, tiles: &[Tile]) -> Result<Vec<u8>, Box<dyn StdError>> {
    let frame = zstd::bulk::compress(&sector::encode(tiles)?, 3)?;
    let mut out = b"OTRB".to_vec();
    out.extend_from_slice(&[1, z]);
    for value in [0u16, 0, 1] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.push(0);
    out.extend_from_slice(&21u32.to_le_bytes());
    out.extend_from_slice(&(frame.len() as u32).to_le_bytes());
    out.extend_from_slice(&frame);
    Ok(out)
}

fn tile_of(palette: &[u32]) -> Tile {
    Tile {
        x: 1,
        y: 2,
        flags: 0,
        house: 0,
        zones: Vec::new(),
        items: palette
            .iter()
            .map(|palette| Item {
                palette: *palette,
                depth: 0,
                attrs: Attrs::default(),
            })
            .collect(),
    }
}

fn compile_keys(registry: &Registry, keys: &[&str]) -> Result<Vec<u8>, Error> {
    let palette: Vec<String> = keys.iter().map(|key| (*key).to_owned()).collect();
    let indices: Vec<u32> = (0..keys.len() as u32).collect();
    let regions = vec![region(7, &[tile_of(&indices)]).map_err(|e| Error::Format(e.to_string()))?];
    let families = Families::default();
    let input = Input {
        regions: &regions,
        palette: &palette,
        identity: Identity {
            content_revision: "rev-1".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 64,
            max_y: 64,
            floors: vec![-7],
        },
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &families,
    };
    Ok(compile(&input, registry)?.bytes)
}

fn ground(walkable: bool, speed: u16) -> Terrain {
    Terrain {
        kind: TerrainKind::Ground,
        walkable: Some(walkable),
        ground_speed: Some(speed),
    }
}

fn other(kind: TerrainKind) -> Terrain {
    Terrain {
        kind,
        walkable: None,
        ground_speed: None,
    }
}

fn fixture_bytes() -> Result<Vec<u8>, Error> {
    compile_keys(&registry(every_kind())?, &KEYS)
}

#[test]
fn one_entry_of_each_kind_compiles_into_its_terrain_values() -> TestResult {
    let bytes = fixture_bytes()?;
    assert_eq!(bytes, fixture_bytes()?, "two builds are byte-identical");
    let read = bundle::read(&bytes)?;
    assert_eq!(read.manifest.format, "OTERYN_WORLD_BUNDLE/v3");
    assert_eq!(read.manifest.min_reader_version, 3);
    let terrain = |key: &str| {
        read.manifest
            .palette
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.terrain)
    };
    // A plain Item and an Item routed to a WorldObject are null, however the object record reads.
    assert_eq!(terrain("item:plain"), Some(None));
    assert_eq!(terrain("item:crate"), Some(None));
    assert_eq!(terrain("item:grass"), Some(Some(ground(true, 150))));
    assert_eq!(terrain("item:pit"), Some(Some(ground(false, 0))));
    assert_eq!(terrain("item:swamp"), Some(Some(ground(false, 120))));
    assert_eq!(terrain("item:fast"), Some(Some(ground(true, 1000))));
    // A non-ground record's own walkable and speed are not carried.
    assert_eq!(terrain("item:edge"), Some(Some(other(TerrainKind::Border))));
    assert_eq!(
        terrain("terrain:direct"),
        Some(Some(other(TerrainKind::Wall)))
    );
    assert_eq!(terrain("item:roof"), Some(Some(other(TerrainKind::Roof))));
    assert_eq!(terrain("item:fire"), Some(Some(other(TerrainKind::Field))));
    assert_eq!(
        terrain("item:mosaic"),
        Some(Some(other(TerrainKind::Common)))
    );
    Ok(())
}

#[test]
fn the_compiler_refuses_unclassified_terrain_routes() -> TestResult {
    let refused = |record: Value| {
        let registry = registry(vec![record])?;
        let result = compile_keys(&registry, &["item:bad"]);
        assert!(matches!(result, Err(Error::Key(_))), "{result:?}");
        Ok::<(), Error>(())
    };
    let bad = |kind, walkable, speed| {
        terrain_record("terrain:bad", Some("item:bad"), kind, walkable, speed)
    };
    // An UNKNOWN kind, and a KNOWN kind that is not a Terrain kind.
    refused(bad(None, Some(true), Some(150)))?;
    refused(bad(Some("door"), None, None))?;
    // A ground with an UNKNOWN walkable or ground_speed.
    refused(bad(Some("ground"), None, Some(150)))?;
    refused(bad(Some("ground"), Some(true), None))?;
    // A speed over 1000, and a walkable ground with speed 0.
    refused(bad(Some("ground"), Some(true), Some(1001)))?;
    refused(bad(Some("ground"), Some(false), Some(1001)))?;
    refused(bad(Some("ground"), Some(true), Some(0)))?;
    // The same through a direct Terrain key.
    let direct = terrain_record("terrain:bad", None, None, None, None);
    let result = compile_keys(&registry(vec![direct])?, &["terrain:bad"]);
    assert!(matches!(result, Err(Error::Key(_))));
    // An unplaced unclassified record does not stop the compile.
    let mut records = every_kind();
    records.push(bad(None, None, None));
    compile_keys(&registry(records)?, &["item:grass"])?;
    Ok(())
}

#[test]
fn a_terrain_family_entry_needs_terrain_in_the_writer_and_the_reader() -> TestResult {
    struct Silent;
    impl KeyResolver for Silent {
        fn resolve(&self, key: &str) -> Resolution {
            match key {
                "terrain:x" => Resolution::Resolved(Family::Terrain, 0),
                _ => Resolution::Unknown,
            }
        }
        fn terrain(&self, _: &str) -> Result<Option<Terrain>, Error> {
            Ok(None)
        }

        fn floor_change(&self, _: &str) -> bool {
            false
        }
    }
    let palette = vec!["terrain:x".to_owned()];
    let regions = vec![region(7, &[tile_of(&[0])])?];
    let families = Families::default();
    let input = Input {
        regions: &regions,
        palette: &palette,
        identity: Identity::default(),
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 64,
            max_y: 64,
            floors: vec![-7],
        },
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &families,
    };
    // The writer refuses a Terrain route without terrain.
    assert!(matches!(compile(&input, &Silent), Err(Error::Format(_))));
    // The reader refuses it too, in a bundle written elsewhere.
    let bytes = fixture_bytes()?;
    let mutated = with_manifest(&bytes, 3, |m| {
        m["palette"][0]["family"] = "terrain".into();
        m["palette"][0]["terrain"] = Value::Null;
    })?;
    assert!(matches!(bundle::read(&mutated), Err(Error::Format(_))));
    // The writer refuses the same manifest.
    let read = bundle::read(&bytes)?;
    let mut manifest = read.manifest.clone();
    manifest.palette[0].family = Family::Terrain;
    assert!(bundle::write(&manifest, &read.sectors, &read.spawns).is_err());
    Ok(())
}

/// Rewrites the manifest of `bytes` with `change`, fixes the length and table offsets and
/// reseals the digest. `version` and the digest domain follow `version`.
fn with_manifest(
    bytes: &[u8],
    version: u16,
    change: impl FnOnce(&mut Value),
) -> Result<Vec<u8>, Box<dyn StdError>> {
    let length = u32::from_le_bytes(bytes[8..12].try_into()?) as usize;
    let count = u32::from_le_bytes(bytes[12..16].try_into()?) as usize;
    let mut manifest: Value = serde_json::from_slice(&bytes[16..16 + length])?;
    change(&mut manifest);
    let json = serde_json::to_vec(&manifest)?;
    let delta = json.len() as i64 - length as i64;
    let mut out = bytes[..16].to_vec();
    out[4..6].copy_from_slice(&version.to_le_bytes());
    out[8..12].copy_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&json);
    out.extend_from_slice(&bytes[16 + length..bytes.len() - 32]);
    let table = 16 + json.len();
    // The sector rows, then the spawn row (offset at its start).
    let rows = (0..count)
        .map(|row| table + 50 * row + 6)
        .chain([table + 50 * count]);
    for at in rows {
        let offset = u32::from_le_bytes(out[at..at + 4].try_into()?) as i64 + delta;
        out[at..at + 4].copy_from_slice(&(offset as u32).to_le_bytes());
    }
    let mut hash = <sha2::Sha256 as sha2::Digest>::new();
    sha2::Digest::update(
        &mut hash,
        format!("OTERYN_WORLD_BUNDLE/v{version}\0").as_bytes(),
    );
    sha2::Digest::update(&mut hash, &out);
    out.extend_from_slice(&sha2::Digest::finalize(hash));
    Ok(out)
}

/// The palette index of `key` in the fixture bundle.
fn index(bytes: &[u8], key: &str) -> Result<usize, Box<dyn StdError>> {
    let manifest = bundle::read(bytes)?.manifest;
    manifest
        .palette
        .iter()
        .position(|entry| entry.key == key)
        .ok_or_else(|| "key".into())
}

#[test]
fn the_reader_accepts_the_fixture_and_rejects_v1_and_malformed_terrain() -> TestResult {
    let bytes = fixture_bytes()?;
    // The unchanged round trip through the helper still reads (key order aside).
    bundle::read(&with_manifest(&bytes, 3, |_| {})?)?;
    bundle::read(&bytes)?;
    // A v1 bundle is refused like any unknown version.
    let v1 = with_manifest(&bytes, 1, |m| {
        m["format"] = "OTERYN_WORLD_BUNDLE/v1".into();
        m["min_reader_version"] = 1.into();
    })?;
    assert!(matches!(bundle::read(&v1), Err(Error::Format(_))));
    // A v2 bundle is refused: v3 has no dual reading.
    let v2 = with_manifest(&bytes, 2, |m| {
        m["format"] = "OTERYN_WORLD_BUNDLE/v2".into();
        m["min_reader_version"] = 2.into();
    })?;
    assert!(matches!(bundle::read(&v2), Err(Error::Format(_))));
    // A v3 header over a manifest that claims v1.
    let claims_v1 = with_manifest(&bytes, 3, |m| {
        m["format"] = "OTERYN_WORLD_BUNDLE/v1".into();
    })?;
    assert!(bundle::read(&claims_v1).is_err());
    let (grass, edge, plain, mosaic) = (
        index(&bytes, "item:grass")?,
        index(&bytes, "item:edge")?,
        index(&bytes, "item:plain")?,
        index(&bytes, "item:mosaic")?,
    );
    let malformed: Vec<(&str, Change)> = vec![
        (
            "missing terrain",
            Box::new(move |m| {
                m["palette"][plain]
                    .as_object_mut()
                    .map(|e| e.remove("terrain"));
            }),
        ),
        (
            "missing walkable",
            Box::new(move |m| {
                m["palette"][grass]["terrain"]
                    .as_object_mut()
                    .map(|t| t.remove("walkable"));
            }),
        ),
        (
            "missing ground_speed",
            Box::new(move |m| {
                m["palette"][grass]["terrain"]
                    .as_object_mut()
                    .map(|t| t.remove("ground_speed"));
            }),
        ),
        (
            "ground without walkable",
            Box::new(move |m| {
                m["palette"][grass]["terrain"]["walkable"] = Value::Null;
            }),
        ),
        (
            "ground without speed",
            Box::new(move |m| {
                m["palette"][grass]["terrain"]["ground_speed"] = Value::Null;
            }),
        ),
        (
            "border with walkable",
            Box::new(move |m| {
                m["palette"][edge]["terrain"]["walkable"] = true.into();
            }),
        ),
        (
            "common with walkable",
            Box::new(move |m| {
                m["palette"][mosaic]["terrain"]["walkable"] = true.into();
            }),
        ),
        (
            "common with speed",
            Box::new(move |m| {
                m["palette"][mosaic]["terrain"]["ground_speed"] = 150.into();
            }),
        ),
        (
            "border with speed",
            Box::new(move |m| {
                m["palette"][edge]["terrain"]["ground_speed"] = 150.into();
            }),
        ),
        (
            "unknown kind",
            Box::new(move |m| {
                m["palette"][grass]["terrain"]["kind"] = "water".into();
            }),
        ),
        (
            "kind not a string",
            Box::new(move |m| {
                m["palette"][grass]["terrain"]["kind"] = Value::Null;
            }),
        ),
        (
            "speed 1001",
            Box::new(move |m| {
                m["palette"][grass]["terrain"]["ground_speed"] = 1001.into();
            }),
        ),
        (
            "speed negative",
            Box::new(move |m| {
                m["palette"][grass]["terrain"]["ground_speed"] = (-1).into();
            }),
        ),
        (
            "speed fractional",
            Box::new(move |m| {
                m["palette"][grass]["terrain"]["ground_speed"] = 150.5.into();
            }),
        ),
        (
            "speed text",
            Box::new(move |m| {
                m["palette"][grass]["terrain"]["ground_speed"] = "150".into();
            }),
        ),
        (
            "walkable speed 0",
            Box::new(move |m| {
                m["palette"][grass]["terrain"]["ground_speed"] = 0.into();
            }),
        ),
        (
            "walkable text",
            Box::new(move |m| {
                m["palette"][grass]["terrain"]["walkable"] = "true".into();
            }),
        ),
        (
            "unknown member",
            Box::new(move |m| {
                m["palette"][grass]["terrain"]["sight"] = true.into();
            }),
        ),
        (
            "terrain not an object",
            Box::new(move |m| {
                m["palette"][grass]["terrain"] = "ground".into();
            }),
        ),
    ];
    for (what, change) in malformed {
        let mutated = with_manifest(&bytes, 3, change)?;
        assert!(bundle::read(&mutated).is_err(), "{what} was accepted");
    }
    // A non-walkable nonzero speed and a non-walkable zero speed are valid.
    let pit = index(&bytes, "item:pit")?;
    let valid = with_manifest(&bytes, 3, move |m| {
        m["palette"][pit]["terrain"]["ground_speed"] = 120.into();
    })?;
    bundle::read(&valid)?;
    // The parse still rejects a v2 manifest that the writer would not have written.
    let read = bundle::read(&bytes)?;
    let mut manifest: Manifest = read.manifest.clone();
    manifest.palette[grass] = PaletteEntry {
        terrain: Some(ground(true, 0)),
        ..manifest.palette[grass].clone()
    };
    assert!(bundle::write(&manifest, &read.sectors, &read.spawns).is_err());
    // The writer refuses `common` with a non-null member as well.
    let mosaic = index(&bytes, "item:mosaic")?;
    manifest.palette[grass] = read.manifest.palette[grass].clone();
    manifest.palette[mosaic] = PaletteEntry {
        terrain: Some(Terrain {
            walkable: Some(true),
            ..other(TerrainKind::Common)
        }),
        ..manifest.palette[mosaic].clone()
    };
    assert!(bundle::write(&manifest, &read.sectors, &read.spawns).is_err());
    Ok(())
}

#[test]
fn the_parity_report_counts_placed_entries_by_class() -> TestResult {
    let mut records = every_kind();
    records.push(terrain_record(
        "terrain:bad",
        Some("item:bad"),
        None,
        None,
        None,
    ));
    let registry = registry(records)?;
    let mut keys = KEYS.to_vec();
    keys.push("item:bad");
    keys.push("donor:unplaced");
    let counts = registry.terrain_counts(keys.iter().copied());
    assert_eq!(counts.by_kind.get("ground"), Some(&4));
    assert_eq!(counts.by_kind.get("border"), Some(&1));
    assert_eq!(counts.by_kind.get("wall"), Some(&1));
    assert_eq!(counts.by_kind.get("roof"), Some(&1));
    assert_eq!(counts.by_kind.get("field"), Some(&1));
    assert_eq!(counts.by_kind.get("common"), Some(&1));
    assert_eq!((counts.world_object, counts.plain_item), (1, 1));
    assert_eq!((counts.unknown_kind, counts.refused), (1, 1));
    Ok(())
}
