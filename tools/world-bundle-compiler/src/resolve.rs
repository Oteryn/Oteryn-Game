//! Palette key resolution against the pinned content revision (ADR-0021 §4.5, A12 §4.6; rulings
//! #162 5910173902 Q1b/Q2a and 5915258560).
//!
//! - An Item key resolves to `item`. Its route to a Terrain or WorldObject record comes from that
//!   record's `item_pointer`, which must be one-to-one; an Item `routed_to`, when present, must
//!   name the same record, and a `routed_to` that no pointer confirms fails closed.
//! - A Terrain key resolves to `terrain` only when its record has no `item_pointer` (an id without
//!   an Item record, Q2a). A catalogue key whose record points at an Item is not a palette key:
//!   the Item key is (Q1b). A WorldObject is reached only through its Item.
//! - A provisional key the placements index flags resolves as provisional, never before a
//!   registry key of the same name.
//!
//! Compact ids are revision-scoped: the index of the key among all keys of its family in this
//! revision, in ascending byte order.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use serde_json::Value;

use crate::Error;
use crate::bundle::{Family, MAX_GROUND_SPEED, Terrain, TerrainKind};
use crate::compile::{KeyResolver, Resolution};

/// A typed definition reference `{family, key, revision}`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Reference {
    pub family: String,
    pub key: String,
    pub revision: String,
}

#[derive(Deserialize)]
struct Shard<T> {
    family: String,
    records: Vec<T>,
}

#[derive(Deserialize)]
struct ItemRecord {
    definition: ItemDefinition,
}

#[derive(Deserialize)]
struct ItemDefinition {
    identity: Reference,
    #[serde(default)]
    routed_to: Option<Reference>,
}

#[derive(Deserialize)]
struct CatalogueRecord {
    identity: Reference,
    provenance: Provenance,
    // Terrain records only; a WorldObject record's fields are never read.
    #[serde(default)]
    kind: Option<Field>,
    #[serde(default)]
    walkable: Option<Field>,
    #[serde(default)]
    ground_speed: Option<Field>,
}

/// A catalogue field `{state, value}`; only `KNOWN` carries a value.
#[derive(Deserialize)]
struct Field {
    state: String,
    #[serde(default)]
    value: Option<Value>,
}

impl Field {
    fn known(field: Option<Field>) -> Option<Value> {
        field.filter(|f| f.state == "KNOWN").and_then(|f| f.value)
    }
}

/// What the compiler knows of a Terrain record's classification, read leniently when the shard
/// is added and judged only when a placed palette entry routes to the record.
#[derive(Debug, Default)]
struct Classified {
    kind: Option<Value>,
    walkable: Option<Value>,
    ground_speed: Option<Value>,
}

#[derive(Deserialize)]
struct Provenance {
    #[serde(default)]
    item_pointer: Option<Reference>,
}

/// The Item registry and the Terrain and WorldObject catalogues of one content revision.
#[derive(Debug, Default)]
pub struct Registry {
    /// Item key to its full identity and its `routed_to`, if any.
    items: BTreeMap<String, (Reference, Option<Reference>)>,
    /// Terrain key to whether its record points at an Item.
    terrain: BTreeMap<String, bool>,
    /// Terrain key to the fields format v2 carries.
    classified: BTreeMap<String, Classified>,
    /// Item key to the `item_pointer` naming it and the catalogue record that holds it.
    routes: BTreeMap<String, (Reference, Reference)>,
    /// Every Terrain and WorldObject identity, `(family, key)`.
    catalogue: BTreeSet<(String, String)>,
    provisional: BTreeSet<String>,
    /// Compact ids, set by [`Registry::seal`]; nothing resolves before it.
    ids: BTreeMap<String, (Family, u32)>,
}

fn format(what: impl Into<String>) -> Error {
    Error::Format(what.into())
}

fn shard<T: for<'de> Deserialize<'de>>(bytes: &[u8], family: &str) -> Result<Vec<T>, Error> {
    let shard: Shard<T> =
        serde_json::from_slice(bytes).map_err(|e| format(format!("{family} shard: {e}")))?;
    if shard.family != family {
        return Err(format(format!("not a {family} shard")));
    }
    Ok(shard.records)
}

