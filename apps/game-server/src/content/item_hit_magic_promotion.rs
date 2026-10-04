//! Explicit relative hit and Rune ML metadata in existing field owners.
use super::{
    DefinitionIdentityDocument, ProjectReferenceRecord, ProjectV2Draft, ProjectV2Family,
    ProjectV2ItemAuthoring, ReferenceItemField, ReferenceItemSemantics, ReferenceItemWeapon,
    ReferenceRationalPercent, world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
pub const ITEM_HIT_MAGIC_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261002-item-hit-magic-promotion-v1.json"
);
pub const ITEM_HIT_MAGIC_PACKET_SHA256: &str =
    "0fcfee82c2c6f1a2b2eceb77277b4ea400d8082d98a553171fdbd7d571bbe314";
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
    hit_chance: Option<ReferenceRationalPercent>,
    required_magic_level: Option<u16>,
}
impl Facts {
    fn count(&self) -> usize {
        usize::from(self.hit_chance.is_some()) + usize::from(self.required_magic_level.is_some())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    target: DefinitionIdentityDocument,
    expected_name: String,
    hit_percentage_points: Option<i64>,
    facts: Facts,
}

pub fn apply_item_hit_magic_promotion_v1(draft: &mut ProjectV2Draft) -> Result<usize, String> {
    if world_project_sha256(ITEM_HIT_MAGIC_PACKET) != ITEM_HIT_MAGIC_PACKET_SHA256 {
        return Err("hit/ML packet digest drift".into());
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
    apply_rows(
        items,
        &mut draft.state.item_authoring,
        ITEM_HIT_MAGIC_PACKET,
        67,
        67,
    )
}
fn prepare(
    semantics: &ReferenceItemSemantics,
    row: &Row,
    owners: &mut Vec<ProjectV2ItemAuthoring>,
) -> Result<ReferenceItemSemantics, String> {
    use ReferenceItemField::{Known, Unknown};
    let mut next = semantics.clone();
    let existing = owners.iter().position(|o| o.item.key == row.target.key);
    if existing.is_some_and(|i| {
        owners[i].item.revision != row.target.revision
            || owners[i].item.family != ProjectV2Family::Item
    }) {
        return Err("hit/ML full authoring identity drift".into());
    }
    let absent_name: ReferenceItemField<String> = Unknown;
    let name = match &next.presentation {
        Known(group) => &group.name,
        Unknown => &absent_name,
        _ => return Err("hit/ML presentation group blocked".into()),
    };
    if row.expected_name.is_empty()
        || !matches!(name, Unknown | Known(_))
        || matches!(name, Known(value) if !value.eq_ignore_ascii_case(&row.expected_name))
    {
        return Err("normal native name guard".into());
    }
    if let Some(value) = row.facts.hit_chance {
        value.validate().map_err(|e| e.to_string())?;
        let points = row
            .hit_percentage_points
            .ok_or("missing explicit hit source unit")?;
        if i128::from(value.numerator) * 100 != i128::from(value.denominator) * i128::from(points) {
            return Err("hit modifier requires relative p/100 units".into());
        }
        if matches!(next.weapon, Unknown) {
            next.weapon = Known(ReferenceItemWeapon {
                weapon_type: Unknown,
                attack: Unknown,
                defense: Unknown,
                extra_defense: Unknown,
                range: Unknown,
                hit_chance: Unknown,
                max_hit_chance: Unknown,
                ammunition: Unknown,
                elemental: Unknown,
            });
        }
        let Known(weapon) = &mut next.weapon else {
            return Err("hit weapon group blocked".into());
        };
        if !matches!(&weapon.hit_chance, Unknown) && weapon.hit_chance != Known(value) {
            return Err("hit field blocked/conflict".into());
        }
        weapon.hit_chance = Known(value);
    }
    if row.facts.hit_chance.is_none() && row.hit_percentage_points.is_some() {
        return Err("unexpected hit source unit".into());
    }
    if let Some(value) = row.facts.required_magic_level {
        if let Some(i) = existing {
            if owners[i]
                .required_magic_level
                .is_some_and(|old| old != value)
            {
                return Err("authored magic level conflict".into());
            }
            owners[i].required_magic_level = Some(value);
        } else {
            owners.push(
                serde_json::from_value(
                    serde_json::json!({"item":row.target,"required_magic_level":value}),
                )
                .map_err(|e| e.to_string())?,
            );
        }
    }
    Ok(next)
}
fn apply_rows<'a>(
    mut items: impl Iterator<
        Item = (
            &'a DefinitionIdentityDocument,
            &'a mut ReferenceItemSemantics,
        ),
    >,
    owners: &mut Vec<ProjectV2ItemAuthoring>,
    bytes: &[u8],
    fields: usize,
    item_count: usize,
) -> Result<usize, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.schema != "OTERYN_ITEM_HIT_MAGIC_PROMOTION/v1"
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
        return Err("hit/ML packet scope drift".into());
    }
    let mut indexed = items.try_fold(BTreeMap::new(), |mut map, (id, semantics)| {
        if map.insert(id.key.as_str(), (id, semantics)).is_some() {
            return Err("duplicate native identity".to_owned());
        }
        Ok(map)
    })?;
    let mut owner_keys = BTreeSet::new();
    if owners.iter().any(|o| !owner_keys.insert(&o.item.key)) {
        return Err("duplicate authoring reference".into());
    }
    let mut planned_owners = owners.clone();
    let mut planned = BTreeMap::new();
    for row in &packet.promotions {
        let (id, semantics) = indexed
            .get(row.target.key.as_str())
            .ok_or("hit/ML Item missing")?;
        if *id != &row.target
            || row.target.family != "Item"
            || row.target.revision != "definition-r1"
            || row.facts.count() == 0
            || planned.contains_key(&row.target.key)
        {
            return Err("hit/ML duplicate/full target drift".into());
        }
        planned.insert(
            row.target.key.clone(),
            prepare(semantics, row, &mut planned_owners)?,
        );
    }
    // Commit only after every native field and authored ML precondition has passed.
    let mut changed = planned_owners
        .iter()
        .filter(|next| {
            owners
                .iter()
                .find(|old| old.item == next.item)
                .and_then(|old| old.required_magic_level)
                != next.required_magic_level
        })
        .count();
    for (key, semantics) in planned {
        let (_, current) = indexed
            .get_mut(key.as_str())
            .ok_or("prepared Item missing")?;
        changed += usize::from(current.weapon != semantics.weapon);
        **current = semantics;
    }
    *owners = planned_owners;
    Ok(changed)
}
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ReferenceItemField::{Conflict, Known, NotApplicable, Unknown};
    fn target() -> DefinitionIdentityDocument {
        DefinitionIdentityDocument {
            family: "Item".into(),
            key: "item".into(),
            revision: "definition-r1".into(),
        }
    }
    fn packet(facts: serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"schema":"OTERYN_ITEM_HIT_MAGIC_PROMOTION/v1","counts":{"fields":facts.as_object().expect("facts").len(),"items":1},"promotions":[{"target":target(),"expected_name":"bow","hit_percentage_points":facts.get("hit_chance").map(|_|6),"facts":facts}]})).expect("packet")
    }
    fn original() -> ReferenceItemSemantics {
        serde_json::from_value(serde_json::json!({"presentation":{"state":"KNOWN","value":{"name":{"state":"KNOWN","value":"bow"},"description":{"state":"CONFLICT"}}}})).expect("semantics")
    }
    #[test]
    fn relative_hit_zero_magic_idempotence_and_siblings() {
        let id = target();
        let mut item = original();
        let mut owners = vec![];
        let bytes = packet(
            serde_json::json!({"hit_chance":{"numerator":3,"denominator":50},"required_magic_level":0}),
        );
        assert_eq!(
            apply_rows(std::iter::once((&id, &mut item)), &mut owners, &bytes, 2, 1)
                .expect("apply"),
            2
        );
        let before = item.clone();
        let before_owners = owners.clone();
        assert_eq!(
            apply_rows(std::iter::once((&id, &mut item)), &mut owners, &bytes, 2, 1)
                .expect("repeat"),
            0
        );
        assert_eq!(item, before);
        assert_eq!(owners, before_owners);
        assert_eq!(owners[0].required_magic_level, Some(0));
        assert!(matches!(item.presentation,Known(ref p) if p.description==Conflict));
        assert!(
            matches!(item.weapon,Known(ref w) if w.max_hit_chance==Unknown && w.attack==Unknown && w.hit_chance==Known(ReferenceRationalPercent{numerator:3,denominator:50}))
        );
    }
    #[test]
    fn names_units_blocked_states_and_authoring_conflicts_are_atomic() {
        let id = target();
        for facts in [
            serde_json::json!({"name":"bow"}),
            serde_json::json!({"hit_chance":{"numerator":6,"denominator":1}}),
            serde_json::json!({"hit_chance":{"numerator":6,"denominator":100}}),
            serde_json::json!({"required_magic_level":65536}),
        ] {
            let mut item = original();
            let before = item.clone();
            let mut owners = vec![];
            assert!(
                apply_rows(
                    std::iter::once((&id, &mut item)),
                    &mut owners,
                    &packet(facts),
                    1,
                    1
                )
                .is_err()
            );
            assert_eq!(item, before);
            assert!(owners.is_empty());
        }
        for state in [Conflict, NotApplicable] {
            let mut item = ReferenceItemSemantics {
                weapon: state,
                ..original()
            };
            let before = item.clone();
            let mut owners = vec![];
            assert!(
                apply_rows(
                    std::iter::once((&id, &mut item)),
                    &mut owners,
                    &packet(serde_json::json!({"hit_chance":{"numerator":3,"denominator":50}})),
                    1,
                    1
                )
                .is_err()
            );
            assert_eq!(item, before);
        }
        for name in [Conflict, NotApplicable, Known("weapon of carving".into())] {
            let mut item = original();
            if let Known(p) = &mut item.presentation {
                p.name = name;
            }
            let before = item.clone();
            let mut owners = vec![];
            assert!(
                apply_rows(
                    std::iter::once((&id, &mut item)),
                    &mut owners,
                    &packet(serde_json::json!({"required_magic_level":0})),
                    1,
                    1
                )
                .is_err()
            );
            assert_eq!(item, before);
            assert!(owners.is_empty());
        }
        let mut item = original();
        let mut owners: Vec<ProjectV2ItemAuthoring> = vec![
            serde_json::from_value(serde_json::json!({"item":id,"required_magic_level":1}))
                .expect("owner"),
        ];
        let before = item.clone();
        let before_owners = owners.clone();
        assert!(
            apply_rows(
                std::iter::once((&id, &mut item)),
                &mut owners,
                &packet(serde_json::json!({"required_magic_level":0})),
                1,
                1
            )
            .is_err()
        );
        assert_eq!(item, before);
        assert_eq!(owners, before_owners);
    }
    #[test]
    fn later_bad_full_target_never_commits_prepared_native_or_owner() {
        let id = target();
        let mut item = original();
        let before = item.clone();
        let mut owners = vec![];
        let bytes = packet(
            serde_json::json!({"hit_chance":{"numerator":3,"denominator":50},"required_magic_level":0}),
        );
        for wrong in ["missing", "wrong-revision", "duplicate"] {
            let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("packet");
            let mut second = value["promotions"][0].clone();
            if wrong == "missing" {
                second["target"]["key"] = "missing".into();
            } else if wrong == "wrong-revision" {
                second["target"]["revision"] = "other".into();
            }
            value["promotions"]
                .as_array_mut()
                .expect("rows")
                .push(second);
            value["counts"] = serde_json::json!({"fields":4,"items":2});
            assert!(
                apply_rows(
                    std::iter::once((&id, &mut item)),
                    &mut owners,
                    &serde_json::to_vec(&value).expect("bytes"),
                    4,
                    2
                )
                .is_err()
            );
            assert_eq!(item, before);
            assert!(owners.is_empty());
        }
    }
}
