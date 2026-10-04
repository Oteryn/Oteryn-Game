//! Closed successor8 documented defaults; prior cohorts and Item admission unchanged.
use super::{
    DefinitionIdentityDocument, ProjectReferenceRecord, ProjectV2Draft,
    item_stack_false_promotion::{StackFalsePromotion, apply_rows},
    world_project_sha256,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-promotion-v1.json"
);
pub const ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET_SHA256: &str =
    "aaad7f5cd2b96a60e42aec5dbce1e0f48e33f338009974ee2fbdca38436c970a";

fn validate_targets<'a>(
    identities: impl Iterator<Item = &'a DefinitionIdentityDocument>,
    bytes: &[u8],
) -> Result<(), String> {
    let packet: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let rows = packet["promotions"]
        .as_array()
        .ok_or("missing successor8 rows")?;
    let ids: BTreeSet<u64> = [23229, 23230, 23231, 23232, 23295, 23299, 23335, 23339]
        .into_iter()
        .collect();
    let mut native = BTreeMap::new();
    for identity in identities {
        if native.insert(identity.key.as_str(), identity).is_some() {
            return Err("duplicate current Item identity".into());
        }
    }
    let mut seen_ids = BTreeSet::new();
    let mut seen_keys = BTreeSet::new();
    for row in rows {
        let iid = row["source_item_id"]
            .as_u64()
            .ok_or("invalid successor8 source ID")?;
        let target: DefinitionIdentityDocument =
            serde_json::from_value(row["target"].clone()).map_err(|e| e.to_string())?;
        if !ids.contains(&iid)
            || !seen_ids.insert(iid)
            || !seen_keys.insert(target.key.clone())
            || target.family != "Item"
            || target.revision != "definition-r1"
            || row["appearance_id"].as_u64() != Some(iid)
            || row["binding"]["external_id"].as_str() != Some(iid.to_string().as_str())
            || row["binding"]["target"] != row["target"]
            || row["item_key"].as_str() != Some(target.key.as_str())
            || native.get(target.key.as_str()).copied() != Some(&target)
        {
            return Err("successor8 scope/full current target drift".into());
        }
    }
    if seen_ids != ids {
        return Err("incomplete successor8 scope".into());
    }
    Ok(())
}

fn apply_records(
    records: &mut [ProjectReferenceRecord],
    bytes: &[u8],
) -> Result<StackFalsePromotion, String> {
    validate_targets(
        records.iter().filter_map(|r| match r {
            ProjectReferenceRecord::Item { identity, .. } => Some(identity),
            _ => None,
        }),
        bytes,
    )?;
    apply_rows(
        records.iter_mut().filter_map(|r| match r {
            ProjectReferenceRecord::Item {
                identity,
                stack_class,
                semantics,
                ..
            } => Some((identity.key.as_str(), *stack_class, semantics)),
            _ => None,
        }),
        bytes,
        8,
    )
}

