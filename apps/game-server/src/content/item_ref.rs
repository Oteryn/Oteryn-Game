//! MAP-ITEM-REF-1 (ARCH-MAP-TRACK-PACKETS-V1 §1.6): the Item definition index of one activated
//! content generation. The wire `item_definition_ref` of an Item definition is 1 + its Item
//! compact id: the index of its key among all Item keys of the generation, ascending by the key's
//! bytes, as the World bundle compiler numbers them.
//!
//! The index is decoded from the pinned Item key set section of the native gameplay artifact
//! (`OTERYN_NATIVE_ITEM_KEYS/v1`, generated from `content/items/definitions`), so the activation
//! digest covers it. It is immutable and held with the Channel content pin; nothing durable
//! stores a reference, and a restart rebuilds the same index from the same artifact.

use super::model::ContentError;
use serde::Deserialize;
use std::num::NonZeroU32;

pub(crate) const ITEM_KEYS_SCHEMA: &str = "OTERYN_NATIVE_ITEM_KEYS/v1";
const ITEM_FAMILY: &str = "Item";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemKeysDocument {
    schema: String,
    records: Vec<(String, String)>,
}

/// The Item keys of one generation with their definition revisions, strictly ascending by key.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ItemDefinitionIndex {
    records: Vec<(Box<str>, Box<str>)>,
}

impl ItemDefinitionIndex {
    /// Decodes and qualifies the Item key set section: the exact schema, valid keys and
    /// revisions, keys strictly ascending (so unique), and every reference within `u32`.
    pub(crate) fn decode(section: &[u8]) -> Result<Self, ContentError> {
        let invalid = || ContentError::InvalidArtifact("native gameplay Item key set");
        let document: ItemKeysDocument = serde_json::from_slice(section).map_err(|_| invalid())?;
        if document.schema != ITEM_KEYS_SCHEMA
            || u32::try_from(document.records.len()).is_err()
            || document
                .records
                .windows(2)
                .any(|pair| pair[0].0.as_bytes() >= pair[1].0.as_bytes())
        {
            return Err(invalid());
        }
        let mut records = Vec::with_capacity(document.records.len());
        for (key, revision) in document.records {
            super::ProductionKey::new(&key)?;
            super::DefinitionRevisionRef::new(&revision)?;
            records.push((key.into_boxed_str(), revision.into_boxed_str()));
        }
        Ok(Self { records })
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The wire reference of a typed Item definition: 1 + the key's index, only when `family`
    /// is Item and the key is an Item of this generation at exactly `revision`. Anything else is
    /// `None`, and the caller fails closed.
    pub(crate) fn definition_ref(
        &self,
        family: &str,
        key: &str,
        revision: &str,
    ) -> Option<NonZeroU32> {
        if family != ITEM_FAMILY {
            return None;
        }
        let index = self
            .records
            .binary_search_by(|(candidate, _)| candidate.as_bytes().cmp(key.as_bytes()))
            .ok()?;
        if &*self.records[index].1 != revision {
            return None;
        }
        u32::try_from(index)
            .ok()?
            .checked_add(1)
            .and_then(NonZeroU32::new)
    }
}

#[cfg(test)]
#[path = "item_ref_tests.rs"]
mod tests;
