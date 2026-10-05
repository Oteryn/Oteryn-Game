//! World Project families the compiler reads besides the B3 placements (ADR-0021 §4.3, §4.6):
//! the World record, `Transition.Teleport`, the House catalogue, the minimap draft areas, the
//! spawn sources (`Spawn.Source`, format v3) and the creature facts the spawns are checked with.
//!
//! Every position stays in the project frame `global-target-2026-09-27` (`floor` = legacy `z`)
//! until the compiler maps it (§4.3).

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use serde_json::Value;

use crate::Error;
use crate::bundle::Extent;
use crate::spawn::{self, Direction};

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
    /// `Spawn.Source` records in file order, positions in the project frame.
    pub spawns: Vec<SpawnSource>,
    /// Creature definition key to the facts a spawn point is checked with.
    pub creatures: BTreeMap<String, CreatureFacts>,
}

/// One point of a spawn source: a creature definition, a cell, a direction and a respawn delay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpawnPoint {
    pub creature: String,
    pub cell: LegacyPosition,
    pub direction: Direction,
    pub respawn_ms: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpawnSource {
    pub key: String,
    pub centre: LegacyPosition,
    pub points: Vec<SpawnPoint>,
}

/// What a spawn point needs of its creature definition (CREATURE-AI-0 §6.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreatureFacts {
    /// `spawn_eligibility.period`, as authored.
    pub period: String,
    /// A `bosstiary` block or `reward_boss`: BOSS-RAID-0's, never realized by a spawn.
    pub boss: bool,
    /// Bound to an Encounter: never realized by a spawn (E3).
    pub encounter_bound: bool,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cell {
    floor: u8,
    x: u16,
    y: u16,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PointRecord {
    cell: Cell,
    creature: String,
    direction: String,
    respawn_ms: u32,
}

#[derive(Deserialize)]
struct SpawnRecord {
    identity: Identity,
    centre: Cell,
    points: Vec<PointRecord>,
}

#[derive(Deserialize)]
struct Identity {
    key: String,
}

#[derive(Deserialize)]
struct SpawnShard {
    coordinate_frame: String,
    family: String,
    records: Vec<Record<SpawnRecord>>,
}

impl Families {
    /// Adds the sources of one `Spawn.Source` shard. Bounds are checked here, before anything
    /// is kept: `CREATUREAI0-RL-01` to `-03` and `-13`. A key given twice fails.
    pub fn add_spawns(&mut self, shard: &[u8]) -> Result<(), Error> {
        let shard: SpawnShard = parse(shard, "Spawn.Source shard")?;
        if shard.family != "Spawn.Source" || shard.coordinate_frame != FRAME {
            return Err(format("Spawn.Source shard family or coordinate frame"));
        }
        let limit = |what: &str| Err(Error::Limit(what.into()));
        for record in shard.records {
            let record = record.declaration;
            if self.spawns.len() >= spawn::MAX_SOURCES {
                return limit("spawn sources over CREATUREAI0-RL-02");
            }
            if record.points.is_empty() {
                return Err(format(format!(
                    "spawn source {} has no points",
                    record.identity.key
                )));
            }
            if record.points.len() > spawn::MAX_POINTS_PER_SOURCE {
                return limit("spawn points per source over CREATUREAI0-RL-03");
            }
            let held: usize = self.spawns.iter().map(|source| source.points.len()).sum();
            if held + record.points.len() > spawn::MAX_POINTS {
                return limit("spawn points over CREATUREAI0-RL-01");
            }
            let centre = (record.centre.x, record.centre.y, record.centre.floor);
            if centre.2 > 15 {
                return Err(Error::Bounds(format!("spawn centre floor {}", centre.2)));
            }
            let mut points = Vec::with_capacity(record.points.len());
            for point in record.points {
                if point.cell.floor != centre.2 {
                    return Err(format(format!(
                        "spawn source {} has a point on another floor",
                        record.identity.key
                    )));
                }
                if !(spawn::MIN_RESPAWN_MS..=spawn::MAX_RESPAWN_MS).contains(&point.respawn_ms) {
                    return limit("respawn delay outside CREATUREAI0-RL-13");
                }
                let direction = match point.direction.as_str() {
                    "north" => Direction::North,
                    "east" => Direction::East,
                    "south" => Direction::South,
                    "west" => Direction::West,
                    other => return Err(format(format!("spawn direction `{other}`"))),
                };
                points.push(SpawnPoint {
                    creature: point.creature,
                    cell: (point.cell.x, point.cell.y, point.cell.floor),
                    direction,
                    respawn_ms: point.respawn_ms,
                });
            }
            self.spawns.push(SpawnSource {
                key: record.identity.key,
                centre,
                points,
            });
        }
        Ok(())
    }

    /// Adds the creature facts of one Creature definitions shard. A key given twice fails.
    pub fn add_creatures(&mut self, shard: &[u8]) -> Result<(), Error> {
        let shard: Value = parse(shard, "Creature shard")?;
        let records = shard
            .get("records")
            .and_then(Value::as_array)
            .filter(|_| shard.get("family").and_then(Value::as_str) == Some("Creature"))
            .ok_or_else(|| format("not a Creature shard"))?;
        for record in records {
            let text = |pointer: &str| record.pointer(pointer).and_then(Value::as_str);
            let key = text("/definition/identity/key")
                .ok_or_else(|| format("Creature record without a key"))?;
            let period = text("/authoring/profile/details/spawn_eligibility/period")
                .ok_or_else(|| format(format!("{key} has no spawn period")))?;
            let present = |pointer: &str| record.pointer(pointer).is_some_and(|v| !v.is_null());
            let facts = CreatureFacts {
                period: period.to_owned(),
                boss: present("/authoring/profile/details/bosstiary")
                    || record
                        .pointer("/authoring/profile/details/system_eligibility/reward_boss")
                        .and_then(Value::as_bool)
                        .ok_or_else(|| format(format!("{key} has no reward_boss flag")))?,
                encounter_bound: record
                    .pointer("/authoring/profile/encounters")
                    .and_then(Value::as_array)
                    .is_some_and(|list| !list.is_empty()),
            };
            if self.creatures.insert(key.to_owned(), facts).is_some() {
                return Err(format(format!("Creature key {key} given twice")));
            }
        }
        Ok(())
    }
}
