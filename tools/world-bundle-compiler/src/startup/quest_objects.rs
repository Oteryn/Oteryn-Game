//! Explicit permanent objects for selected personal quest recipes; source B3 stays intact.
use crate::{Error, b3::Region};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::Path};

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Clone, Debug, Default)]
pub(super) struct Source {
    objects: Vec<Object>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Object {
    quest: String,
    position: (u16, u16, u8),
    item: u16,
    ground: u16,
    description: String,
    recipe_path: String,
    recipe_sha256: String,
    source_world: String,
    source_world_sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    basis: String,
    objects: Vec<Object>,
}
impl Source {
    pub(super) fn load(root: &Path) -> Result<Self, Error> {
        let path = root.join("content/quests/authored-world-objects.json");
        if !path.is_file() {
            return Ok(Self::default());
        }
        let raw = crate::source::read(&path)?;
        if raw.len() > 128 * 1024 {
            return Err(Error::Format("quest object document byte bound".into()));
        }
        let doc: Document = serde_json::from_slice(&raw)
            .map_err(|e| Error::Format(format!("quest objects: {e}")))?;
        let fail = || Error::Format("invalid selected quest object provenance or bounds".into());
        if doc.schema != 1
            || doc.basis != "CHOSEN_OTERYN_APPROXIMATION"
            || doc.objects.is_empty()
            || doc.objects.len() > 128
        {
            return Err(fail());
        }
        let mut positions = BTreeSet::new();
        for object in &doc.objects {
            let (x, y, z) = object.position;
            if z > 15
                || object.item == 0
                || object.ground == 0
                || object.item == object.ground
                || object.description.trim().is_empty()
                || object.description.len() > 256
                || object.description.chars().any(char::is_control)
                || !positions.insert(object.position)
                || !object.quest.starts_with("oteryn:quest.")
                || !object
                    .recipe_path
                    .starts_with("content/quests/definitions/quests-")
                || !object.recipe_path.ends_with(".json")
                || object.recipe_path.contains(['\\', '\0'])
                || object.recipe_path.contains("..")
                || object.source_world
                    != format!(
                        "content/world/placements/region-z{z:02}-x{:03}-y{:03}.b3",
                        x / 256,
                        y / 256
                    )
            {
                return Err(fail());
            }
            let recipe = crate::source::read(&root.join(&object.recipe_path))?;
            let region = crate::source::read(&root.join(&object.source_world))?;
            if digest(&recipe) != object.recipe_sha256
                || digest(&region) != object.source_world_sha256
            {
                return Err(fail());
            }
            let document: serde_json::Value =
                serde_json::from_slice(&recipe).map_err(|_| fail())?;
            let Some(records) = document.get("records").and_then(|r| r.as_array()) else {
                return Err(fail());
            };
            let matched = records
                .iter()
                .filter(|r| {
                    r["definition"]["identity"]["key"].as_str() == Some(object.quest.as_str())
                })
                .collect::<Vec<_>>();
            let [record] = matched.as_slice() else {
                return Err(fail());
            };
            if record["definition"]["recipe"]["repeat"]["kind"] != "once"
                || !record["definition"]["recipe"]["stages"]
                    .as_array()
                    .is_some_and(|stages| !stages.is_empty() && stages.len() <= 256)
            {
                return Err(fail());
            }
        }
        Ok(Self {
            objects: doc.objects,
        })
    }
    pub(super) fn apply(&self, region: &mut Region, palette: &[String]) -> Result<(), Error> {
        for object in self.objects.iter().filter(|o| {
            o.position.2 == region.z
                && o.position.0 / 256 == region.rx
                && o.position.1 / 256 == region.ry
        }) {
            let fail = || {
                Error::Format(format!(
                    "quest object collision/source mismatch: {} at {:?}",
                    object.quest, object.position
                ))
            };
            let tile = region
                .sectors
                .iter_mut()
                .flat_map(|(_, _, tiles)| tiles)
                .find(|t| (t.x, t.y) == (object.position.0, object.position.1))
                .ok_or_else(fail)?;
            if tile.house != 0
                || tile.items.len() != 1
                || tile.items[0].depth != 0
                || palette.get(tile.items[0].palette as usize)
                    != Some(&format!("oteryn:item.tibia.i{}", object.ground))
            {
                return Err(fail());
            }
            let wanted = format!("oteryn:item.tibia.i{}", object.item);
            let entries = palette
                .iter()
                .enumerate()
                .filter(|(_, key)| **key == wanted)
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            let [index] = entries.as_slice() else {
                return Err(fail());
            };
            tile.items.push(crate::sector::Item {
                palette: u32::try_from(*index).map_err(|_| fail())?,
                depth: 0,
                attrs: crate::sector::Attrs {
                    description: Some(object.description.clone()),
                    ..Default::default()
                },
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sector::{Attrs, Item, Tile};
    struct Fixture(std::path::PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "quest-object-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("content/quests/definitions"))?;
        std::fs::create_dir_all(root.join("content/world/placements"))?;
        let recipe=serde_json::json!({"records":[{"definition":{"identity":{"key":"oteryn:quest.authored.test"},"recipe":{"repeat":{"kind":"once"},"stages":[{"key":"s1"}]}}}]}).to_string();
        let world = b"retained source region bytes";
        std::fs::write(
            root.join("content/quests/definitions/quests-test.json"),
            &recipe,
        )?;
        std::fs::write(
            root.join("content/world/placements/region-z07-x000-y000.b3"),
            world,
        )?;
        let doc = serde_json::json!({"schema":1,"basis":"CHOSEN_OTERYN_APPROXIMATION","objects":[{
            "quest":"oteryn:quest.authored.test","position":[1,1,7],"item":2472,"ground":231,"description":"A fixed supply cache.",
            "recipe_path":"content/quests/definitions/quests-test.json","recipe_sha256":digest(recipe.as_bytes()),
            "source_world":"content/world/placements/region-z07-x000-y000.b3","source_world_sha256":digest(world)}]});
        std::fs::write(
            root.join("content/quests/authored-world-objects.json"),
            doc.to_string(),
        )?;
        Ok(Fixture(root))
    }
    fn region() -> Region {
        Region {
            z: 7,
            rx: 0,
            ry: 0,
            sectors: vec![(
                0,
                0,
                vec![Tile {
                    x: 1,
                    y: 1,
                    flags: 0,
                    house: 0,
                    zones: vec![],
                    items: vec![Item {
                        palette: 0,
                        depth: 0,
                        attrs: Attrs::default(),
                    }],
                }],
            )],
        }
    }
    #[test]
    fn personal_quest_objects_preserve_ground_and_refuse_house_occupied_or_wrong_ground()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = fixture()?;
        let source = Source::load(&fixture.0)?;
        let palette = vec![
            "oteryn:item.tibia.i231".into(),
            "oteryn:item.tibia.i2472".into(),
        ];
        let mut good = region();
        let ground = good.sectors[0].2[0].items[0].clone();
        source.apply(&mut good, &palette)?;
        assert_eq!(good.sectors[0].2[0].items[0], ground);
        assert_eq!(good.sectors[0].2[0].items.len(), 2);
        let cache = &good.sectors[0].2[0].items[1];
        assert_eq!(cache.palette, 1);
        assert_eq!(
            cache.attrs.description.as_deref(),
            Some("A fixed supply cache."),
            "description keeps map pickup closed"
        );
        for edit in 0..3 {
            let mut bad = region();
            match edit {
                0 => bad.sectors[0].2[0].house = 42,
                1 => bad.sectors[0].2[0].items.push(ground.clone()),
                _ => bad.sectors[0].2[0].items[0].palette = 1,
            };
            let before = bad.sectors[0].2[0].clone();
            assert!(source.apply(&mut bad, &palette).is_err());
            assert_eq!(bad.sectors[0].2[0], before);
        }
        assert!(
            source.apply(&mut good, &palette).is_err(),
            "an occupied tile cannot receive a duplicate object"
        );
        Ok(())
    }
    #[test]
    fn personal_quest_objects_require_exact_recipe_and_source_witnesses()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = fixture()?;
        let path = fixture.0.join("content/quests/authored-world-objects.json");
        let original: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        for edit in 0..6 {
            let mut bad = original.clone();
            match edit {
                0 => bad["objects"][0]["recipe_sha256"] = "0".repeat(64).into(),
                1 => bad["objects"][0]["source_world_sha256"] = "0".repeat(64).into(),
                2 => bad["objects"][0]["quest"] = "oteryn:quest.authored.absent".into(),
                3 => {
                    bad["objects"][0]["recipe_path"] =
                        "content/quests/definitions/quests-../../outside.json".into()
                }
                4 => bad["objects"][0]["description"] = "".into(),
                _ => bad["objects"]
                    .as_array_mut()
                    .ok_or("objects")?
                    .push(original["objects"][0].clone()),
            }
            std::fs::write(&path, bad.to_string())?;
            assert!(Source::load(&fixture.0).is_err(), "edit{edit}");
        }
        Ok(())
    }
    #[test]
    fn selected_nomads_objects_materialize_on_the_retained_source_cells()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let source = Source::load(&root)?;
        let nomads = source
            .objects
            .iter()
            .filter(|o| o.quest == "oteryn:quest.authored.nomads_mini_world_change")
            .collect::<Vec<_>>();
        assert_eq!(nomads.len(), 3);
        let index: serde_json::Value = serde_json::from_slice(&crate::source::read(
            &root.join("content/world/placements/index.json"),
        )?)?;
        let palette = index["palette"]
            .as_array()
            .ok_or("palette")?
            .iter()
            .map(|r| r["key"].as_str().map(str::to_owned).ok_or("palette key"))
            .collect::<Result<Vec<_>, _>>()?;
        let paths = nomads
            .iter()
            .map(|o| o.source_world.as_str())
            .collect::<BTreeSet<_>>();
        let mut checked = 0;
        for path in paths {
            let raw = crate::source::read(&root.join(path))?;
            let mut budget = crate::bundle::BUNDLE_BUDGET;
            let mut region =
                crate::b3::decode_region(&raw, crate::bundle::TILE_LIMITS, &mut budget)?;
            source.apply(&mut region, &palette)?;
            for object in nomads.iter().filter(|o| o.source_world == path) {
                let tile = region
                    .sectors
                    .iter()
                    .flat_map(|(_, _, tiles)| tiles)
                    .find(|t| (t.x, t.y) == (object.position.0, object.position.1))
                    .ok_or("cache cell")?;
                assert_eq!(tile.house, 0);
                assert_eq!(tile.items.len(), 2);
                assert_eq!(
                    palette[tile.items[0].palette as usize],
                    "oteryn:item.tibia.i231"
                );
                assert_eq!(
                    palette[tile.items[1].palette as usize],
                    "oteryn:item.tibia.i2472"
                );
                assert_eq!(
                    tile.items[1].attrs.description.as_deref(),
                    Some("A fixed nomad supply cache.")
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 3);
        Ok(())
    }
}
