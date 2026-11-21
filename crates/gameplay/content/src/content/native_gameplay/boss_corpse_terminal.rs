//! Closed source43666 binding: the real Elder corpse's XML-declared terminal.
use super::*;
#[path = "boss_corpse_terminal_pin.rs"]
mod pin;

pub(super) fn qualified_record() -> Result<ItemProfileRecord, ContentError> {
    let capture = include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/boss-corpse-item-profiles.json"
    );
    let packet = include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/candidate-bindings/elder-corpse-terminal-candidate.json"
    );
    if hex(sha256(capture)) != pin::CAPTURE_SHA256 || hex(sha256(packet)) != pin::PACKET_SHA256 {
        return Err(invalid("native Elder corpse terminal source hash"));
    }
    let capture: serde_json::Value = serde_json::from_slice(capture)
        .map_err(|_| invalid("native Elder corpse source capture"))?;
    let packet: serde_json::Value = serde_json::from_slice(packet)
        .map_err(|_| invalid("native Elder corpse binding packet"))?;
    if capture["schema"] != "OTERYN_NATIVE_BOSS_CORPSE_ITEM_PROFILES/v1"
        || capture["source_revision"] != "201189f871d23597b82caf88fd611454a2ddefb8"
        || packet["schema"] != "OTERYN_NATIVE_ELDER_CORPSE_TERMINAL_BINDINGS/v1"
        || packet["source_revision"] != "201189f871d23597b82caf88fd611454a2ddefb8"
        || packet["evidence"]["ordinal"] != 25377
        || packet["evidence"]["reference_disposition"] != "EXISTING_CANONICAL_REFERENCE"
        || packet["evidence"]["source_chain"] != serde_json::json!([43669, 43668, 43667, 43666])
        || packet["evidence"]["source_durations_seconds"] != serde_json::json!([10, 300, 300, 0])
    {
        return Err(invalid("native Elder corpse terminal source identity"));
    }
    let bindings: Vec<super::super::project::ProjectV2SourceIdentityBinding> =
        serde_json::from_value(packet["bindings"].clone())
            .map_err(|_| invalid("native Elder corpse exact binding"))?;
    let [binding] = bindings.as_slice() else {
        return Err(invalid("native Elder corpse binding bound"));
    };
    if binding.external_id != "43666"
        || binding.source_key != "oteryn:source.crystalserver"
        || binding.source_revision != "201189f871d23597b82caf88fd611454a2ddefb8"
        || binding.identity_namespace != "ots/item_server_id"
        || binding.disposition != super::super::ProjectV2SourceIdentityDisposition::Exact
        || binding.target.family != ProjectV2Family::Item
        || binding.target.key != "oteryn:item.tibia.i43666"
        || binding.target.revision != "definition-r1"
    {
        return Err(invalid("native Elder corpse exact existing reference"));
    }
    let records: Vec<ItemProfileRecord> = serde_json::from_value(capture["records"].clone())
        .map_err(|_| invalid("native Elder corpse source records"))?;
    if records.len() != 199 {
        return Err(invalid("native Elder corpse complete source closure"));
    }
    let mut matches = records
        .into_iter()
        .filter(|row| row.production_binding.external_id == "43666");
    let record = matches
        .next()
        .ok_or_else(|| invalid("native Elder corpse terminal missing"))?;
    if matches.next().is_some() {
        return Err(invalid("native Elder corpse terminal duplicate"));
    }
    use super::super::ReferenceItemField as F;
    if &record.production_binding != binding
        || record.production_binding_qualification.as_ref() != Some(&packet)
        || record.production_definition.family != "Item"
        || record.production_definition.production_key != binding.target.key
        || record.production_definition.revision_ref != binding.target.revision
        || !record.admission.materializable
        || record.admission.stack_class != NativeItemStackClass::NonStackable
        || record.admission.legal_destinations != [NativeItemDestination::Ground]
        || !matches!(&record.semantics.physical,F::Known(value)
            if value.weight==F::Known(0) && value.movable==F::Known(true) && value.pickupable==F::Known(false))
        || record.semantics.container != F::NotApplicable
        || record.semantics.temporal != F::NotApplicable
    {
        return Err(invalid("native Elder corpse terminal closed Ground policy"));
    }
    Ok(record)
}
