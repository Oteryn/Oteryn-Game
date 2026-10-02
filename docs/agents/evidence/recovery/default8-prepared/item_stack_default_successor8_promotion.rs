//! Closed successor8 documented defaults; prior cohorts and Item admission unchanged.
use super::{
    ProjectReferenceRecord, ProjectV2Draft,
    item_stack_false_promotion::{StackFalsePromotion, apply_rows},
    world_project_sha256,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-promotion-v1.json"
);
pub const ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET_SHA256: &str =
    "PENDING_ACTUAL_PARENT_PACKET_COMPILATION";

fn validate_targets(draft: &ProjectV2Draft, bytes: &[u8]) -> Result<(), String> {
    let packet: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let rows = packet["promotions"].as_array().ok_or("missing successor8 rows")?;
    let ids: BTreeSet<u64> = [23229, 23230, 23231, 23232, 23295, 23299, 23335, 23339]
        .into_iter().collect();
    let mut seen_ids = BTreeSet::new();
    let mut targets = BTreeMap::new();
    for row in rows {
        let iid = row["source_item_id"].as_u64().ok_or("invalid successor8 source ID")?;
        let key = row["item_key"].as_str().ok_or("invalid successor8 key")?;
        if !ids.contains(&iid) || !seen_ids.insert(iid)
            || row["target"]["family"] != "Item"
            || row["target"]["key"] != key
            || row["target"]["revision"] != "definition-r1"
            || targets.insert(key, &row["target"]).is_some()
        {
            return Err("successor8 scope/full target drift".into());
        }
    }
    if seen_ids != ids { return Err("incomplete successor8 scope".into()); }
    let mut found = BTreeSet::new();
    for record in &draft.core.records {
        if let ProjectReferenceRecord::Item { identity, .. } = record {
            if let Some(target) = targets.get(identity.key.as_str()) {
                if !found.insert(identity.key.as_str())
                    || target["family"] != identity.family
                    || target["key"] != identity.key
                    || target["revision"] != identity.revision
                {
                    return Err("successor8 duplicate/current full identity drift".into());
                }
            }
        }
    }
    if found.len() != 8 { return Err("successor8 missing current Item".into()); }
    Ok(())
}

pub fn apply_item_stack_default_successor8_promotion_v1(
    draft: &mut ProjectV2Draft,
) -> Result<StackFalsePromotion, String> {
    if world_project_sha256(ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET)
        != ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET_SHA256
    {
        return Err("successor8 packet digest drift".into());
    }
    validate_targets(draft, ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET)?;
    apply_rows(draft.core.records.iter_mut().filter_map(|r| match r {
        ProjectReferenceRecord::Item { identity, stack_class, semantics, .. } =>
            Some((identity.key.as_str(), *stack_class, semantics)),
        _ => None,
    }), ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET, 8)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::content::{
        ItemStackDocument, ReferenceItemField, ReferenceItemSemantics, ReferenceItemStack,
    };

    fn source_keys() -> Vec<String> {
        serde_json::from_slice::<Value>(ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET)
            .expect("packet")["promotions"].as_array().expect("rows").iter()
            .map(|r| r["item_key"].as_str().expect("key").to_owned()).collect()
    }

    #[test]
    fn exact_eight_preserve_maxima_and_are_idempotent() {
        let keys = source_keys();
        let mut items = vec![ReferenceItemSemantics {
            stack: ReferenceItemField::Known(ReferenceItemStack {
                stackable: ReferenceItemField::Unknown,
                stack_max: ReferenceItemField::Known(7),
            }),
            ..ReferenceItemSemantics::default()
        }; 8];
        for changed in [8, 0] {
            assert_eq!(apply_rows(keys.iter().zip(items.iter_mut())
                .map(|(k, v)| (k.as_str(), ItemStackDocument::Unknown, v)),
                ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET, 8),
                Ok(StackFalsePromotion { fields: 8, changed }));
        }
        for item in items {
            assert_eq!(item.stack, ReferenceItemField::Known(ReferenceItemStack {
                stackable: ReferenceItemField::Known(false),
                stack_max: ReferenceItemField::Known(7),
            }));
        }
    }

    #[test]
    fn last_missing_or_conflicting_item_cannot_partially_write() {
        let keys = source_keys();
        for missing in [false, true] {
            let mut items = vec![ReferenceItemSemantics::default(); 8];
            if !missing { items[7].stack = ReferenceItemField::Conflict; }
            let before = items.clone();
            let supplied = if missing { 7 } else { 8 };
            assert!(apply_rows(keys.iter().zip(items.iter_mut()).take(supplied)
                .map(|(k, v)| (k.as_str(), ItemStackDocument::Unknown, v)),
                ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET, 8).is_err());
            assert_eq!(items, before);
        }
    }
}