impl Registry {
    /// Adds the records of one Item definitions shard. A key given twice fails.
    pub fn add_items(&mut self, bytes: &[u8]) -> Result<(), Error> {
        for record in shard::<ItemRecord>(bytes, "Item")? {
            let ItemDefinition {
                identity,
                routed_to,
            } = record.definition;
            let key = identity.key.clone();
            if identity.family != "Item" || self.items.insert(key, (identity, routed_to)).is_some()
            {
                return Err(format("Item record family, or an Item key given twice"));
            }
        }
        Ok(())
    }

    /// Adds the records of one Terrain or WorldObject catalogue shard (`family`).
    pub fn add_catalogue(&mut self, family: &str, bytes: &[u8]) -> Result<(), Error> {
        if family != "Terrain" && family != "WorldObject" {
            return Err(format(format!("{family} is not a catalogue family")));
        }
        for record in shard::<CatalogueRecord>(bytes, family)? {
            let identity = record.identity;
            if identity.family != family {
                return Err(format(format!("{} is not a {family} record", identity.key)));
            }
            if !self
                .catalogue
                .insert((family.to_owned(), identity.key.clone()))
            {
                return Err(format(format!("{family} key {} given twice", identity.key)));
            }
            let pointer = record.provenance.item_pointer;
            if family == "Terrain" {
                self.terrain.insert(identity.key.clone(), pointer.is_some());
                self.classified.insert(
                    identity.key.clone(),
                    Classified {
                        kind: Field::known(record.kind),
                        walkable: Field::known(record.walkable),
                        ground_speed: Field::known(record.ground_speed),
                    },
                );
            }
            let Some(pointer) = pointer else { continue };
            if pointer.family != "Item" {
                return Err(Error::Key(format!("{} points outside Item", identity.key)));
            }
            let item = pointer.key.clone();
            if let Some((_, other)) = self.routes.insert(item.clone(), (pointer, identity)) {
                return Err(Error::Key(format!(
                    "{item} is pointed at twice, by {} and another record",
                    other.key
                )));
            }
        }
        Ok(())
    }

    /// Flags the provisional keys of the placements index (ADR-0021 §4.5).
    pub fn add_provisional(&mut self, key: String) {
        self.provisional.insert(key);
    }

    /// Checks the routes once every shard is added and assigns the compact ids: each
    /// `item_pointer` names an Item record, and each `routed_to` names the one record that
    /// points back at its Item.
    pub fn seal(&mut self) -> Result<(), Error> {
        // A pointer names one Item definition by its whole typed reference, revision included.
        for (item, (pointer, _)) in &self.routes {
            if self.items.get(item).map(|(identity, _)| identity) != Some(pointer) {
                return Err(Error::Key(format!(
                    "item_pointer {item} ({}) names no Item record of that revision",
                    pointer.revision
                )));
            }
        }
        for (item, (_, routed_to)) in &self.items {
            let Some(routed_to) = routed_to else { continue };
            if self.routes.get(item).map(|(_, record)| record) != Some(routed_to) {
                return Err(Error::Key(format!(
                    "{item} routed_to {} disagrees with the catalogue item_pointer",
                    routed_to.key
                )));
            }
        }
        let too_many = || Error::Limit("more than 2^32 keys in one family".into());
        for (family, keys) in [
            (Family::Item, self.items.keys().collect::<Vec<_>>()),
            (Family::Terrain, self.terrain.keys().collect()),
        ] {
            for (id, key) in keys.into_iter().enumerate() {
                let id = u32::try_from(id).map_err(|_| too_many())?;
                // A Terrain key whose record points at an Item keeps its id but never resolves.
                if (family == Family::Item || self.terrain.get(key) == Some(&false))
                    && self.ids.insert(key.clone(), (family, id)).is_some()
                {
                    return Err(Error::Key(format!(
                        "{key} is both an Item and a Terrain key"
                    )));
                }
            }
        }
        Ok(())
    }