pub fn apply_item_stack_default_successor8_promotion_v1(
    draft: &mut ProjectV2Draft,
) -> Result<StackFalsePromotion, String> {
    if world_project_sha256(ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET)
        != ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET_SHA256
    {
        return Err("successor8 packet digest drift".into());
    }
    apply_records(
        &mut draft.core.records,
        ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET,
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::content::{ItemStackDocument, ReferenceItemField, ReferenceItemStack};
    fn fixture() -> (Vec<ProjectReferenceRecord>, Value) {
        let packet: Value =
            serde_json::from_slice(ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET).expect("packet");
        let mut records: Vec<ProjectReferenceRecord> = packet["promotions"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|r| {
                serde_json::from_value(serde_json::json!({"kind":"Item", "identity":r["target"],
                "client_projection":"ClientSafe", "materializable":false, "stack_class":"Unknown",
                "semantics":{"stack":{"state":"KNOWN","value":{"stackable":{"state":"UNKNOWN"},
                    "stack_max":{"state":"KNOWN","value":7}}}}}))
                .expect("Item fixture")
            })
            .collect();
        records.push(
            serde_json::from_value(serde_json::json!({"kind":"Item",
            "identity":{"family":"Item","key":"test:item.outside","revision":"definition-r1"},
            "client_projection":"ClientSafe","materializable":false,"stack_class":"Unknown"}))
            .expect("outside Item"),
        );
        (records, packet)
    }
    fn apply(
        records: &mut [ProjectReferenceRecord],
        packet: &Value,
    ) -> Result<StackFalsePromotion, String> {
        apply_records(records, &serde_json::to_vec(packet).expect("packet bytes"))
    }
    #[test]
    fn full_preguard_then_eight_preserve_maxima_and_are_idempotent() {
        let (mut records, packet) = fixture();
        let outside = records[8].clone();
        for changed in [8, 0] {
            assert_eq!(
                apply(&mut records, &packet),
                Ok(StackFalsePromotion { fields: 8, changed })
            );
        }
        assert_eq!(records[8], outside);
        for record in &records[..8] {
            if let ProjectReferenceRecord::Item {
                semantics,
                materializable,
                stack_class,
                ..
            } = record
            {
                assert!(!*materializable);
                assert_eq!(*stack_class, ItemStackDocument::Unknown);
                assert_eq!(
                    semantics.stack,
                    ReferenceItemField::Known(ReferenceItemStack {
                        stackable: ReferenceItemField::Known(false),
                        stack_max: ReferenceItemField::Known(7)
                    })
                );
            }
        }
    }
    #[test]
    fn full_target_source_scope_and_current_duplicates_fail_before_mutation() {
        for variant in 0..12 {
            let (mut records, mut packet) = fixture();
            match variant {
                0 => packet["promotions"][7]["source_item_id"] = Value::from(1),
                1 => packet["promotions"][7]["target"]["family"] = Value::from("Creature"),
                2 => packet["promotions"][7]["target"]["revision"] = Value::from("foreign"),
                3 => packet["promotions"][7]["item_key"] = Value::from("outside"),
                4 => packet["promotions"][7] = packet["promotions"][0].clone(),
                5 => {
                    packet["promotions"].as_array_mut().expect("rows").pop();
                }
                6 => {
                    records.remove(7);
                }
                7 => {
                    records.push(records[7].clone());
                }
                9 => packet["promotions"][7]["appearance_id"] = Value::from(23229),
                10 => packet["promotions"][7]["binding"]["external_id"] = Value::from("23229"),
                11 => {
                    packet["promotions"][7]["binding"]["target"]["revision"] =
                        Value::from("foreign")
                }
                _ => {
                    if let ProjectReferenceRecord::Item { identity, .. } = &mut records[7] {
                        identity.revision = "foreign".into();
                    }
                }
            }
            let before = records.clone();
            assert!(apply(&mut records, &packet).is_err(), "variant {variant}");
            assert_eq!(records, before);
        }
    }
    #[test]
    fn late_blocked_group_leaf_true_and_class_fail_atomically_after_preguard() {
        for variant in 0..5 {
            let (mut records, packet) = fixture();
            if let ProjectReferenceRecord::Item {
                semantics,
                stack_class,
                ..
            } = &mut records[7]
            {
                match variant {
                    0 => semantics.stack = ReferenceItemField::Conflict,
                    1 => semantics.stack = ReferenceItemField::NotApplicable,
                    2 | 3 => {
                        semantics.stack = ReferenceItemField::Known(ReferenceItemStack {
                            stackable: if variant == 2 {
                                ReferenceItemField::Known(true)
                            } else {
                                ReferenceItemField::NotApplicable
                            },
                            stack_max: ReferenceItemField::Known(7),
                        })
                    }
                    _ => *stack_class = ItemStackDocument::StackCapable,
                }
            }
            let before = records.clone();
            assert!(apply(&mut records, &packet).is_err());
            assert_eq!(records, before);
        }
    }
}
