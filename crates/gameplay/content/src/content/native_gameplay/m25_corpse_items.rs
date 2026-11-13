//! Exact M25 corpse policies: Crystal Ahau and canonical Canary Mitmah.
//! The lever's Crystal origin never selects the native Mitmah Item defaults.
use super::*;
#[path = "m25_corpse_items_pin.rs"]
mod pin;

const CRYSTAL: &str = "201189f871d23597b82caf88fd611454a2ddefb8";
// Historical exact identity binding is preserved independently of current policy facts.
const CRYSTAL_BINDING: &str = "ff7ede593c69d4c658b382c97443e8155926924a";
const CANARY: &str = "47dfd51f45280a59a1d3e50ba7edd573d7234446";
const IDS: [&str; 8] = [
    "42069", "42070", "42071", "42072", "44684", "44685", "44686", "44687",
];
const CAPTURE: &[u8] = include_bytes!(
    "../../../../../../tools/content-schema/native-gameplay/m25-corpse-item-profiles.json"
);
const PACKET: &[u8] = include_bytes!(
    "../../../../../../tools/content-schema/native-gameplay/candidate-bindings/m25-corpse-items-candidate.json"
);

pub(super) fn qualified_records() -> Result<Vec<ItemProfileRecord>, ContentError> {
    qualify(CAPTURE, PACKET)
}

fn qualify(capture: &[u8], packet: &[u8]) -> Result<Vec<ItemProfileRecord>, ContentError> {
    if hex(sha256(capture)) != pin::CAPTURE_SHA256 || hex(sha256(packet)) != pin::PACKET_SHA256 {
        return Err(invalid("M25 corpse complete source hash"));
    }
    let capture: serde_json::Value =
        serde_json::from_slice(capture).map_err(|_| invalid("M25 corpse source capture"))?;
    let packet: serde_json::Value = serde_json::from_slice(packet)
        .map_err(|_| invalid("M25 corpse existing identity packet"))?;
    if capture["schema"] != "OTERYN_M25_CORPSE_ITEM_PROFILES/v1"
        || capture["source_revisions"] != serde_json::json!({"crystal":CRYSTAL,"canary":CANARY})
        || capture["roots"] != serde_json::json!([42069, 44687])
        || capture["decay_chains"]
            != serde_json::json!([[42069, 42072, 42071, 42070], [44687, 44686, 44685, 44684]])
        || packet["schema"] != "OTERYN_SOURCE_ITEM_IDENTITY_CANDIDATE/v1"
        || packet["family"] != "Item"
        || packet["qualification"]["source_revision"] != CANARY
    {
        return Err(invalid("M25 corpse exact source discriminator"));
    }
    let bindings: Vec<super::super::project::ProjectV2SourceIdentityBinding> =
        serde_json::from_value(packet["bindings"].clone())
            .map_err(|_| invalid("M25 corpse exact existing bindings"))?;
    let records: Vec<ItemProfileRecord> = serde_json::from_value(capture["records"].clone())
        .map_err(|_| invalid("M25 corpse complete source records"))?;
    if bindings.len() != 4 || records.len() != IDS.len() {
        return Err(invalid("M25 corpse exact closure bound"));
    }
    for (binding, id) in bindings.iter().zip(&IDS[4..]) {
        if !exact_binding(binding, id, "oteryn:source.canary", CANARY) {
            return Err(invalid("M25 corpse Canary existing reference"));
        }
    }
    for (record, id) in records.iter().zip(IDS) {
        let binding = &record.production_binding;
        let canary = id.starts_with("446");
        if !exact_binding(
            binding,
            id,
            if canary {
                "oteryn:source.canary"
            } else {
                "oteryn:source.crystalserver"
            },
            if canary { CANARY } else { CRYSTAL_BINDING },
        ) || record.production_definition.family != "Item"
            || record.production_definition.production_key != binding.target.key
            || record.production_definition.revision_ref != binding.target.revision
            || !record.admission.materializable
            || record.admission.stack_class != NativeItemStackClass::NonStackable
            || record.admission.legal_destinations != [NativeItemDestination::Ground]
            || (canary
                && (record.production_binding_qualification.as_ref() != Some(&packet)
                    || !bindings.contains(binding)))
            || (!canary && record.production_binding_qualification.is_some())
        {
            return Err(invalid("M25 corpse complete native Ground policy"));
        }
    }
    Ok(records)
}

fn exact_binding(
    binding: &super::super::project::ProjectV2SourceIdentityBinding,
    id: &str,
    source: &str,
    revision: &str,
) -> bool {
    binding.external_id == id
        && binding.source_key == source
        && binding.source_revision == revision
        && binding.identity_namespace == "ots/item_server_id"
        && binding.disposition == super::super::ProjectV2SourceIdentityDisposition::Exact
        && binding.target.family == ProjectV2Family::Item
        && binding.target.key == format!("oteryn:item.tibia.i{id}")
        && binding.target.revision == "definition-r1"
}

pub(super) fn admit_exact_bindings(
    admitted: &mut BTreeMap<String, super::super::project::ProjectV2SourceIdentityBinding>,
    records: &[ItemProfileRecord],
) -> Result<(), ContentError> {
    for record in records {
        let binding = &record.production_binding;
        if let Some(existing) = admitted.get(&binding.external_id) {
            if existing != binding {
                return Err(invalid("M25 corpse conflicting source identity"));
            }
        } else if binding.source_key == "oteryn:source.canary" {
            admitted.insert(binding.external_id.clone(), binding.clone());
        } else {
            return Err(invalid("M25 corpse missing exact Crystal binding"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn m25_corpse_source_packet_is_exact_and_rejects_mutation() -> Result<(), ContentError> {
        let records = qualified_records()?;
        assert_eq!(records.len(), 8);
        for (bytes, capture) in [(CAPTURE, true), (PACKET, false)] {
            let mut value: serde_json::Value =
                serde_json::from_slice(bytes).map_err(|_| invalid("M25 test source JSON"))?;
            if capture {
                value["records"][0]["admission"]["materializable"] = false.into();
            } else {
                value["bindings"][0]["target"]["key"] = "oteryn:item.tibia.i1".into();
            }
            let altered =
                serde_json::to_vec(&value).map_err(|_| invalid("M25 test altered JSON"))?;
            assert!(
                if capture {
                    qualify(&altered, PACKET)
                } else {
                    qualify(CAPTURE, &altered)
                }
                .is_err()
            );
        }
        let mut admitted = BTreeMap::new();
        for record in &records[..4] {
            admitted.insert(
                record.production_binding.external_id.clone(),
                record.production_binding.clone(),
            );
        }
        admit_exact_bindings(&mut admitted, &records)?;
        assert_eq!(admitted.len(), 8);
        let mut conflict = records[4].production_binding.clone();
        conflict.target.revision = "definition-r2".into();
        admitted.insert(conflict.external_id.clone(), conflict);
        assert!(admit_exact_bindings(&mut admitted, &records).is_err());
        Ok(())
    }
}
