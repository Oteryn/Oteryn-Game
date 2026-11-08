//! Closed official-appearance identities for M20's source-qualified materials.
//! No OTS server-ID binding is fabricated for an Item absent from that table.
use super::{ContentError, ItemProfileRecord, invalid};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn profiles() -> Result<BTreeMap<String, ItemProfileRecord>, ContentError> {
    let packet: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/qready1-cradle-item-profiles.json"
    ))
    .map_err(|_| invalid("M20 Cradle source packet"))?;
    let official: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../../imports/official/appearance-membership/admitted.json"
    ))
    .map_err(|_| invalid("M20 official identity decision"))?;
    if packet["schema"] != "OTERYN_QREADY1_CRADLE_ITEM_PROFILES/v1"
        || packet["identity_decision"] != official["decision"]
        || official["key_rule"] != "oteryn:item.tibia.i<id>"
        || !official["files"].as_array().is_some_and(|files| {
            files.iter().any(|file| {
                file["appearances_sha256"] == packet["appearance_sha256"]
                    && file["manifest_sha256"] == packet["official_membership_sha256"]
            })
        })
    {
        return Err(invalid("M20 Cradle official provenance"));
    }
    let records: Vec<ItemProfileRecord> = serde_json::from_value(packet["records"].clone())
        .map_err(|_| invalid("M20 Cradle normalized profiles"))?;
    let mut result = BTreeMap::new();
    for record in records {
        let binding = &record.production_binding;
        let key = format!("oteryn:item.tibia.i{}", binding.external_id);
        let fact = packet["evidence"]
            .as_array()
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row["wiki_item_id"]
                        .as_u64()
                        .map(|id| id.to_string())
                        .as_deref()
                        == Some(binding.external_id.as_str())
                })
            })
            .ok_or_else(|| invalid("M20 Cradle explicit Wiki item observation"))?;
        let revision = fact["wiki_fact"]["revision_id"]
            .as_u64()
            .ok_or_else(|| invalid("M20 Cradle Wiki revision"))?;
        if binding.source_key != "oteryn:source.tibiawiki"
            || binding.identity_namespace != "tibiawiki/item_id"
            || binding.source_revision != format!("fandom-r{revision}")
            || binding.target.key != key
            || binding.target.revision != "definition-r1"
            || record.production_definition.production_key != key
            || record.production_definition.revision_ref != "definition-r1"
            || record.production_binding_qualification.is_some()
            || result.insert(binding.external_id.clone(), record).is_some()
        {
            return Err(invalid("M20 Cradle exact official identity"));
        }
    }
    let expected: BTreeSet<_> = ["40016", "40598", "40601", "40603"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    if result.keys().cloned().collect::<BTreeSet<_>>() != expected {
        return Err(invalid("M20 Cradle identity cohort"));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cradle_materials_keep_their_official_identity_and_exact_source_profiles() {
        let profiles = profiles().expect("source-qualified Cradle materials");
        assert_eq!(profiles.len(), 4);
        for (id, record) in profiles {
            assert_eq!(
                record.production_binding.identity_namespace,
                "tibiawiki/item_id"
            );
            assert_eq!(record.production_binding.external_id, id);
            assert!(record.admission.materializable);
            assert!(record.production_binding_qualification.is_none());
        }
    }
}
