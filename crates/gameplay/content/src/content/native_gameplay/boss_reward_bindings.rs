//! Nine exact reward identities absent from the historical XML-only binding
//! partition;43860 has a separately declared reference-only canonical Item. Pinned DAT initialization + Item factory + actual boss loot
//! qualify these exact source IDs; no alias, identity allocation or Item use follows.
use super::*;

#[path = "boss_reward_bindings_pin.rs"]
mod pin;

pub(super) fn qualified_records() -> Result<Vec<ItemProfileRecord>, ContentError> {
    let capture = include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/boss-reward-yield-profiles.json"
    );
    let packet = include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/candidate-bindings/boss-reward-items-candidate.json"
    );
    if hex(sha256(capture)) != pin::CAPTURE_SHA256 || hex(sha256(packet)) != pin::PACKET_SHA256 {
        return Err(invalid("native boss reward binding source hash"));
    }
    let capture: serde_json::Value =
        serde_json::from_slice(capture).map_err(|_| invalid("native boss reward Item capture"))?;
    let packet: serde_json::Value =
        serde_json::from_slice(packet).map_err(|_| invalid("native boss reward binding packet"))?;
    if capture["schema"] != "OTERYN_NATIVE_BOSS_REWARD_YIELD_PROFILES/v1"
        || packet["schema"] != "OTERYN_NATIVE_BOSS_REWARD_BINDING_EXTENSION/v1"
        || packet["source_revision"] != "201189f871d23597b82caf88fd611454a2ddefb8"
    {
        return Err(invalid("native boss reward binding discriminator"));
    }
    let bindings: Vec<super::super::project::ProjectV2SourceIdentityBinding> =
        serde_json::from_value(packet["bindings"].clone())
            .map_err(|_| invalid("native boss reward binding shape"))?;
    if bindings.len() != 9 {
        return Err(invalid("native boss reward binding bound"));
    }
    for (binding, external) in bindings.iter().zip([
        "43778", "43779", "43780", "43781", "43782", "43860", "43946", "43947", "44048",
    ]) {
        if binding.external_id != external
            || binding.source_key != "oteryn:source.crystalserver"
            || binding.source_revision != "201189f871d23597b82caf88fd611454a2ddefb8"
            || binding.identity_namespace != "ots/item_server_id"
            || binding.disposition != super::super::ProjectV2SourceIdentityDisposition::Exact
            || binding.target.family != ProjectV2Family::Item
            || binding.target.key != format!("oteryn:item.tibia.i{external}")
            || binding.target.revision != "definition-r1"
        {
            return Err(invalid("native boss reward exact existing identity"));
        }
    }
    let records: Vec<ItemProfileRecord> = serde_json::from_value(capture["records"].clone())
        .map_err(|_| invalid("native boss reward source Item profiles"))?;
    let records = records
        .into_iter()
        .filter(|record| {
            matches!(
                record.production_binding.external_id.as_str(),
                "43778"
                    | "43779"
                    | "43780"
                    | "43781"
                    | "43782"
                    | "43860"
                    | "43946"
                    | "43947"
                    | "44048"
            )
        })
        .collect::<Vec<_>>();
    if records.len() != 9 {
        return Err(invalid("native boss reward complete source Item set"));
    }
    for (record, binding) in records.iter().zip(&bindings) {
        use super::super::ReferenceItemField as F;
        if &record.production_binding != binding
            || record.production_binding_qualification.as_ref() != Some(&packet)
            || record.production_definition.family != "Item"
            || record.production_definition.production_key != binding.target.key
            || record.production_definition.revision_ref != binding.target.revision
            || !record.admission.materializable
            || record.admission.stack_class != NativeItemStackClass::NonStackable
            || record.admission.legal_destinations
                != [
                    NativeItemDestination::CharacterInventory,
                    NativeItemDestination::Ground,
                ]
            || !matches!(&record.semantics.physical, F::Known(value)
                if value.weight==F::Known(0) && value.movable==F::Known(true) && value.pickupable==F::Known(true))
        {
            return Err(invalid("native boss reward closed Item factory policy"));
        }
    }
    Ok(records)
}
