#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use oteryn_world_bundle_compiler::bundle::{BuildClass, Extent, Family, Identity};
use oteryn_world_bundle_compiler::compile::{Input, compile};
use oteryn_world_bundle_compiler::project::Families;
use oteryn_world_bundle_compiler::resolve::Registry;
use oteryn_world_bundle_compiler::sector::{self, Attrs, Item, Tile};
use std::path::{Path, PathBuf};

const PRODUCTION_KEYS: &[u8] =
    include_bytes!("../../../../tools/content-schema/native-gameplay/item-keys.json");

fn document(records: &[(&str, &str)]) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "schema": ITEM_KEYS_SCHEMA,
        "records": records,
    }))
    .unwrap()
}

fn reference(index: &ItemDefinitionIndex, key: &str, revision: &str) -> Option<u32> {
    index
        .definition_ref("Item", key, revision)
        .map(NonZeroU32::get)
}

#[test]
fn reference_is_one_plus_the_ascending_byte_order_item_index() {
    // Byte order, not numeric order: "i1000" sorts before "i200".
    let index = ItemDefinitionIndex::decode(&document(&[
        ("oteryn:item.tibia.i100", "definition-r1"),
        ("oteryn:item.tibia.i1000", "definition-r1"),
        ("oteryn:item.tibia.i200", "definition-r2"),
    ]))
    .unwrap();
    assert_eq!(
        reference(&index, "oteryn:item.tibia.i100", "definition-r1"),
        Some(1)
    );
    assert_eq!(
        reference(&index, "oteryn:item.tibia.i1000", "definition-r1"),
        Some(2)
    );
    assert_eq!(
        reference(&index, "oteryn:item.tibia.i200", "definition-r2"),
        Some(3)
    );
}

#[test]
fn lookup_refuses_a_non_item_family_an_unknown_key_and_another_revision() {
    let index =
        ItemDefinitionIndex::decode(&document(&[("oteryn:item.tibia.i100", "definition-r1")]))
            .unwrap();
    for family in ["Terrain", "WorldObject", "Creature", "item", ""] {
        assert_eq!(
            index.definition_ref(family, "oteryn:item.tibia.i100", "definition-r1"),
            None
        );
    }
    assert_eq!(
        reference(&index, "oteryn:item.tibia.i101", "definition-r1"),
        None
    );
    assert_eq!(
        reference(&index, "oteryn:terrain.grass", "definition-r1"),
        None
    );
    assert_eq!(
        reference(&index, "oteryn:item.tibia.i100", "definition-r2"),
        None
    );
    assert_eq!(reference(&index, "oteryn:item.tibia.i100", ""), None);
}

#[test]
fn decode_refuses_an_unordered_duplicate_or_foreign_key_set() {
    let key = |k: &'static str| (k, "definition-r1");
    for records in [
        vec![key("oteryn:item.tibia.i200"), key("oteryn:item.tibia.i100")],
        vec![key("oteryn:item.tibia.i200"), key("oteryn:item.tibia.i200")],
        vec![key("no-namespace")],
        vec![("oteryn:item.tibia.i200", "")],
    ] {
        assert!(ItemDefinitionIndex::decode(&document(&records)).is_err());
    }
    assert!(
        ItemDefinitionIndex::decode(br#"{"schema":"OTERYN_NATIVE_ITEM_KEYS/v2","records":[]}"#)
            .is_err()
    );
    assert!(
        ItemDefinitionIndex::decode(
            br#"{"schema":"OTERYN_NATIVE_ITEM_KEYS/v1","records":[],"extra":1}"#
        )
        .is_err()
    );
    let empty = ItemDefinitionIndex::decode(&document(&[])).unwrap();
    assert!(empty.is_empty());
    assert_eq!(
        reference(&empty, "oteryn:item.tibia.i100", "definition-r1"),
        None
    );
}

#[test]
fn the_production_key_set_decodes_and_a_rebuild_gives_the_same_index() {
    let first = ItemDefinitionIndex::decode(PRODUCTION_KEYS).unwrap();
    let second = ItemDefinitionIndex::decode(PRODUCTION_KEYS).unwrap();
    assert_eq!(first, second);
    assert!(!first.is_empty());
    assert_eq!(
        reference(&first, "oteryn:item.tibia.i100", "definition-r1"),
        Some(1)
    );
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn shards(directory: &str, prefix: &str) -> Vec<Vec<u8>> {
    let mut paths: Vec<_> = std::fs::read_dir(repo_root().join(directory))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".json"))
        })
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|path| std::fs::read(path).unwrap())
        .collect()
}

