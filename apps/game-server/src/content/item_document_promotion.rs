//! Independent explicit document facts; siblings and admission remain unchanged.
use super::{
    DefinitionIdentityDocument, ProjectReferenceRecord, ProjectV2Draft, ReferenceItemField,
    ReferenceItemReadableWriteable, ReferenceItemSemantics, world_project_sha256,
};
use serde::Deserialize;
use std::collections::BTreeMap;
pub const ITEM_DOCUMENT_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-item-document-promotion-v1.json"
);
pub const ITEM_DOCUMENT_PACKET_SHA256: &str =
    "d83101b5cb2231b55ed331960c35e57f2ac58e9db9e76ef632db8c02f61e33ba";
#[derive(Deserialize)]
struct Packet {
    schema: String,
    counts: Counts,
    promotions: Vec<Row>,
}
#[derive(Deserialize)]
struct Counts {
    fields: usize,
    items: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Facts {
    readable: Option<bool>,
    writeable: Option<bool>,
    max_text_length: Option<u32>,
}
impl Facts {
    fn count(&self) -> usize {
        usize::from(self.readable.is_some())
            + usize::from(self.writeable.is_some())
            + usize::from(self.max_text_length.is_some())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    target: DefinitionIdentityDocument,
    facts: Facts,
}

pub fn apply_item_document_promotion_v1(draft: &mut ProjectV2Draft) -> Result<usize, String> {
    if world_project_sha256(ITEM_DOCUMENT_PACKET) != ITEM_DOCUMENT_PACKET_SHA256 {
        return Err("document packet digest drift".into());
    }
    let items = draft
        .core
        .records
        .iter_mut()
        .filter_map(|record| match record {
            ProjectReferenceRecord::Item {
                identity,
                semantics,
                ..
            } => Some((&*identity, semantics)),
            _ => None,
        });
    apply_rows(items, ITEM_DOCUMENT_PACKET, 207, 97)
}
fn check<T: PartialEq>(old: &ReferenceItemField<T>, value: &T) -> Result<(), String> {
    match old {
        ReferenceItemField::Unknown => Ok(()),
        ReferenceItemField::Known(v) if v == value => Ok(()),
        _ => Err("document leaf blocked/conflict".into()),
    }
}
fn validate(facts: &Facts, semantics: &ReferenceItemSemantics) -> Result<(), String> {
    use ReferenceItemField::{Known, Unknown};
    let group = match &semantics.readable_writeable {
        Unknown => None,
        Known(group) => Some(group),
        _ => return Err("document group blocked".into()),
    };
    if let Some(value) = facts.readable {
        if !value {
            return Err("document readable must be affirmative".into());
        }
        check(&group.map_or(Unknown, |g| g.readable.clone()), &value)?;
    }
    if let Some(value) = facts.writeable {
        if value && group.is_some_and(|g| g.readable == Known(false)) {
            return Err("document readability coherence conflict".into());
        }
        check(&group.map_or(Unknown, |g| g.writeable.clone()), &value)?;
    }
    if let Some(value) = facts.max_text_length {
        if value == 0 {
            return Err("document length must be positive".into());
        }
        check(
            &group.map_or(Unknown, |g| g.max_text_length.clone()),
            &value,
        )?;
    }
    Ok(())
}
fn apply_rows<'a>(
    mut items: impl Iterator<
        Item = (
            &'a DefinitionIdentityDocument,
            &'a mut ReferenceItemSemantics,
        ),
    >,
    bytes: &[u8],
    fields: usize,
    item_count: usize,
) -> Result<usize, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.schema != "OTERYN_ITEM_DOCUMENT_PROMOTION/v1"
        || packet.counts.fields != fields
        || packet.counts.items != item_count
        || packet.promotions.len() != item_count
        || packet
            .promotions
            .iter()
            .map(|r| r.facts.count())
            .sum::<usize>()
            != fields
    {
        return Err("document packet count/shape".into());
    }
    let mut items = items.try_fold(BTreeMap::new(), |mut map, (identity, semantics)| {
        if map
            .insert(identity.key.as_str(), (identity, semantics))
            .is_some()
        {
            return Err("duplicate native document Item".to_owned());
        }
        Ok(map)
    })?;
    let mut seen = std::collections::BTreeSet::new();
    for row in &packet.promotions {
        if !seen.insert(&row.target.key)
            || row.facts.count() == 0
            || row.target.family != "Item"
            || row.target.revision != "definition-r1"
        {
            return Err("document duplicate/target".into());
        }
        let (identity, semantics) = items
            .get(row.target.key.as_str())
            .ok_or("document Item missing")?;
        if *identity != &row.target {
            return Err("document full native identity drift".into());
        }
        validate(&row.facts, semantics)?;
    }
    // Prevalidate the complete packet before any leaf mutation or group initialization.
    for row in packet.promotions {
        let (_, semantics) = items
            .get_mut(row.target.key.as_str())
            .ok_or("validated document Item missing")?;
        if matches!(semantics.readable_writeable, ReferenceItemField::Unknown) {
            semantics.readable_writeable =
                ReferenceItemField::Known(ReferenceItemReadableWriteable {
                    readable: ReferenceItemField::Unknown,
                    writeable: ReferenceItemField::Unknown,
                    distance_read: ReferenceItemField::Unknown,
                    max_text_length: ReferenceItemField::Unknown,
                    write_once_target: ReferenceItemField::Unknown,
                });
        }
        if let ReferenceItemField::Known(group) = &mut semantics.readable_writeable {
            if let Some(value) = row.facts.readable {
                group.readable = ReferenceItemField::Known(value);
            }
            if let Some(value) = row.facts.writeable {
                group.writeable = ReferenceItemField::Known(value);
            }
            if let Some(value) = row.facts.max_text_length {
                group.max_text_length = ReferenceItemField::Known(value);
            }
        }
    }
    Ok(fields)
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ReferenceItemField::{Conflict, Known, NotApplicable, Unknown};
    fn identity(key: &str) -> DefinitionIdentityDocument {
        DefinitionIdentityDocument {
            family: "Item".into(),
            key: key.into(),
            revision: "definition-r1".into(),
        }
    }
    fn row(key: &str, facts: serde_json::Value) -> serde_json::Value {
        serde_json::json!({"target":identity(key),"facts":facts})
    }
    fn packet(rows: Vec<serde_json::Value>, fields: usize) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"schema":"OTERYN_ITEM_DOCUMENT_PROMOTION/v1","counts":{"fields":fields,"items":rows.len()},"promotions":rows})).expect("packet")
    }
    #[test]
    fn false_write_length_and_idempotence_preserve_siblings() {
        let key = identity("item");
        let mut item = ReferenceItemSemantics::default();
        let bytes = packet(
            vec![row(
                "item",
                serde_json::json!({"writeable":false,"max_text_length":1023}),
            )],
            2,
        );
        apply_rows(std::iter::once((&key, &mut item)), &bytes, 2, 1).expect("first");
        let before = item.clone();
        apply_rows(std::iter::once((&key, &mut item)), &bytes, 2, 1).expect("repeat");
        assert_eq!(item, before);
        let group = match item.readable_writeable {
            Known(group) => Some(group),
            _ => None,
        }
        .expect("Known document group");
        assert_eq!(group.readable, Unknown);
        assert_eq!(group.writeable, Known(false));
        assert_eq!(group.max_text_length, Known(1023));
        assert_eq!(group.distance_read, Unknown);
        assert_eq!(group.write_once_target, Unknown);
    }
    #[test]
    fn group_and_leaf_conflicts_reject_atomically() {
        for blocked in [Conflict, NotApplicable] {
            let (a, b) = (identity("first"), identity("second"));
            let mut first = ReferenceItemSemantics::default();
            let mut second = ReferenceItemSemantics {
                readable_writeable: blocked,
                ..ReferenceItemSemantics::default()
            };
            let before = second.clone();
            let bytes = packet(
                vec![
                    row("first", serde_json::json!({"readable":true})),
                    row("second", serde_json::json!({"readable":true})),
                ],
                2,
            );
            assert!(
                apply_rows(
                    [(&a, &mut first), (&b, &mut second)].into_iter(),
                    &bytes,
                    2,
                    2
                )
                .is_err()
            );
            assert_eq!(first, ReferenceItemSemantics::default());
            assert_eq!(second, before);
        }
        for old in [Conflict, NotApplicable, Known(false)] {
            let key = identity("item");
            let mut item = ReferenceItemSemantics {
                readable_writeable: Known(ReferenceItemReadableWriteable {
                    readable: old,
                    writeable: Unknown,
                    distance_read: Conflict,
                    max_text_length: Unknown,
                    write_once_target: NotApplicable,
                }),
                ..ReferenceItemSemantics::default()
            };
            let before = item.clone();
            assert!(
                apply_rows(
                    std::iter::once((&key, &mut item)),
                    &packet(vec![row("item", serde_json::json!({"readable":true}))], 1),
                    1,
                    1
                )
                .is_err()
            );
            assert_eq!(item, before);
        }
    }
    #[test]
    fn bad_values_duplicates_missing_and_revision_hold() {
        let mut key = identity("item");
        for facts in [
            serde_json::json!({"readable":false}),
            serde_json::json!({"max_text_length":0}),
            serde_json::json!({"writeable":5}),
            serde_json::json!({"max_text_length":4294967296u64}),
        ] {
            let mut item = ReferenceItemSemantics::default();
            assert!(
                apply_rows(
                    std::iter::once((&key, &mut item)),
                    &packet(vec![row("item", facts)], 1),
                    1,
                    1
                )
                .is_err()
            );
            assert_eq!(item, ReferenceItemSemantics::default());
        }
        for keys in [vec!["item", "item"], vec!["item", "missing"]] {
            let mut item = ReferenceItemSemantics::default();
            let rows = keys
                .iter()
                .map(|k| row(k, serde_json::json!({"readable":true})))
                .collect();
            assert!(
                apply_rows(std::iter::once((&key, &mut item)), &packet(rows, 2), 2, 2).is_err()
            );
            assert_eq!(item, ReferenceItemSemantics::default());
        }
        key.revision = "other".into();
        let mut item = ReferenceItemSemantics::default();
        assert!(
            apply_rows(
                std::iter::once((&key, &mut item)),
                &packet(vec![row("item", serde_json::json!({"readable":true}))], 1),
                1,
                1
            )
            .is_err()
        );
        assert_eq!(item, ReferenceItemSemantics::default());
    }
}
