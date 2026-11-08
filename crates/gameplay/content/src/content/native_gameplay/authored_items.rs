//! First-party constructors have their own namespace and exact generated admission closure.
use super::super::project::ProjectV2SourceIdentityBinding;
use super::*;

pub(super) const SOURCE: &str = "oteryn:source.oteryn_authored";
const NAMESPACE: &str = "oteryn/static_item_constructor";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    schema: String,
    records: Vec<Admission>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Admission {
    binding: ProjectV2SourceIdentityBinding,
    profile_sha256: String,
}

fn canonical(value: &serde_json::Value, output: &mut Vec<u8>) -> Result<(), ContentError> {
    use serde_json::Value;
    match value {
        Value::Object(fields) => {
            output.push(b'{');
            let ordered = fields.iter().collect::<BTreeMap<_, _>>();
            for (index, (key, value)) in ordered.into_iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                output.extend(
                    serde_json::to_vec(key).map_err(|_| invalid("authored constructor key"))?,
                );
                output.push(b':');
                canonical(value, output)?;
            }
            output.push(b'}');
        }
        Value::Array(values) => {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                canonical(value, output)?;
            }
            output.push(b']');
        }
        _ => output
            .extend(serde_json::to_vec(value).map_err(|_| invalid("authored constructor scalar"))?),
    }
    Ok(())
}

/// Admit only exact first-party rows. Imported source bindings are never overwritten.
pub(super) fn admit(
    items: &ItemProfilesDocument,
    raw: &[u8],
    admitted: &mut BTreeMap<String, ProjectV2SourceIdentityBinding>,
) -> Result<(), ContentError> {
    if !items
        .records
        .iter()
        .any(|record| record.production_binding.source_key == SOURCE)
    {
        return Ok(());
    }
    let packet: Packet = serde_json::from_str(include_str!(
        "../../../../../../content/items/authored-static-admissions.json"
    ))
    .map_err(|_| invalid("authored constructor admission packet"))?;
    if packet.schema != "OTERYN_AUTHORED_STATIC_ITEM_ADMISSIONS/v1" || packet.records.len() > 64 {
        return Err(invalid("authored constructor admission bounds"));
    }
    let document: serde_json::Value =
        serde_json::from_slice(raw).map_err(|_| invalid("authored constructor raw profiles"))?;
    let rows = document["records"]
        .as_array()
        .ok_or_else(|| invalid("authored constructor raw rows"))?;
    if rows.len() != items.records.len() {
        return Err(invalid("authored constructor row count"));
    }
    for (index, record) in items
        .records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.production_binding.source_key == SOURCE)
    {
        let binding = &record.production_binding;
        let external = binding
            .external_id
            .parse::<u32>()
            .map_err(|_| invalid("authored constructor identity"))?;
        let matches = packet
            .records
            .iter()
            .filter(|entry| entry.binding == *binding)
            .collect::<Vec<_>>();
        if external == 0
            || external.to_string() != binding.external_id
            || binding.identity_namespace != NAMESPACE
            || binding.target.key != format!("oteryn:item.tibia.i{external}")
            || binding.target.family != ProjectV2Family::Item
            || binding.source_revision.len() != 64
            || !binding
                .source_revision
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || matches.len() != 1
            || admitted.contains_key(&binding.external_id)
            || record
                .production_binding_qualification
                .as_ref()
                .is_none_or(|qualification| {
                    qualification["schema"] != "OTERYN_FIRST_PARTY_STATIC_CONSTRUCTOR/v1"
                        || qualification["basis"] != "CHOSEN_OTERYN_APPROXIMATION"
                })
        {
            return Err(invalid("authored constructor binding or qualification"));
        }
        let mut bytes = Vec::new();
        canonical(&rows[index], &mut bytes)?;
        if hex(sha256(&bytes)) != matches[0].profile_sha256 {
            return Err(invalid("authored constructor profile closure"));
        }
        use super::super::ReferenceItemField as F;
        if !record.admission.materializable
            || record.admission.stack_class != NativeItemStackClass::NonStackable
            || record.admission.legal_destinations
                != [
                    NativeItemDestination::CharacterInventory,
                    NativeItemDestination::Ground,
                ]
            || !matches!(&record.semantics.physical, F::Known(value)
                if value.movable == F::Known(true) && value.pickupable == F::Known(true)
                && matches!(value.weight, F::Known(1..=u32::MAX)))
            || !matches!(&record.semantics.stack, F::Known(value) if value.stackable == F::Known(false))
            || !matches!(record.semantics.container, F::NotApplicable)
            || !record.attributes.spell_augments.is_empty()
            || record.attributes.speed_bonus != Some(0)
            || record.attributes.field_condition.is_some()
            || record.attributes.rune_consumption.is_some()
        {
            return Err(invalid("authored constructor plain custody"));
        }
        admitted.insert(binding.external_id.clone(), binding.clone());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::super::test_support::{input, pinned};
    use super::*;
    use crate::content::production::test_source;

    #[test]
    fn authored_static_profiles_are_bound_to_exact_constructor_admissions() {
        let world = test_source(1).unwrap().world_id;
        let baseline = crate::content::qualify_native_entry_room(world).unwrap();
        let raw = include_bytes!(
            "../../../../../../content/items/definitions/spell-native-profiles.json"
        );
        let original: serde_json::Value = serde_json::from_slice(raw).unwrap();
        let indexes = original["records"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .filter_map(|(index, record)| {
                (record["production_binding"]["source_key"] == SOURCE).then_some(index)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            indexes.len(),
            4,
            "four explicitly declared first-party constructors"
        );
        let mut supplied = input();
        supplied.item_profiles = Some(pinned(raw));
        compile_native_gameplay(baseline.compiled(), &supplied)
            .expect("declared constructor profiles");
        for index in indexes {
            for case in 0..5 {
                let mut changed = original.clone();
                let record = &mut changed["records"][index];
                match case {
                    0 => {
                        record["semantics"]["physical"]["value"]["weight"]["value"] =
                            serde_json::json!(0)
                    }
                    1 => {
                        record["production_binding"]["source_key"] =
                            serde_json::json!("oteryn:source.crystalserver")
                    }
                    2 => record["production_binding_qualification"] = serde_json::Value::Null,
                    3 => record["admission"]["stack_class"] = serde_json::json!("StackCapable"),
                    _ => record["attributes"]["speed_bonus"] = serde_json::json!(1),
                }
                supplied.item_profiles = Some(pinned(&serde_json::to_vec(&changed).unwrap()));
                assert!(
                    compile_native_gameplay(baseline.compiled(), &supplied).is_err(),
                    "record{index} case{case}"
                );
            }
        }
    }
}