/// A B3 region file of `tiles` in local sector 0 (format of `world_region_codec.encode_region`).
fn region(tiles: &[Tile]) -> Vec<u8> {
    let frame = zstd::bulk::compress(&sector::encode(tiles).unwrap(), 3).unwrap();
    let mut out = b"OTRB".to_vec();
    out.extend_from_slice(&[1, 7]);
    for value in [0u16, 0, 1] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.push(0);
    out.extend_from_slice(&(12u32 + 9).to_le_bytes());
    out.extend_from_slice(&(frame.len() as u32).to_le_bytes());
    out.extend_from_slice(&frame);
    out
}

/// The pinned production key set is exactly the Item identities of every checked-in Item
/// definition shard, in ascending byte order, so a stale pinned index fails the tests.
#[test]
fn the_pinned_key_set_equals_the_item_definition_shards() {
    let mut derived: Vec<(String, String)> = shards("content/items/definitions", "items-")
        .iter()
        .flat_map(|shard| {
            serde_json::from_slice::<serde_json::Value>(shard).unwrap()["records"]
                .as_array()
                .unwrap()
                .clone()
        })
        .map(|record| record["definition"]["identity"].clone())
        .filter(|identity| identity["family"] == "Item")
        .map(|identity| {
            (
                identity["key"].as_str().unwrap().to_owned(),
                identity["revision"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    derived.sort();
    let pinned: Vec<(String, String)> =
        serde_json::from_slice::<serde_json::Value>(PRODUCTION_KEYS).unwrap()["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| serde_json::from_value(row.clone()).unwrap())
            .collect();
    assert!(!derived.is_empty());
    assert_eq!(pinned, derived);
}

/// The index of the pinned production key set gives every Item palette entry of a bundle,
/// compiled by the World bundle compiler over the checked-in Item definitions, exactly
/// 1 + its palette `id` (§1.6 "Bundle World").
#[test]
fn the_pinned_index_equals_the_bundle_palette_ids_of_a_fixture_bundle() {
    let mut registry = Registry::default();
    for shard in shards("content/items/definitions", "items-") {
        registry.add_items(&shard).unwrap();
    }
    for (family, directory, prefix) in [
        ("Terrain", "content/world/terrain", "terrain-"),
        ("WorldObject", "content/world/objects", "objects-"),
    ] {
        for shard in shards(directory, prefix) {
            registry.add_catalogue(family, &shard).unwrap();
        }
    }
    registry.seal().unwrap();
    let index = ItemDefinitionIndex::decode(PRODUCTION_KEYS).unwrap();
    let records: Vec<(String, String)> =
        serde_json::from_slice::<serde_json::Value>(PRODUCTION_KEYS).unwrap()["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| serde_json::from_value(row.clone()).unwrap())
            .collect();
    // The first, last and spread Item keys, so the ids cover both ends of the key space.
    let step = records.len() / 16;
    let palette: Vec<String> = (0..records.len())
        .step_by(step)
        .chain([records.len() - 1])
        .map(|at| records[at].0.clone())
        .collect();
    let tiles: Vec<Tile> = (0..palette.len())
        .map(|at| Tile {
            x: at as u16,
            y: 0,
            flags: 0,
            house: 0,
            zones: Vec::new(),
            items: vec![Item {
                palette: at as u32,
                depth: 0,
                attrs: Attrs::default(),
            }],
        })
        .collect();
    let families = Families::default();
    let compiled = compile(
        &Input {
            regions: &[region(&tiles)],
            palette: &palette,
            identity: Identity {
                content_revision: "rev-1".into(),
                ..Identity::default()
            },
            world: Extent {
                min_x: 0,
                min_y: 0,
                max_x: 256,
                max_y: 256,
                floors: vec![-7],
            },
            build_class: BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &families,
        },
        &registry,
    )
    .unwrap();
    let bundle = oteryn_world_bundle::bundle::read(&compiled.bytes).unwrap();
    let items: Vec<_> = bundle
        .manifest
        .palette
        .iter()
        .filter(|entry| entry.family == Family::Item)
        .collect();
    assert_eq!(items.len(), palette.len());
    for entry in items {
        assert_eq!(
            reference(&index, &entry.key, "definition-r1"),
            Some(entry.id + 1),
            "{}",
            entry.key
        );
    }
}
