//! Five explicit source movement IDs with new reference-only declarations.
//! All thirteen movement/event profiles are body-bound, including the protected identities.
use super::*;
#[path = "boss_movement_bindings_pin.rs"]
mod pin;

pub(super) fn qualified_records() -> Result<Vec<ItemProfileRecord>, ContentError> {
    let capture = include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/boss-movement-item-profiles.json"
    );
    let packet = include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/candidate-bindings/boss-movement-items-candidate.json"
    );
    if hex(sha256(capture)) != pin::CAPTURE_SHA256 || hex(sha256(packet)) != pin::PACKET_SHA256 {
        return Err(invalid("native movement Item source hash"));
    }
    let capture: serde_json::Value =
        serde_json::from_slice(capture).map_err(|_| invalid("native movement Item capture"))?;
    let packet: serde_json::Value = serde_json::from_slice(packet)
        .map_err(|_| invalid("native movement Item binding packet"))?;
    if capture["schema"] != "OTERYN_NATIVE_BOSS_MOVEMENT_ITEMS/v1"
        || packet["schema"] != "OTERYN_NATIVE_BOSS_MOVEMENT_BINDINGS/v1"
        || packet["source_revision"] != "201189f871d23597b82caf88fd611454a2ddefb8"
    {
        return Err(invalid("native movement Item source discriminator"));
    }
    let bindings: Vec<super::super::project::ProjectV2SourceIdentityBinding> =
        serde_json::from_value(packet["bindings"].clone())
            .map_err(|_| invalid("native movement Item exact bindings"))?;
    if bindings.len() != 5 {
        return Err(invalid("native movement Item binding bound"));
    }
    for (binding, id) in bindings
        .iter()
        .zip(["43589", "43590", "42851", "42853", "43625"])
    {
        if binding.external_id != id
            || binding.source_key != "oteryn:source.crystalserver"
            || binding.source_revision != "201189f871d23597b82caf88fd611454a2ddefb8"
            || binding.identity_namespace != "ots/item_server_id"
            || binding.disposition != super::super::ProjectV2SourceIdentityDisposition::Exact
            || binding.target.family != ProjectV2Family::Item
            || binding.target.key != format!("oteryn:item.tibia.i{id}")
            || binding.target.revision != "definition-r1"
        {
            return Err(invalid("native movement Item source identity"));
        }
    }
    let records: Vec<ItemProfileRecord> = serde_json::from_value(capture["records"].clone())
        .map_err(|_| invalid("native movement Item source profiles"))?;
    if records.len() != 13 {
        return Err(invalid("native movement Item complete closure"));
    }
    for (record, id) in records.iter().zip([
        "20121", "23726", "23727", "23728", "33004", "37000", "42851", "42853", "43294", "43295",
        "43589", "43590", "43625",
    ]) {
        use super::super::ReferenceItemField as F;
        let binding = &record.production_binding;
        let new = bindings.iter().find(|binding| binding.external_id == id);
        if binding.external_id != id
            || binding.disposition != super::super::ProjectV2SourceIdentityDisposition::Exact
            || binding.target.family != ProjectV2Family::Item
            || binding.target.key != format!("oteryn:item.tibia.i{id}")
            || binding.target.revision != "definition-r1"
            || new.is_some_and(|expected| expected != binding)
            || record.production_binding_qualification.as_ref() != new.map(|_| &packet)
            || record.production_definition.family != "Item"
            || record.production_definition.production_key != binding.target.key
            || record.production_definition.revision_ref != binding.target.revision
            || !record.admission.materializable
            || record.admission.stack_class != NativeItemStackClass::NonStackable
            || record.admission.legal_destinations != [NativeItemDestination::Ground]
            || !matches!(&record.semantics.physical,F::Known(value)
                if value.weight==F::Known(0) && value.movable==F::Known(false) && value.pickupable==F::Known(false))
        {
            return Err(invalid("native movement Item closed Ground policy"));
        }
    }
    Ok(records)
}

/// Existing protected identities must compare exactly; only the separately
/// declared five genuinely new references may be inserted.
pub(super) fn admit_exact_bindings(
    admitted: &mut BTreeMap<String, crate::content::project::ProjectV2SourceIdentityBinding>,
    records: &[ItemProfileRecord],
) -> Result<(), ContentError> {
    for record in records {
        let binding = &record.production_binding;
        if matches!(
            binding.external_id.as_str(),
            "20121" | "23726" | "23727" | "23728" | "33004" | "37000" | "43294" | "43295"
        ) {
            if admitted.get(&binding.external_id) != Some(binding) {
                return Err(invalid("native movement protected Item binding changed"));
            }
        } else if matches!(
            binding.external_id.as_str(),
            "43589" | "43590" | "42851" | "42853" | "43625"
        ) {
            if admitted
                .insert(binding.external_id.clone(), binding.clone())
                .is_some()
            {
                return Err(invalid("native movement new Item binding conflict"));
            }
        } else {
            return Err(invalid("native movement unregistered Item binding"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn imports() -> Result<
        BTreeMap<String, crate::content::project::ProjectV2SourceIdentityBinding>,
        ContentError,
    > {
        let data: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../../../../imports/crystalserver/bindings/items.json"
        ))
        .map_err(|_| invalid("test protected imported Item bindings"))?;
        let rows: Vec<crate::content::project::ProjectV2SourceIdentityBinding> =
            serde_json::from_value(data["bindings"].clone())
                .map_err(|_| invalid("test exact Item binding records"))?;
        Ok(rows
            .into_iter()
            .map(|row| (row.external_id.clone(), row))
            .collect())
    }
    #[test]
    fn srx3_movement_binding_closure_reuses_exact_eight_and_only_inserts_five_without_factory()
    -> Result<(), ContentError> {
        let rows = qualified_records()?;
        assert_eq!(rows.len(), 13);
        let mut map = imports()?;
        let count = map.len();
        admit_exact_bindings(&mut map, &rows)?;
        assert_eq!(map.len(), count + 5);
        for row in rows {
            assert_eq!(
                map.get(&row.production_binding.external_id),
                Some(&row.production_binding)
            );
        }
        Ok(())
    }
    #[test]
    fn srx3_movement_binding_closure_rejects_changed_protected_and_duplicate_new_without_factory()
    -> Result<(), ContentError> {
        let rows = qualified_records()?;
        for id in [
            "20121", "23726", "23727", "23728", "33004", "37000", "43294", "43295",
        ] {
            let mut map = imports()?;
            let row = map
                .get_mut(id)
                .ok_or_else(|| invalid("test protected existing identity"))?;
            row.target.key = "oteryn:item.tibia.i100".into();
            assert!(
                admit_exact_bindings(&mut map, &rows).is_err(),
                "changed protected{id}"
            );
        }
        for id in ["43589", "43590", "42851", "42853", "43625"] {
            let mut map = imports()?;
            let row = rows
                .iter()
                .find(|row| row.production_binding.external_id == id)
                .ok_or_else(|| invalid("test genuine new identity"))?;
            map.insert(id.into(), row.production_binding.clone());
            assert!(
                admit_exact_bindings(&mut map, &rows).is_err(),
                "duplicate new{id}"
            );
        }
        let mut unknown = rows[0].clone();
        unknown.production_binding.external_id = "99999".into();
        assert!(admit_exact_bindings(&mut imports()?, &[unknown]).is_err());
        Ok(())
    }
}
