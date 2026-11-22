//! Crystal startup attributes layered over immutable source regions before compilation.
use crate::{Error, authored_props, b3, bundle, source};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};

const CAPTURE: &str = "tools/world-bundle-compiler/data/crystal-startup-bindings.json";

#[derive(Deserialize)]
struct Capture {
    schema: u8,
    revision: String,
    bindings: Vec<Binding>,
}

#[derive(Deserialize)]
struct Binding {
    position: [u16; 3],
    item: u16,
    attribute: Attribute,
    value: u16,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Attribute {
    Action,
    Unique,
}

pub(super) fn apply(
    root: &Path,
    index: &mut Value,
    regions: &mut [Vec<u8>],
    selected: Option<&[(u16, u16)]>,
) -> Result<(), Error> {
    let path = root.join(CAPTURE);
    if !path.is_file() {
        return Ok(());
    }
    let bad = |message: String| Error::Format(format!("Crystal startup bindings: {message}"));
    let capture: Capture =
        serde_json::from_slice(&source::read(&path)?).map_err(|error| bad(error.to_string()))?;
    if capture.schema != 1
        || capture.revision != "201189f871d23597b82caf88fd611454a2ddefb8"
        || capture.bindings.is_empty()
        || capture.bindings.len() > 256
    {
        return Err(bad("schema or binding limit".into()));
    }
    let palette = index["palette"]
        .as_array_mut()
        .ok_or_else(|| bad("missing palette".into()))?;
    let mut bindings = BTreeMap::new();
    for binding in capture.bindings {
        let [x, y, z] = binding.position;
        let region = (
            u8::try_from(z).map_err(|_| bad("floor".into()))?,
            x / 256,
            y / 256,
        );
        if selected.is_some_and(|chosen| !chosen.contains(&(region.1, region.2))) {
            continue;
        }
        let group: &mut BTreeMap<_, _> = bindings.entry(region).or_default();
        if z > 15
            || binding.item == 0
            || binding.value == 0
            || group
                .insert((x, y, binding.item, binding.attribute), binding.value)
                .is_some()
        {
            return Err(bad(format!(
                "invalid or duplicate selector {:?}",
                binding.position
            )));
        }
    }
    for bytes in regions {
        let header = bytes.get(..12).ok_or_else(|| bad("region header".into()))?;
        let key = (
            header[5],
            u16::from_le_bytes([header[6], header[7]]),
            u16::from_le_bytes([header[8], header[9]]),
        );
        let Some(group) = bindings.remove(&key) else {
            continue;
        };
        let mut budget = bundle::BUNDLE_BUDGET;
        let mut region = b3::decode_region(bytes, bundle::TILE_LIMITS, &mut budget)?;
        for ((x, y, appearance, attribute), value) in group {
            let fail = || {
                bad(format!(
                    "expected one unconflicted appearance {appearance} at ({x}, {y}, {})",
                    key.0
                ))
            };
            let tile = region
                .sectors
                .iter_mut()
                .flat_map(|(_, _, tiles)| tiles)
                .find(|tile| tile.x == x && tile.y == y && tile.house == 0)
                .ok_or_else(fail)?;
            let mut matches = tile.items.iter_mut().filter(|item| {
                item.depth == 0
                    && palette
                        .get(item.palette as usize)
                        .is_some_and(|entry| entry["source_item_id"] == appearance)
            });
            let item = matches.next().ok_or_else(fail)?;
            if matches.next().is_some() {
                return Err(fail());
            }
            let attribute = match attribute {
                Attribute::Action => &mut item.attrs.action,
                Attribute::Unique => &mut item.attrs.unique,
            };
            if *attribute.get_or_insert(value) != value {
                return Err(fail());
            }
            // Crystal doors.lua:251 identifies this appearance even though items.xml
            // lacks it. The closed form is already placed, with its storage action.
            if appearance == 36547 {
                palette[item.palette as usize]["key"] = "oteryn:item.tibia.i36547".into();
                palette[item.palette as usize]["provisional"] = false.into();
            }
        }
        *bytes = authored_props::encode(&region)?;
    }
    if !bindings.is_empty() {
        return Err(bad("missing source region".into()));
    }
    Ok(())
}
