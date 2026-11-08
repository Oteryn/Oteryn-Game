//! Three Wiki-own-ID/client-qualified Cook materials; no numeric fallback.
use super::*;
type Binding = super::super::project::ProjectV2SourceIdentityBinding;
include!("questpc5_cook_material_pin.rs");

pub(super) fn extend(admitted: &mut BTreeMap<String, Binding>) -> Result<(), ContentError> {
    let raw = include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/questpc5-cook-material-bindings.json"
    );
    if hex(sha256(raw)) != SOURCE_SHA256 {
        return Err(invalid("Cook material binding proof digest"));
    }
    let packet: serde_json::Value =
        serde_json::from_slice(raw).map_err(|_| invalid("Cook material binding proof"))?;
    let bindings = packet["bindings"]
        .as_array()
        .filter(|rows| rows.len() == 3 && packet["schema"] == 1)
        .ok_or_else(|| invalid("Cook material binding scope"))?;
    for (row, id) in bindings.iter().zip(["44525", "44527", "44528"]) {
        let binding: Binding =
            serde_json::from_value(row.clone()).map_err(|_| invalid("Cook material binding"))?;
        if binding.external_id != id
            || binding.source_key != "oteryn:source.tibiawiki"
            || binding.identity_namespace != "tibiawiki/item_id"
            || binding.target.family != super::super::project::ProjectV2Family::Item
            || binding.target.revision != "definition-r1"
            || binding.target.key != format!("oteryn:item.tibia.i{id}")
        {
            return Err(invalid("Cook material exact identity"));
        }
        if admitted.insert(id.to_owned(), binding).is_some() {
            return Err(invalid("Cook material binding conflict"));
        }
    }
    Ok(())
}
