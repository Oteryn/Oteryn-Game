//! One explicit DAT/factory-qualified mount operand; no general unbound Item fallback.
use super::*;

pub(super) fn admit(
    bindings: &mut BTreeMap<String, super::super::project::ProjectV2SourceIdentityBinding>,
) -> Result<serde_json::Value, ContentError> {
    let bytes = include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/candidate-bindings/srv7-cosmetic-item.json"
    );
    let expected = include_str!(
        "../../../../../../tools/content-schema/native-gameplay/candidate-bindings/srv7-cosmetic-item.sha256"
    );
    if hex(sha256(bytes)) != expected.trim() {
        return Err(invalid("SRV7 cosmetic Item qualification digest"));
    }
    let packet: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| invalid("SRV7 cosmetic Item qualification"))?;
    let source = serde_json::json!({"source_key":"oteryn:source.crystalserver",
        "source_revision":"201189f871d23597b82caf88fd611454a2ddefb8",
        "identity_namespace":"ots/item_server_id","external_id":"44048",
        "target":{"family":"Item","key":"oteryn:item.tibia.i44048","revision":"definition-r1"},
        "disposition":"EXACT"});
    if packet["schema"] != "OTERYN_SRV7_COSMETIC_ITEM_IDENTITY/v1"
        || packet["source_revision"] != "201189f871d23597b82caf88fd611454a2ddefb8"
        || packet["bindings"] != serde_json::json!([source])
    {
        return Err(invalid("SRV7 cosmetic Item closed identity"));
    }
    let binding = serde_json::from_value(packet["bindings"][0].clone())
        .map_err(|_| invalid("SRV7 cosmetic Item binding"))?;
    if bindings.insert("44048".into(), binding).is_some() {
        return Err(invalid("SRV7 cosmetic Item binding conflict"));
    }
    Ok(packet)
}

pub(super) fn validate(
    record: &ItemProfileRecord,
    packet: &serde_json::Value,
) -> Result<(), ContentError> {
    if record.production_binding_qualification.as_ref() != Some(packet) {
        return Err(invalid("SRV7 cosmetic Item source proof"));
    }
    let mut actual =
        serde_json::to_value(record).map_err(|_| invalid("SRV7 cosmetic Item profile"))?;
    actual
        .as_object_mut()
        .ok_or_else(|| invalid("SRV7 cosmetic Item profile shape"))?
        .remove("production_binding_qualification");
    if actual != packet["profile"] {
        return Err(invalid(
            "SRV7 cosmetic Item exceeds exact source factory policy",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srv7_spiritual_horseshoe_factory_identity_rejects_changed_physics_or_authority()
    -> Result<(), Box<dyn std::error::Error>> {
        let packet = admit(&mut BTreeMap::new())?;
        let mut value = packet["profile"].clone();
        value["production_binding_qualification"] = packet.clone();
        let record: ItemProfileRecord = serde_json::from_value(value.clone())?;
        validate(&record, &packet)?;
        for pointer in [
            "/semantics/physical/value/weight/value",
            "/production_definition/production_key",
        ] {
            let mut changed = value.clone();
            let slot = changed.pointer_mut(pointer).ok_or("fixture pointer")?;
            *slot = if pointer.ends_with("weight/value") {
                serde_json::json!(1)
            } else {
                serde_json::json!("oteryn:item.tibia.i44049")
            };
            let changed: ItemProfileRecord = serde_json::from_value(changed)?;
            assert!(validate(&changed, &packet).is_err());
        }
        let mut changed = value;
        changed["production_binding_qualification"]["source_revision"] =
            serde_json::json!("unproven");
        let changed: ItemProfileRecord = serde_json::from_value(changed)?;
        assert!(validate(&changed, &packet).is_err());
        Ok(())
    }
}
