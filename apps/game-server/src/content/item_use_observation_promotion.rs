//! Optional source observations only; native semantics and execution remain untouched.
use super::{
    DefinitionIdentityDocument, ProjectReferenceRecord, ProjectV2Draft, ProjectV2ItemAuthoring,
    ProjectV2ItemDamageObservation, ProjectV2ItemUseObservation, ReferenceItemField,
    ReferenceItemSemantics, world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
pub const ITEM_USE_OBSERVATION_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261002-item-use-observation-promotion-v1.json"
);
pub const ITEM_USE_OBSERVATION_PACKET_SHA256: &str =
    "990eca66d4d34156aaa7a2fd5c42251e26bdf503402f3d46d6beb6f7b2de99eb";
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
struct Row {
    target: DefinitionIdentityDocument,
    official_name: String,
    facts: ProjectV2ItemUseObservation,
}
fn count(facts: &ProjectV2ItemUseObservation) -> usize {
    usize::from(facts.damage.is_some())
        + usize::from(facts.damage_type.is_some())
        + usize::from(facts.mana_cost.is_some())
}
fn text(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.len() <= 4096
        && !value.chars().any(char::is_control)
}
fn merge<T: PartialEq + Clone>(old: &mut Option<T>, value: &Option<T>) -> Result<usize, String> {
    match (old.as_ref(), value.as_ref()) {
        (Some(a), Some(b)) if a != b => Err("source observation conflict".into()),
        (None, Some(value)) => {
            *old = Some(value.clone());
            Ok(1)
        }
        _ => Ok(0),
    }
}
pub fn apply_item_use_observation_promotion_v1(
    draft: &mut ProjectV2Draft,
) -> Result<usize, String> {
    if world_project_sha256(ITEM_USE_OBSERVATION_PACKET) != ITEM_USE_OBSERVATION_PACKET_SHA256 {
        return Err("source observation packet digest drift".into());
    }
    let items = draft.core.records.iter().filter_map(|r| match r {
        ProjectReferenceRecord::Item {
            identity,
            semantics,
            ..
        } => Some((identity, semantics)),
        _ => None,
    });
    apply_rows(
        items,
        &mut draft.state.item_authoring,
        ITEM_USE_OBSERVATION_PACKET,
        345,
        139,
    )
}
fn apply_rows<'a>(
    mut items: impl Iterator<Item = (&'a DefinitionIdentityDocument, &'a ReferenceItemSemantics)>,
    authors: &mut Vec<ProjectV2ItemAuthoring>,
    bytes: &[u8],
    fields: usize,
    item_count: usize,
) -> Result<usize, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.schema != "OTERYN_ITEM_USE_OBSERVATION_PROMOTION/v1"
        || packet.counts.fields != fields
        || packet.counts.items != item_count
        || packet.promotions.len() != item_count
        || packet
            .promotions
            .iter()
            .map(|r| count(&r.facts))
            .sum::<usize>()
            != fields
    {
        return Err("source observation packet shape/count".into());
    }
    let native = items.try_fold(BTreeMap::new(), |mut map, (id, semantics)| {
        if map.insert(id.key.as_str(), (id, semantics)).is_some() {
            return Err("duplicate native source-observation Item".to_owned());
        }
        Ok(map)
    })?;
    let mut owner_indices = BTreeMap::new();
    for (i, owner) in authors.iter().enumerate() {
        if owner_indices.insert(owner.item.key.clone(), i).is_some() {
            return Err("duplicate Item authoring owner".into());
        }
    }
    // Mutate only a private copy. Any later error leaves every original owner untouched.
    let mut updated = authors.clone();
    let mut seen = BTreeSet::new();
    let mut changed = 0;
    for row in packet.promotions {
        if !seen.insert(row.target.key.clone())
            || count(&row.facts) == 0
            || row.target.family != "Item"
            || row.target.revision != "definition-r1"
            || !text(&row.official_name)
        {
            return Err("source observation duplicate/target/empty row".into());
        }
        let (id, semantics) = native
            .get(row.target.key.as_str())
            .ok_or("source observation Item missing")?;
        if *id != &row.target {
            return Err("full native source-observation identity drift".into());
        }
        match &semantics.presentation {
            ReferenceItemField::Unknown => {}
            ReferenceItemField::Known(p) => match &p.name {
                ReferenceItemField::Unknown => {}
                ReferenceItemField::Known(name)
                    if name.trim().to_lowercase() == row.official_name.to_lowercase() => {}
                _ => return Err("source observation current native name conflict".into()),
            },
            _ => return Err("source observation presentation blocked".into()),
        }
        match &row.facts.damage {
            Some(ProjectV2ItemDamageObservation::Range { min, max }) if min > max => {
                return Err("inverted source damage range".into());
            }
            Some(ProjectV2ItemDamageObservation::Text(value)) if !text(value) => {
                return Err("invalid damage source text".into());
            }
            _ => {}
        }
        if row.facts.damage_type.as_ref().is_some_and(|v| !text(v)) {
            return Err("invalid damage-type source text".into());
        }
        let target = serde_json::to_value(&row.target).map_err(|e| e.to_string())?;
        let index = match owner_indices.get(&row.target.key) {
            Some(i) => *i,
            None => {
                let owner = serde_json::from_value(serde_json::json!({"item":target}))
                    .map_err(|e| e.to_string())?;
                updated.push(owner);
                let i = updated.len() - 1;
                owner_indices.insert(row.target.key.clone(), i);
                i
            }
        };
        let owner = &mut updated[index];
        if serde_json::to_value(&owner.item).map_err(|e| e.to_string())? != target {
            return Err("full existing authoring identity drift".into());
        }
        let observation = owner.use_observation.get_or_insert_with(Default::default);
        changed += merge(&mut observation.damage, &row.facts.damage)?;
        changed += merge(&mut observation.damage_type, &row.facts.damage_type)?;
        changed += merge(&mut observation.mana_cost, &row.facts.mana_cost)?;
    }
    *authors = updated;
    Ok(changed)
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    fn id(key: &str) -> DefinitionIdentityDocument {
        DefinitionIdentityDocument {
            family: "Item".into(),
            key: key.into(),
            revision: "definition-r1".into(),
        }
    }
    fn packet(rows: Vec<serde_json::Value>, fields: usize) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"schema":"OTERYN_ITEM_USE_OBSERVATION_PROMOTION/v1","counts":{"fields":fields,"items":rows.len()},"promotions":rows})).expect("packet")
    }
    fn row(key: &str, facts: serde_json::Value) -> serde_json::Value {
        serde_json::json!({"target":id(key),"official_name":"source name","facts":facts})
    }
    #[test]
    fn typed_observations_merge_preserve_siblings_and_are_idempotent() {
        let id = id("item");
        let native = ReferenceItemSemantics::default();
        let mut authors = vec![serde_json::from_value(serde_json::json!({"item":id,"required_magic_level":0,"use_observation":{"mana_cost":0}})).expect("owner")];
        let bytes = packet(
            vec![row(
                "item",
                serde_json::json!({"damage":{"kind":"Text","value":"13 (8-18)"},"damage_type":"Energy","mana_cost":0}),
            )],
            3,
        );
        assert_eq!(
            apply_rows(std::iter::once((&id, &native)), &mut authors, &bytes, 3, 1).expect("merge"),
            2
        );
        assert_eq!(authors[0].required_magic_level, Some(0));
        let before = authors.clone();
        assert_eq!(
            apply_rows(std::iter::once((&id, &native)), &mut authors, &bytes, 3, 1)
                .expect("repeat"),
            0
        );
        assert_eq!(authors, before);
    }
    #[test]
    fn later_conflict_and_wrong_owner_reject_without_partial_mutation() {
        let a = id("a");
        let b = id("b");
        let native = ReferenceItemSemantics::default();
        for owner in [
            serde_json::json!({"item":b,"use_observation":{"mana_cost":4}}),
            serde_json::json!({"item":{"family":"Item","key":"b","revision":"other"}}),
        ] {
            let mut authors = vec![serde_json::from_value(owner).expect("owner")];
            let before = authors.clone();
            let bytes = packet(
                vec![
                    row("a", serde_json::json!({"mana_cost":3})),
                    row("b", serde_json::json!({"mana_cost":3})),
                ],
                2,
            );
            assert!(
                apply_rows(
                    [(&a, &native), (&b, &native)].into_iter(),
                    &mut authors,
                    &bytes,
                    2,
                    2
                )
                .is_err()
            );
            assert_eq!(authors, before);
        }
    }
    #[test]
    fn invalid_range_duplicate_rows_and_targets_reject_atomically() {
        let a = id("a");
        let native = ReferenceItemSemantics::default();
        let mut authors = vec![];
        for facts in [
            serde_json::json!({"damage":{"kind":"Range","value":{"min":4,"max":3}}}),
            serde_json::json!({"damage_type":""}),
        ] {
            let bytes = packet(vec![row("a", facts)], 1);
            assert!(
                apply_rows(std::iter::once((&a, &native)), &mut authors, &bytes, 1, 1).is_err()
            );
            assert!(authors.is_empty());
        }
        let r = row("a", serde_json::json!({"mana_cost":0}));
        let bytes = packet(vec![r.clone(), r], 2);
        assert!(apply_rows(std::iter::once((&a, &native)), &mut authors, &bytes, 2, 2).is_err());
        assert!(authors.is_empty());
    }
    #[test]
    fn actual_packet_uses_existing_typed_observation_carrier() {
        let packet: Packet =
            serde_json::from_slice(ITEM_USE_OBSERVATION_PACKET).expect("typed packet");
        assert_eq!(packet.promotions.len(), 139);
        assert_eq!(
            packet
                .promotions
                .iter()
                .map(|r| count(&r.facts))
                .sum::<usize>(),
            345
        );
        assert!(packet.promotions.iter().any(|r| matches!(
            r.facts.damage,
            Some(ProjectV2ItemDamageObservation::Integer(_))
        )));
        assert!(packet.promotions.iter().any(|r| matches!(
            r.facts.damage,
            Some(ProjectV2ItemDamageObservation::Range { .. })
        )));
        assert!(packet.promotions.iter().any(|r| matches!(
            r.facts.damage,
            Some(ProjectV2ItemDamageObservation::Text(_))
        )));
    }
}