    /// The terrain semantics of a palette key (format v2, ADR-0021 §4.2): the Terrain record the
    /// key routes to, if any. Stops on a Terrain record with an UNKNOWN kind, a `ground` record
    /// with an UNKNOWN `walkable` or `ground_speed`, a speed outside `0..=1000` and a walkable
    /// ground with speed 0. A WorldObject route and a plain Item are never read.
    pub fn terrain_of(&self, key: &str) -> Result<Option<Terrain>, Error> {
        let record = match self.ids.get(key) {
            Some((Family::Terrain, _)) => key,
            Some((Family::Item, _)) => match self.route(key) {
                Some(record) if record.family == "Terrain" => &record.key,
                _ => return Ok(None),
            },
            None => return Ok(None),
        };
        let fields = self.classified.get(record).ok_or_else(|| {
            Error::Key(format!("Terrain record {record} is not in the catalogue"))
        })?;
        let unclassified = |what: &str| Error::Key(format!("Terrain record {record}: {what}"));
        let kind = match fields.kind.as_ref().and_then(Value::as_str) {
            Some("ground") => TerrainKind::Ground,
            Some("border") => TerrainKind::Border,
            Some("wall") => TerrainKind::Wall,
            Some("roof") => TerrainKind::Roof,
            Some("field") => TerrainKind::Field,
            _ => return Err(unclassified("kind is UNKNOWN or not a Terrain kind")),
        };
        let terrain = if kind == TerrainKind::Ground {
            let walkable = fields
                .walkable
                .as_ref()
                .and_then(Value::as_bool)
                .ok_or_else(|| unclassified("ground walkable is UNKNOWN"))?;
            let speed = fields
                .ground_speed
                .as_ref()
                .and_then(Value::as_u64)
                .and_then(|speed| u16::try_from(speed).ok())
                .filter(|speed| *speed <= MAX_GROUND_SPEED)
                .ok_or_else(|| unclassified("ground_speed is UNKNOWN or outside 0..=1000"))?;
            if walkable && speed == 0 {
                return Err(unclassified("a walkable ground has speed 0"));
            }
            Terrain {
                kind,
                walkable: Some(walkable),
                ground_speed: Some(speed),
            }
        } else {
            Terrain {
                kind,
                walkable: None,
                ground_speed: None,
            }
        };
        Ok(Some(terrain))
    }

    /// Counts the placed palette entries by terrain class, for the parity report (§1.4). Never
    /// stops: what [`Registry::terrain_of`] would refuse is counted, not raised.
    pub fn terrain_counts<'a>(&self, placed: impl IntoIterator<Item = &'a str>) -> TerrainCounts {
        let mut counts = TerrainCounts::default();
        for key in placed {
            match self.terrain_of(key) {
                Ok(Some(terrain)) => {
                    *counts
                        .by_kind
                        .entry(format!("{:?}", terrain.kind).to_lowercase())
                        .or_default() += 1;
                }
                Ok(None) => match self.ids.get(key) {
                    Some((Family::Item, _)) if self.route(key).is_some() => {
                        counts.world_object += 1
                    }
                    Some((Family::Item, _)) => counts.plain_item += 1,
                    _ => {}
                },
                Err(_) => {
                    counts.refused += 1;
                    let record = match self.ids.get(key) {
                        Some((Family::Terrain, _)) => Some(key),
                        _ => self.route(key).map(|record| record.key.as_str()),
                    };
                    let kind = record.and_then(|record| self.classified.get(record));
                    if kind.is_some_and(|f| f.kind.as_ref().and_then(Value::as_str).is_none()) {
                        counts.unknown_kind += 1;
                    }
                }
            }
        }
        counts
    }

    /// The catalogue record an Item key routes to, if any.
    pub fn route(&self, item: &str) -> Option<&Reference> {
        self.routes.get(item).map(|(_, record)| record)
    }
}

/// Placed palette entries by terrain class (the MAP-BUNDLE-2 parity report).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct TerrainCounts {
    /// Terrain-routed entries by `kind`.
    pub by_kind: BTreeMap<String, usize>,
    /// Terrain-routed entries whose kind is UNKNOWN; each stops the compile.
    pub unknown_kind: usize,
    /// Terrain-routed entries the compiler refuses for any reason, `unknown_kind` included.
    pub refused: usize,
    /// WorldObject-routed entries, written as `null`.
    pub world_object: usize,
    /// Plain Item entries, written as `null`.
    pub plain_item: usize,
}

impl KeyResolver for Registry {
    fn resolve(&self, key: &str) -> Resolution {
        match self.ids.get(key) {
            Some((family, id)) => Resolution::Resolved(*family, *id),
            None if self.provisional.contains(key) => Resolution::Provisional,
            None => Resolution::Unknown,
        }
    }

    fn terrain(&self, key: &str) -> Result<Option<Terrain>, Error> {
        self.terrain_of(key)
    }
}
