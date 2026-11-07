//! Five literal Magnolia loot identities; no general non-XML Item admission.
use super::*;
const IDS: [&str; 5] = ["53197", "53199", "53201", "53203", "53205"];
pub(super) fn admit(
    bindings: &mut BTreeMap<String, super::super::project::ProjectV2SourceIdentityBinding>,
) -> Result<serde_json::Value, ContentError> {
    let bytes = include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/candidate-bindings/srv7-summer-amulets.json"
    );
    let expected = include_str!(
        "../../../../../../tools/content-schema/native-gameplay/candidate-bindings/srv7-summer-amulets.sha256"
    );
    if hex(sha256(bytes)) != expected.trim() {
        return Err(invalid("Summer amulet identity source digest"));
    }
    let packet: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| invalid("Summer amulet identity packet"))?;
    if packet["schema"] != "OTERYN_SRV7_SUMMER_AMULET_IDENTITIES/v1"
        || packet["source_revision"] != "201189f871d23597b82caf88fd611454a2ddefb8"
        || packet["bindings"].as_array().map(Vec::len) != Some(5)
        || packet["profiles"].as_array().map(Vec::len) != Some(5)
    {
        return Err(invalid("Summer amulet exact five identities"));
    }
    for (index, id) in IDS.into_iter().enumerate() {
        let expected = serde_json::json!({"source_key":"oteryn:source.crystalserver",
          "source_revision":"201189f871d23597b82caf88fd611454a2ddefb8",
          "identity_namespace":"ots/item_server_id","external_id":id,
          "target":{"family":"Item","key":format!("oteryn:item.tibia.i{id}"),"revision":"definition-r1"},
          "disposition":"EXACT"});
        if packet["bindings"][index] != expected
            || packet["profiles"][index]["production_binding"] != expected
        {
            return Err(invalid("Summer amulet exact source binding"));
        }
        let binding = serde_json::from_value(expected)
            .map_err(|_| invalid("Summer amulet source binding"))?;
        if bindings.insert(id.into(), binding).is_some() {
            return Err(invalid("Summer amulet historical binding conflict"));
        }
    }
    Ok(packet)
}
pub(super) fn owns(external: &str) -> bool {
    IDS.contains(&external)
}
pub(super) fn validate(
    record: &ItemProfileRecord,
    packet: &serde_json::Value,
) -> Result<(), ContentError> {
    if record.production_binding_qualification.as_ref() != Some(packet) {
        return Err(invalid("Summer amulet physical source proof"));
    }
    let index = IDS
        .iter()
        .position(|id| *id == record.production_binding.external_id)
        .ok_or_else(|| invalid("Summer amulet unqualified identity"))?;
    let mut actual = serde_json::to_value(record).map_err(|_| invalid("Summer amulet profile"))?;
    actual
        .as_object_mut()
        .ok_or_else(|| invalid("Summer amulet profile shape"))?
        .remove("production_binding_qualification");
    if actual != packet["profiles"][index] {
        return Err(invalid(
            "Summer amulet exceeds exact physical-only source policy",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srv7_exact_five_amulet_identities_reject_equipment_timer_or_custody_invention()
    -> Result<(), Box<dyn std::error::Error>> {
        let packet = admit(&mut BTreeMap::new())?;
        for profile in packet["profiles"].as_array().ok_or("profiles")? {
            let mut value = profile.clone();
            value["production_binding_qualification"] = packet.clone();
            validate(&serde_json::from_value(value.clone())?, &packet)?;
            for section in [
                "equipment",
                "protection",
                "skill_modifiers",
                "temporal",
                "charges",
            ] {
                assert_eq!(value["semantics"][section]["state"], "UNKNOWN");
                let mut changed = value.clone();
                changed["semantics"][section] = serde_json::json!({"state":"NOT_APPLICABLE"});
                assert!(validate(&serde_json::from_value(changed)?, &packet).is_err());
            }
            let mut changed = value.clone();
            changed["admission"]["legal_destinations"] = serde_json::json!(["Ground"]);
            assert!(validate(&serde_json::from_value(changed)?, &packet).is_err());
            let mut changed = value;
            changed["production_binding"]["external_id"] = serde_json::json!("53198");
            assert!(validate(&serde_json::from_value(changed)?, &packet).is_err());
        }
        assert!(!owns("53198"));
        Ok(())
    }
}
