//! World Project families the compiler reads besides the B3 placements (ADR-0021 §4.3, §4.6):
//! the World record, `Transition.Teleport`, the House catalogue and the minimap draft areas.
//!
//! Every position stays in the project frame `global-target-2026-09-27` (`floor` = legacy `z`)
//! until the compiler maps it (§4.3).

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use serde_json::Value;

use crate::Error;
use crate::bundle::Extent;

/// The source frame of every World Project family (ADR-0021 §4.3, D190).
pub const FRAME: &str = "global-target-2026-09-27";

/// A project-frame position: `z` is legacy, 0 (highest) to 15.
pub type LegacyPosition = (u16, u16, u8);

/// The families one compilation checks the placements against.
#[derive(Clone, Debug, Default)]
pub struct Families {
    /// `Transition.Teleport` records, keyed by their `from` position, valued by `to`.
    pub teleports: BTreeMap<LegacyPosition, LegacyPosition>,
    /// Engine house ids of the House catalogue, the ids B3 tiles carry.
    pub houses: BTreeSet<u32>,
}

fn format(what: impl Into<String>) -> Error {
    Error::Format(what.into())
}

fn parse<T: for<'de> Deserialize<'de>>(bytes: &[u8], what: &str) -> Result<T, Error> {
    serde_json::from_slice(bytes).map_err(|e| format(format!("{what}: {e}")))
}

fn native_floor(z: u8) -> Result<i8, Error> {
    i8::try_from(z)
        .ok()
        .filter(|z| *z <= 15)
        .map(|z| -z)
        .ok_or(Error::Bounds(format!("z {z}")))
}

#[derive(Deserialize)]
struct Shard<T> {
    family: String,
    records: Vec<T>,
}

#[derive(Deserialize)]
struct Record<T> {
    declaration: T,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bounds {
    min_x: u16,
    min_y: u16,
    max_x_exclusive: u16,
    max_y_exclusive: u16,
}

#[derive(Deserialize)]
struct World {
    bounds: Bounds,
    coordinate_frame: String,
    floors: Vec<u8>,
}

/// The one World record of a World shard, as the native extent (ADR-0021 §4.3): `x` and `y`
/// unchanged and half-open, each legacy `z` mapped to `-z`, floors ascending.
pub fn world_extent(shard: &[u8]) -> Result<Extent, Error> {
    let shard: Shard<Record<World>> = parse(shard, "World shard")?;
    let [record] = shard.records.as_slice() else {
        return Err(format("World shard must hold exactly one record"));
    };
    let world = &record.declaration;
    if shard.family != "World" || world.coordinate_frame != FRAME {
        return Err(format("World record family or coordinate frame"));
    }
    let mut floors = world
        .floors
        .iter()
        .map(|z| native_floor(*z))
        .collect::<Result<Vec<_>, _>>()?;
    floors.sort_unstable();
    if floors.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(format("World record repeats a floor"));
    }
    let b = &world.bounds;
    Ok(Extent {
        min_x: b.min_x,
        min_y: b.min_y,
        max_x: b.max_x_exclusive,
        max_y: b.max_y_exclusive,
        floors,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Position {
    coordinate_frame: String,
    floor: u8,
    x: u16,
    y: u16,
}

impl Position {
    fn legacy(&self) -> Result<LegacyPosition, Error> {
        if self.coordinate_frame != FRAME || self.floor > 15 {
            return Err(format("Transition position frame or floor"));
        }
        Ok((self.x, self.y, self.floor))
    }
}

#[derive(Deserialize)]
struct Teleport {
    from: Position,
    to: Position,
    transition_kind: String,
}

impl Families {
    /// Adds the records of one `Transition.Teleport` shard. Two records from one position fail.
    pub fn add_teleports(&mut self, shard: &[u8]) -> Result<(), Error> {
        let shard: Shard<Record<Teleport>> = parse(shard, "Transition.Teleport shard")?;
        if shard.family != "Transition.Teleport" {
            return Err(format("not a Transition.Teleport shard"));
        }
        for record in &shard.records {
            let teleport = &record.declaration;
            if teleport.transition_kind != "teleport" {
                return Err(format("Transition record is not a teleport"));
            }
            let from = teleport.from.legacy()?;
            if self.teleports.insert(from, teleport.to.legacy()?).is_some() {
                return Err(format(format!("two Transition records from {from:?}")));
            }
        }
        Ok(())
    }

    /// Adds the houses of one House catalogue shard (`OTERYN_HOUSE_AUTHORING`), by the engine
    /// house id their tiles carry in B3.
    pub fn add_houses(&mut self, shard: &[u8]) -> Result<(), Error> {
        let shard: Value = parse(shard, "House shard")?;
        let houses = shard
            .get("houses")
            .and_then(Value::as_array)
            .ok_or_else(|| format("House shard has no houses list"))?;
        for house in houses {
            let id = house
                .pointer("/provenance/engine_house/house_id")
                .and_then(Value::as_u64)
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| *id != 0)
                .ok_or_else(|| format("House record without an engine house id"))?;
            if !self.houses.insert(id) {
                return Err(format(format!("engine house id {id} given twice")));
            }
        }
        Ok(())
    }
}

/// The names of the minimap draft areas the placements index declares
/// (`source.minimap_draft.areas[].name`), sorted and unique (ADR-0021 §4.6).
pub fn draft_areas(index: &Value) -> Result<Vec<String>, Error> {
    let Some(areas) = index.pointer("/source/minimap_draft/areas") else {
        return Ok(Vec::new());
    };
    let mut names = BTreeSet::new();
    for area in areas
        .as_array()
        .ok_or_else(|| format("minimap draft areas is not a list"))?
    {
        let name = area
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| format("minimap draft area without a name"))?;
        if !names.insert(name.to_owned()) {
            return Err(format(format!("minimap draft area {name} given twice")));
        }
    }
    Ok(names.into_iter().collect())
}
