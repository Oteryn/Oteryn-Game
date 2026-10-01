//! Explicit agreed wiki non-stackability, independent of Item identity admission.
use super::{
    ItemStackDocument, ProjectReferenceRecord, ProjectV2Draft, ReferenceItemField,
    ReferenceItemSemantics, ReferenceItemStack, world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const ITEM_STACK_FALSE_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-item-stack-false-promotion-v1.json"
);
pub const ITEM_STACK_FALSE_PACKET_SHA256: &str =
    "47a5e0626d1fc14948a68c1f85c3908eac9c148c157816ca13da4ea285af5659";
#[derive(Deserialize)]
struct Packet {
    schema: String,
    counts: Counts,
    promotions: Vec<Row>,
}
#[derive(Deserialize)]
struct Counts {
    promotions: usize,
}
#[derive(Deserialize)]
struct Row {
    item_key: String,
    stackable: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub struct StackFalsePromotion {
    pub fields: usize,
    pub changed: usize,
}

/// Apply after starter Item admission. Identity classes and stack maxima never change.
pub fn apply_item_stack_false_promotion_v1(
    draft: &mut ProjectV2Draft,
) -> Result<StackFalsePromotion, String> {
    if world_project_sha256(ITEM_STACK_FALSE_PACKET) != ITEM_STACK_FALSE_PACKET_SHA256 {
        return Err("stack-false packet digest drift".into());
    }
    let items = draft.core.records.iter_mut().filter_map(|r| match r {
        ProjectReferenceRecord::Item {
            identity,
            stack_class,
            semantics,
            ..
        } => Some((identity.key.as_str(), *stack_class, semantics)),
        _ => None,
    });
    apply_rows(items, ITEM_STACK_FALSE_PACKET, 2380)
}

fn current(semantics: &ReferenceItemSemantics) -> Result<Option<bool>, String> {
    use ReferenceItemField::{Known, Unknown};
    match &semantics.stack {
        Unknown => Ok(None),
        Known(stack) => match stack.stackable {
            Unknown => Ok(None),
            Known(value) => Ok(Some(value)),
            _ => Err("blocked stackable evidence state".into()),
        },
        _ => Err("blocked stack group evidence state".into()),
    }
}

fn apply_rows<'a>(
    items: impl Iterator<Item = (&'a str, ItemStackDocument, &'a mut ReferenceItemSemantics)>,
    bytes: &[u8],
    expected: usize,
) -> Result<StackFalsePromotion, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.schema != "OTERYN_ITEM_STACK_FALSE_PROMOTION/v1"
        || packet.counts.promotions != expected
        || packet.promotions.len() != expected
    {
        return Err("stack-false packet shape/count drift".into());
    }
    let mut items: BTreeMap<_, _> = items
        .map(|(key, class, value)| (key, (class, value)))
        .collect();
    let mut seen = BTreeSet::new();
    let mut changed = 0;
    for row in &packet.promotions {
        if row.stackable || !seen.insert(&row.item_key) {
            return Err("invalid or duplicate stack-false row".into());
        }
        let (class, semantics) = items
            .get(row.item_key.as_str())
            .ok_or_else(|| format!("stack-false has no Item: {}", row.item_key))?;
        if *class == ItemStackDocument::StackCapable || current(semantics)? == Some(true) {
            return Err("known stackable conflict".into());
        }
        changed += usize::from(current(semantics)?.is_none());
    }
    // All rows are qualified before the first write; mutate only the target field.
    for row in &packet.promotions {
        let (_, semantics) = items
            .get_mut(row.item_key.as_str())
            .ok_or_else(|| "validated stack-false Item missing".to_owned())?;
        if matches!(semantics.stack, ReferenceItemField::Unknown) {
            semantics.stack = ReferenceItemField::Known(ReferenceItemStack {
                stackable: ReferenceItemField::Unknown,
                stack_max: ReferenceItemField::Unknown,
            });
        }
        if let ReferenceItemField::Known(stack) = &mut semantics.stack {
            stack.stackable = ReferenceItemField::Known(false);
        }
    }
    Ok(StackFalsePromotion {
        fields: expected,
        changed,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    fn packet(rows: serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(
            &serde_json::json!({"schema":"OTERYN_ITEM_STACK_FALSE_PROMOTION/v1",
            "counts":{"promotions":rows.as_array().expect("rows").len()},"promotions":rows}),
        )
        .expect("packet")
    }
    fn row(key: &str) -> serde_json::Value {
        serde_json::json!({"item_key":key,"stackable":false})
    }
    #[test]
    fn preserves_class_and_maximum_and_is_idempotent() {
        let mut item = ReferenceItemSemantics {
            stack: ReferenceItemField::Known(ReferenceItemStack {
                stackable: ReferenceItemField::Unknown,
                stack_max: ReferenceItemField::Known(7),
            }),
            ..ReferenceItemSemantics::default()
        };
        let bytes = packet(serde_json::json!([row("item")]));
        for changed in [1, 0] {
            assert_eq!(
                apply_rows(
                    std::iter::once(("item", ItemStackDocument::Unknown, &mut item)),
                    &bytes,
                    1
                ),
                Ok(StackFalsePromotion { fields: 1, changed })
            );
        }
        if let ReferenceItemField::Known(stack) = item.stack {
            assert_eq!(stack.stack_max, ReferenceItemField::Known(7));
        }
    }
    #[test]
    fn rejects_conflicts_blocks_duplicates_missing_items_atomically() {
        for state in [
            ReferenceItemField::Conflict,
            ReferenceItemField::NotApplicable,
            ReferenceItemField::Known(ReferenceItemStack {
                stackable: ReferenceItemField::Known(true),
                stack_max: ReferenceItemField::Known(100),
            }),
        ] {
            let mut item = ReferenceItemSemantics {
                stack: state,
                ..ReferenceItemSemantics::default()
            };
            let before = item.clone();
            assert!(
                apply_rows(
                    std::iter::once(("item", ItemStackDocument::Unknown, &mut item)),
                    &packet(serde_json::json!([row("item")])),
                    1
                )
                .is_err()
            );
            assert_eq!(item, before);
        }
        for rows in [
            serde_json::json!([row("item"), row("item")]),
            serde_json::json!([row("item"), row("missing")]),
        ] {
            let mut item = ReferenceItemSemantics::default();
            assert!(
                apply_rows(
                    std::iter::once(("item", ItemStackDocument::Unknown, &mut item)),
                    &packet(rows),
                    2
                )
                .is_err()
            );
            assert!(item.is_all_unknown());
        }
    }
}
