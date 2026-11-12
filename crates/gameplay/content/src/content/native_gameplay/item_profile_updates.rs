//! Compose pinned Item corrections without rewriting the historical provider.
use super::{
    ContentError, ItemProfileRecord, ItemProfilesDocument, PinnedGameplayBytes, hex, invalid,
    qualify, sha256,
};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Updates {
    schema: String,
    base_sha256: String,
    records: Vec<ItemProfileRecord>,
}

pub(super) fn merge(
    base: Option<PinnedGameplayBytes>,
    updates: Option<PinnedGameplayBytes>,
) -> Result<Option<PinnedGameplayBytes>, ContentError> {
    let Some(updates) = updates else {
        return Ok(base);
    };
    let base = base.ok_or_else(|| invalid("Item updates require the native provider"))?;
    qualify(&base, super::MAX_ITEM_PROFILES)?;
    qualify(&updates, 200_000)?;
    let updates: Updates =
        serde_json::from_slice(&updates.bytes).map_err(|_| invalid("Item updates document"))?;
    let mut items: ItemProfilesDocument =
        serde_json::from_slice(&base.bytes).map_err(|_| invalid("Item updates base"))?;
    if updates.schema != "OTERYN_NATIVE_ITEM_UPDATES/v1"
        || items.schema != "OTERYN_NATIVE_ITEM_PROFILES/v1"
        || updates.base_sha256 != base.sha256
        || updates.records.is_empty()
        || updates.records.len() > 128
    {
        return Err(invalid("Item updates discriminator, bounds or base pin"));
    }
    let mut keys = BTreeSet::new();
    for record in updates.records {
        let key = &record.production_definition.production_key;
        if !keys.insert(key.clone()) {
            return Err(invalid("duplicate Item update"));
        }
        if let Some(existing) = items
            .records
            .iter_mut()
            .find(|old| old.production_definition.production_key == *key)
        {
            if existing.production_definition != record.production_definition {
                return Err(invalid("Item update changes definition identity"));
            }
            *existing = record;
        } else {
            items.records.push(record);
        }
    }
    if items.records.len() > super::MAX_ITEM_RECORDS {
        return Err(invalid("Item updates combined bounds"));
    }
    let bytes = serde_json::to_vec(&items).map_err(|_| invalid("Item updates encoding"))?;
    super::bounded(&bytes, super::MAX_ITEM_PROFILES)?;
    Ok(Some(PinnedGameplayBytes {
        sha256: hex(sha256(&bytes)),
        bytes,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pin(bytes: &[u8]) -> PinnedGameplayBytes {
        PinnedGameplayBytes {
            bytes: bytes.to_vec(),
            sha256: hex(sha256(bytes)),
        }
    }
    #[test]
    fn srx1_item_updates_bind_base_and_preserve_untouched_records() -> Result<(), ContentError> {
        let base = pin(include_bytes!(
            "../../../../../../content/items/definitions/spell-native-profiles.json"
        ));
        let updates = pin(include_bytes!(
            "../../../../../../content/items/definitions/srx1-item-updates.json"
        ));
        assert_eq!(
            merge(Some(base.clone()), None)?
                .ok_or_else(|| invalid("base"))?
                .bytes,
            base.bytes
        );
        let expected: Updates =
            serde_json::from_slice(&updates.bytes).map_err(|_| invalid("updates"))?;
        let original: ItemProfilesDocument =
            serde_json::from_slice(&base.bytes).map_err(|_| invalid("base"))?;
        let merged =
            merge(Some(base.clone()), Some(updates.clone()))?.ok_or_else(|| invalid("merged"))?;
        let actual: ItemProfilesDocument =
            serde_json::from_slice(&merged.bytes).map_err(|_| invalid("merged"))?;
        for record in &expected.records {
            assert!(actual.records.iter().any(|row| row == record));
        }
        for record in original.records {
            if !expected.records.iter().any(|row| {
                row.production_definition.production_key
                    == record.production_definition.production_key
            }) {
                assert!(actual.records.iter().any(|row| row == &record));
            }
        }
        assert!(merge(Some(merged), Some(updates.clone())).is_err());
        assert!(merge(None, Some(updates.clone())).is_err());
        let mut stale = updates.clone();
        stale.sha256 = "0".repeat(64);
        assert!(merge(Some(base.clone()), Some(stale)).is_err());
        let mut duplicate: serde_json::Value =
            serde_json::from_slice(&updates.bytes).map_err(|_| invalid("updates"))?;
        let first = duplicate["records"][0].clone();
        duplicate["records"]
            .as_array_mut()
            .ok_or_else(|| invalid("records"))?
            .push(first);
        let encoded = serde_json::to_vec(&duplicate).map_err(|_| invalid("duplicate"))?;
        assert!(merge(Some(base), Some(pin(&encoded))).is_err());
        Ok(())
    }
}
