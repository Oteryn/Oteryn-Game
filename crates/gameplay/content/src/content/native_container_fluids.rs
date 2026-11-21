//! Source fluid arguments are distinct from the quantity of fresh vessels.
use serde_json::Value;

use sha2::{Digest, Sha256};
use std::sync::OnceLock;

const SOURCE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../tools/content-schema/reward-claim-authoring/container_native_source.json"
));

pub(super) fn source_subtype(
    row: &Value,
    unique: Option<u64>,
    ordinal: usize,
) -> Option<Option<u8>> {
    let Some(subtype) = row.get("fluid_subtype") else {
        return Some(None);
    };
    // Literal one is unchanged by the donor constructor's lower/upper clamp.
    // Other arguments stay held until that enum/clamp mapping is qualified.
    if subtype.as_u64() != Some(1)
        || row["count"].as_u64() != Some(1)
        || row.get("text").is_some()
        || row.get("key_number").is_some()
    {
        return None;
    }
    static PARSED: OnceLock<Option<Value>> = OnceLock::new();
    let source = PARSED
        .get_or_init(|| {
            let digest: String = Sha256::digest(SOURCE)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            if digest != super::CONTAINER_NATIVE_SOURCE_SHA256 {
                return None;
            }
            serde_json::from_slice(SOURCE).ok()
        })
        .as_ref()?;
    let unique = unique?;
    let entry = source["entries"]
        .as_array()?
        .iter()
        .find(|entry| entry["unique_id"].as_u64() == Some(unique))?;
    let argument = entry["setting"]["reward"]
        .as_array()?
        .get(ordinal)?
        .as_array()?;
    if argument.len() != 2
        || argument[1].as_u64() != Some(1)
        || row["item"]["family"] != "Item"
        || row["item"]["key"].as_str()? != format!("oteryn:item.tibia.i{}", argument[0].as_u64()?)
    {
        return None;
    }
    Some(Some(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_container_fluid_requires_exact_donor_argument_and_child() -> Result<(), String> {
        let row = serde_json::json!({"item":{"family":"Item","key":"oteryn:item.tibia.i2874","revision":"definition-r1"},"count":1,"fluid_subtype":1});
        for unique in 6197..=6200 {
            assert_eq!(source_subtype(&row, Some(unique), 1), Some(Some(1)));
        }
        for mutation in 0..9 {
            let mut invalid = row.clone();
            let mut unique = Some(6197);
            let mut ordinal = 1;
            match mutation {
                0 => unique = None,
                1 => ordinal = 0,
                2 => invalid["item"]["key"] = "oteryn:item.tibia.i3602".into(),
                3 => invalid["fluid_subtype"] = 2.into(),
                4 => invalid["fluid_subtype"] = true.into(),
                5 => invalid["fluid_subtype"] = Value::Null,
                6 => invalid["count"] = 2.into(),
                7 => invalid["text"] = "unexpected".into(),
                _ => invalid["key_number"] = 3012.into(),
            }
            assert_eq!(
                source_subtype(&invalid, unique, ordinal),
                None,
                "mutation={mutation}"
            );
        }
        let mut ordinary = row;
        ordinary
            .as_object_mut()
            .ok_or("row")?
            .remove("fluid_subtype");
        assert_eq!(source_subtype(&ordinary, None, 0), Some(None));
        Ok(())
    }
}
