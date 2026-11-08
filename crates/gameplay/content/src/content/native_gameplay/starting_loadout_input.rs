//! SRVGAP-C1: compose the separately pinned starter Item closure into the existing artifact.
//! The old provider bytes remain exact when the optional supplement is absent.
use super::{
    ContentError, ItemProfilesDocument, MAX_ITEM_PROFILES, MAX_RECORDS, PinnedGameplayBytes, hex,
    invalid, qualify, sha256,
};
use std::collections::BTreeSet;

pub(super) fn merge(
    base: Option<PinnedGameplayBytes>,
    supplement: Option<PinnedGameplayBytes>,
) -> Result<Option<PinnedGameplayBytes>, ContentError> {
    let Some(supplement) = supplement else {
        return Ok(base);
    };
    let base = base.ok_or_else(|| invalid("starting loadout requires the native Item provider"))?;
    qualify(&base, MAX_ITEM_PROFILES)?;
    qualify(&supplement, super::MAX_PROFILES)?;
    let mut items: ItemProfilesDocument = serde_json::from_slice(&base.bytes)
        .map_err(|_| invalid("starting loadout base Item profiles"))?;
    let extra: ItemProfilesDocument = serde_json::from_slice(&supplement.bytes)
        .map_err(|_| invalid("starting loadout Item supplement"))?;
    if items.schema != "OTERYN_NATIVE_ITEM_PROFILES/v1"
        || extra.schema != items.schema
        || extra.records.is_empty()
        || extra.records.len() > 20
        || items.records.len() + extra.records.len() > MAX_RECORDS
    {
        return Err(invalid("starting loadout Item discriminator or bounds"));
    }
    let mut keys: BTreeSet<&str> = items
        .records
        .iter()
        .map(|record| record.production_definition.production_key.as_str())
        .collect();
    for record in &extra.records {
        if !keys.insert(&record.production_definition.production_key) {
            return Err(invalid("duplicate starting loadout Item definition"));
        }
    }
    drop(keys);
    items.records.extend(extra.records);
    let bytes =
        serde_json::to_vec(&items).map_err(|_| invalid("starting loadout Item encoding"))?;
    super::bounded(&bytes, MAX_ITEM_PROFILES)?;
    Ok(Some(PinnedGameplayBytes {
        sha256: hex(sha256(&bytes)),
        bytes,
    }))
}

#[cfg(test)]
mod reaudit_starting_progression {
    use super::*;
    fn pin(bytes: &[u8]) -> PinnedGameplayBytes {
        PinnedGameplayBytes {
            bytes: bytes.to_vec(),
            sha256: hex(sha256(bytes)),
        }
    }
    #[test]
    fn reaudit_starting_progression_pins_both_inputs_and_preserves_legacy_bytes()
    -> Result<(), ContentError> {
        let base = pin(br#"{"schema":"OTERYN_NATIVE_ITEM_PROFILES/v1","records":[]}"#);
        let original =
            merge(Some(base.clone()), None)?.ok_or_else(|| invalid("legacy provider"))?;
        assert_eq!(original.bytes, base.bytes);
        let extra = pin(include_bytes!(
            "../../../../../../content/items/definitions/starting-loadout-profiles.json"
        ));
        // Three original starter profiles now belong to the base provider.
        // Keep the complete thirteen-item assertion across the actual partition.
        let provider: ItemProfilesDocument = serde_json::from_slice(include_bytes!(
            "../../../../../../content/items/definitions/spell-native-profiles.json"
        ))
        .map_err(|_| invalid("starter base fixture"))?;
        let moved: Vec<_> = provider
            .records
            .into_iter()
            .filter(|r| {
                matches!(
                    r.production_definition.production_key.as_str(),
                    "oteryn:item.tibia.i3571"
                        | "oteryn:item.tibia.i3572"
                        | "oteryn:item.tibia.i7991"
                )
            })
            .collect();
        assert_eq!(moved.len(), 3);
        let base = pin(&serde_json::to_vec(&ItemProfilesDocument {
            schema: "OTERYN_NATIVE_ITEM_PROFILES/v1".into(),
            records: moved,
        })
        .map_err(|_| invalid("starter base fixture encoding"))?);
        let merged = merge(Some(base.clone()), Some(extra.clone()))?
            .ok_or_else(|| invalid("merged provider"))?;
        let items: ItemProfilesDocument =
            serde_json::from_slice(&merged.bytes).map_err(|_| invalid("merged test provider"))?;
        // The original eight starter profiles plus the five source-qualified Newhaven items.
        assert_eq!(items.records.len(), 13);
        for tibia_id in [34017, 21348, 21400, 21350, 50166] {
            let key = format!("oteryn:item.tibia.i{tibia_id}");
            assert!(
                items
                    .records
                    .iter()
                    .any(|record| { record.production_definition.production_key == key }),
                "missing Newhaven starter profile {key}"
            );
        }
        assert!(merge(Some(merged), Some(extra.clone())).is_err());
        let mut stale = extra;
        stale.sha256 = "0".repeat(64);
        assert!(merge(Some(base), Some(stale)).is_err());
        assert!(merge(None, Some(original)).is_err());
        Ok(())
    }
}
