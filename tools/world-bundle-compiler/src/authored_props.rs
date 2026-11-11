//! Pinned first-party quest props; derive region bytes without rewriting retained B3 files.
use crate::{Error, b3, bundle, sector};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pin {
    path: String,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Quest {
    key: String,
    path: String,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Position {
    x: u16,
    y: u16,
    floor: i8,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Prop {
    key: String,
    item: String,
    position: Position,
    role: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u8,
    basis: String,
    quest: Quest,
    sources: Vec<String>,
    props: Vec<Prop>,
}
fn bad(message: &str) -> Error {
    Error::Format(format!("authored quest props: {message}"))
}
fn pinned(
    root: &Path,
    path: &str,
    digest: &str,
    prefix: &str,
    limit: usize,
) -> Result<Vec<u8>, Error> {
    let p = Path::new(path);
    if !path.starts_with(prefix)
        || !path.ends_with(".json")
        || p.components().any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(bad("unsafe source path"));
    }
    let full = root
        .join(p)
        .canonicalize()
        .map_err(|_| bad("missing pinned source"))?;
    let base = root.canonicalize().map_err(|_| bad("source root"))?;
    if !full.starts_with(base)
        || std::fs::metadata(&full)
            .map_err(|_| bad("source metadata"))?
            .len()
            > u64::try_from(limit).map_err(|_| bad("source limit"))?
    {
        return Err(bad("source escape or length"));
    }
    let bytes = std::fs::read(full).map_err(|_| bad("missing pinned source"))?;
    let actual: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if bytes.len() > limit || digest != actual {
        return Err(bad("source length or digest"));
    }
    Ok(bytes)
}
fn encode(region: &b3::Region) -> Result<Vec<u8>, Error> {
    let count = u16::try_from(region.sectors.len()).map_err(|_| bad("sector count"))?;
    let mut frames = Vec::new();
    for (sx, sy, tiles) in &region.sectors {
        let raw = sector::encode(tiles)?;
        let frame = zstd::bulk::compress(&raw, 3).map_err(|_| bad("sector encoding"))?;
        frames.push((
            u8::try_from((sy % 8) * 8 + sx % 8).map_err(|_| bad("sector coordinate"))?,
            frame,
        ));
    }
    let mut output = b"OTRB".to_vec();
    output.extend([1, region.z]);
    output.extend(region.rx.to_le_bytes());
    output.extend(region.ry.to_le_bytes());
    output.extend(count.to_le_bytes());
    let mut offset = 12usize + 9 * frames.len();
    for (local, frame) in &frames {
        output.push(*local);
        output.extend(
            u32::try_from(offset)
                .map_err(|_| bad("region size"))?
                .to_le_bytes(),
        );
        output.extend(
            u32::try_from(frame.len())
                .map_err(|_| bad("sector size"))?
                .to_le_bytes(),
        );
        offset = offset
            .checked_add(frame.len())
            .ok_or_else(|| bad("region overflow"))?;
    }
    for (_, frame) in frames {
        output.extend(frame);
    }
    Ok(output)
}
pub(super) fn apply(
    root: &Path,
    index: &mut Value,
    regions: &mut [Vec<u8>],
    selected: Option<&[(u16, u16)]>,
) -> Result<(), Error> {
    let Some(value) = index.get("authored_props") else {
        return Ok(());
    };
    let pins: Vec<Pin> = serde_json::from_value(value.clone()).map_err(|_| bad("invalid index"))?;
    if pins.is_empty() || pins.len() > 64 {
        return Err(bad("index bound"));
    }
    let mut paths = BTreeSet::new();
    let mut keys = BTreeSet::new();
    let mut cells = BTreeSet::new();
    let mut props = Vec::new();
    for pin in pins {
        if !paths.insert(pin.path.clone()) {
            return Err(bad("duplicate source"));
        }
        let bytes = pinned(
            root,
            &pin.path,
            &pin.sha256,
            "content/world/placements/",
            262144,
        )?;
        let doc: Document = serde_json::from_slice(&bytes).map_err(|_| bad("invalid document"))?;
        if doc.schema != 1
            || doc.basis != "CHOSEN_OTERYN_APPROXIMATION"
            || doc.props.is_empty()
            || doc.props.len() > 128
            || doc.sources.is_empty()
            || doc.sources.len() > 16
            || doc.sources.iter().any(|s| s.is_empty() || s.len() > 2048)
        {
            return Err(bad("document bounds or basis"));
        }
        let quest = pinned(
            root,
            &doc.quest.path,
            &doc.quest.sha256,
            "content/quests/definitions/",
            8 * 1024 * 1024,
        )?;
        let quest: Value = serde_json::from_slice(&quest).map_err(|_| bad("quest source"))?;
        let recipe = quest["records"]
            .as_array()
            .and_then(|rows| {
                rows.iter()
                    .find(|r| r["definition"]["identity"]["key"] == doc.quest.key)
            })
            .map(|r| &r["definition"]["oteryn_recipe"]["payload"]["recipe"])
            .ok_or_else(|| bad("quest identity"))?;
        let roles: BTreeSet<&str> = recipe["stages"]
            .as_array()
            .ok_or_else(|| bad("selected recipe"))?
            .iter()
            .filter_map(|s| s["targets"].as_array())
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        for prop in doc.props {
            if !prop.key.starts_with("oteryn:world-prop/")
                || prop.key.len() > 180
                || !prop
                    .key
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-._:/".contains(&b))
                || !prop.item.starts_with("oteryn:item.")
                || prop.item.len() > 180
                || !(-15..=0).contains(&prop.position.floor)
                || !roles.contains(prop.role.as_str())
                || !keys.insert(prop.key.clone())
                || !cells.insert((prop.position.x, prop.position.y, prop.position.floor))
            {
                return Err(bad("unbound identity, role, cell or duplicate"));
            }
            props.push(prop);
        }
    }
    if props.len() > 512 {
        return Err(bad("total prop bound"));
    }
    props.sort_by(|a, b| a.key.cmp(&b.key));
    let palette = index["palette"]
        .as_array_mut()
        .ok_or_else(|| bad("palette"))?;
    let mut groups: BTreeMap<(u8, u16, u16), Vec<(Prop, u32)>> = BTreeMap::new();
    for prop in props {
        let region = (
            prop.position.floor.unsigned_abs(),
            prop.position.x / 256,
            prop.position.y / 256,
        );
        if selected.is_some_and(|chosen| !chosen.contains(&(region.1, region.2))) {
            continue;
        }
        let at = match palette.iter().position(|p| p["key"] == prop.item) {
            Some(i) => i,
            None => {
                palette.push(serde_json::json!({"key":prop.item.clone(),"provisional":false}));
                palette.len() - 1
            }
        };
        groups
            .entry(region)
            .or_default()
            .push((prop, u32::try_from(at).map_err(|_| bad("palette bound"))?));
    }
    for bytes in regions {
        if bytes.len() < 12 {
            return Err(bad("region header"));
        }
        let key = (
            bytes[5],
            u16::from_le_bytes([bytes[6], bytes[7]]),
            u16::from_le_bytes([bytes[8], bytes[9]]),
        );
        let Some(props) = groups.remove(&key) else {
            continue;
        };
        let mut budget = bundle::BUNDLE_BUDGET;
        let mut region = b3::decode_region(bytes, bundle::TILE_LIMITS, &mut budget)?;
        for (prop, palette) in props {
            let tile = region
                .sectors
                .iter_mut()
                .flat_map(|(_, _, tiles)| tiles)
                .find(|t| t.x == prop.position.x && t.y == prop.position.y)
                .ok_or_else(|| bad("missing source tile"))?;
            if tile.house != 0
                || tile.items.is_empty()
                || tile.items.len() >= bundle::TILE_LIMITS.max_entries
                || tile
                    .items
                    .iter()
                    .any(|i| i.depth == 0 && i.palette == palette)
            {
                return Err(bad("house, empty, full or duplicate source tile"));
            }
            tile.items.push(sector::Item {
                palette,
                depth: 0,
                attrs: sector::Attrs::default(),
            });
        }
        *bytes = encode(&region)?;
    }
    if !groups.is_empty() {
        return Err(bad("missing source region"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Temporary(std::path::PathBuf);
    impl Drop for Temporary {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    struct Fixture {
        root: Temporary,
        index: Value,
        regions: Vec<Vec<u8>>,
    }
    fn hash(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
    fn fixture() -> Result<Fixture, String> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = Temporary(std::env::temp_dir().join(format!(
            "oteryn-props-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        )));
        std::fs::create_dir_all(root.0.join("content/quests/definitions"))
            .map_err(|e| e.to_string())?;
        std::fs::create_dir_all(root.0.join("content/world/placements"))
            .map_err(|e| e.to_string())?;
        let quest=serde_json::to_vec(&serde_json::json!({"records":[{"definition":{"identity":{"key":"oteryn:quest.fixture"},"oteryn_recipe":{"payload":{"recipe":{"stages":[{"targets":["Ulcer Probe"]}]}}}}}]})).map_err(|e|e.to_string())?;
        std::fs::write(root.0.join("content/quests/definitions/quest.json"), &quest)
            .map_err(|e| e.to_string())?;
        let props=serde_json::to_vec(&serde_json::json!({"schema":1,"basis":"CHOSEN_OTERYN_APPROXIMATION","quest":{"key":"oteryn:quest.fixture","path":"content/quests/definitions/quest.json","sha256":hash(&quest)},"sources":["owner:selected-recipe"],"props":[{"key":"oteryn:world-prop/fixture-ulcer","item":"oteryn:item.tibia.i32173","position":{"x":31915,"y":32354,"floor":-8},"role":"Ulcer Probe"}]})).map_err(|e|e.to_string())?;
        std::fs::write(
            root.0
                .join("content/world/placements/authored-fixture.json"),
            &props,
        )
        .map_err(|e| e.to_string())?;
        let tile = sector::Tile {
            x: 31915,
            y: 32354,
            flags: 8,
            house: 0,
            zones: vec![3],
            items: vec![sector::Item {
                palette: 0,
                depth: 0,
                attrs: sector::Attrs::default(),
            }],
        };
        let region = b3::Region {
            z: 8,
            rx: 124,
            ry: 126,
            sectors: vec![(31915 / 32, 32354 / 32, vec![tile])],
        };
        let regions = vec![encode(&region).map_err(|e| format!("{e:?}"))?];
        let index = serde_json::json!({"palette":[{"key":"oteryn:item.tibia.i100"}],"authored_props":[{"path":"content/world/placements/authored-fixture.json","sha256":hash(&props)}]});
        Ok(Fixture {
            root,
            index,
            regions,
        })
    }
    #[test]
    fn derives_exact_props_in_memory_and_keeps_original_tile_fields() -> Result<(), String> {
        let mut f = fixture()?;
        let original = f.regions[0].clone();
        apply(&f.root.0, &mut f.index, &mut f.regions, None).map_err(|e| format!("{e:?}"))?;
        assert_ne!(original, f.regions[0]);
        let mut budget = bundle::BUNDLE_BUDGET;
        let r = b3::decode_region(&f.regions[0], bundle::TILE_LIMITS, &mut budget)
            .map_err(|e| format!("{e:?}"))?;
        let t = &r.sectors[0].2[0];
        assert_eq!(
            (t.flags, t.house, t.zones.as_slice()),
            (8, 0, [3].as_slice())
        );
        assert_eq!(t.items.len(), 2);
        assert_eq!(
            (
                t.items[1].palette,
                t.items[1].depth,
                t.items[1].attrs.clone()
            ),
            (1, 0, sector::Attrs::default())
        );
        assert_eq!(f.index["palette"][1]["key"], "oteryn:item.tibia.i32173");
        Ok(())
    }
    #[test]
    fn partial_region_selection_does_not_inject_foreign_props() -> Result<(), String> {
        let mut f = fixture()?;
        let before = f.regions.clone();
        apply(&f.root.0, &mut f.index, &mut f.regions, Some(&[(125, 126)]))
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(f.regions, before);
        assert_eq!(f.index["palette"].as_array().ok_or("palette")?.len(), 1);
        Ok(())
    }
    #[test]
    fn refuses_unpinned_unbound_duplicate_missing_or_house_props() -> Result<(), String> {
        for case in 0..13 {
            let mut f = fixture()?;
            if case == 0 {
                f.index["authored_props"][0]["sha256"] = "0".repeat(64).into();
            } else if case == 1 {
                f.index["authored_props"][0]["path"] =
                    "content/world/placements/../../../outside.json".into();
            } else if case == 2 {
                f.regions.clear();
            } else if case == 3 {
                let mut budget = bundle::BUNDLE_BUDGET;
                let mut region = b3::decode_region(&f.regions[0], bundle::TILE_LIMITS, &mut budget)
                    .map_err(|e| format!("{e:?}"))?;
                region.sectors[0].2[0].house = 1;
                f.regions[0] = encode(&region).map_err(|e| format!("{e:?}"))?;
            } else {
                let path = f
                    .root
                    .0
                    .join("content/world/placements/authored-fixture.json");
                let mut doc: Value =
                    serde_json::from_slice(&std::fs::read(&path).map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                match case {
                    4 => doc["basis"] = "SOURCE_QUALIFIED".into(),
                    5 => doc["quest"]["sha256"] = "0".repeat(64).into(),
                    6 => doc["quest"]["key"] = "oteryn:quest.other".into(),
                    7 => doc["props"][0]["role"] = "Unrelated".into(),
                    8 => doc["props"][0]["position"]["floor"] = (-16).into(),
                    9 => doc["props"][0]["position"]["x"] = 31916.into(),
                    10 => {
                        let row = doc["props"][0].clone();
                        doc["props"].as_array_mut().ok_or("props")?.push(row);
                    }
                    11 => doc["props"][0]["item"] = "canary:item/32173".into(),
                    _ => doc["props"][0]["arbitrary_attributes"] = serde_json::json!({"unique":1}),
                }
                let bytes = serde_json::to_vec(&doc).map_err(|e| e.to_string())?;
                std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
                f.index["authored_props"][0]["sha256"] = hash(&bytes).into();
            }
            assert!(
                apply(&f.root.0, &mut f.index, &mut f.regions, None).is_err(),
                "case {case}"
            );
        }
        Ok(())
    }
}
