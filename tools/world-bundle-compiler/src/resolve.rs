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

use crate::Error;
use crate::bundle::Family;
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

    /// The catalogue record an Item key routes to, if any.
    pub fn route(&self, item: &str) -> Option<&Reference> {
        self.routes.get(item).map(|(_, record)| record)
    }
}

impl KeyResolver for Registry {
    fn resolve(&self, key: &str) -> Resolution {
        match self.ids.get(key) {
            Some((family, id)) => Resolution::Resolved(*family, *id),
            None if self.provisional.contains(key) => Resolution::Provisional,
            None => Resolution::Unknown,
        }
    }
}
