//! Explicit13+4 whole-vector successor; old stats setters remain unchanged.
use super::{
    DefinitionIdentityDocument, ItemStackDocument, ProjectReferenceRecord, ProjectV2Draft,
    ProjectionDocument, ReferenceItemDefinition, ReferenceItemField, ReferenceItemPhysicalClass,
    ReferenceItemSemantics, ReferenceItemSkillModifiers, ReferenceItemStackClass,
    ReferenceModifierBinding, ReferenceModifierParameter, ReferenceSkillModifierKind,
    reference_playable::validate_item_definition, world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const ITEM_NUMERIC_MODIFIER_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-promotion-v1.json"
);
pub const ITEM_NUMERIC_MODIFIER_PACKET_SHA256: &str =
    "5d816ef227021d7e2ab3a347a4b2617fe86ac4383dbea6b9d9c12fec59713538";
const IDS: [u32; 17] = [
    36656, 36657, 36658, 36659, 36660, 36661, 36662, 36666, 36672, 36673, 39147, 39148, 39150,
    45639, 45640, 50169, 50170,
];

#[derive(Deserialize)]
struct Packet {
    schema: String,
    counts: Counts,
    promotions: Vec<Row>,
}
#[derive(Deserialize)]
struct Counts {
    items: usize,
    vectors: usize,
    atoms: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Headers {
    kind: String,
    stack_class: ItemStackDocument,
    client_projection: ProjectionDocument,
    materializable: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    source_item_id: u32,
    target: DefinitionIdentityDocument,
    expected_name: String,
    headers: Headers,
    modifiers: Vec<ReferenceModifierBinding>,
}

pub fn apply_item_numeric_modifier_promotion_v1(
    draft: &mut ProjectV2Draft,
) -> Result<usize, String> {
    if world_project_sha256(ITEM_NUMERIC_MODIFIER_PACKET) != ITEM_NUMERIC_MODIFIER_PACKET_SHA256 {
        return Err("numeric modifier packet digest drift".into());
    }
    let items = draft
        .core
        .records
        .iter_mut()
        .filter_map(|record| match record {
            ProjectReferenceRecord::Item {
                identity,
                client_projection,
                materializable,
                stack_class,
                semantics,
            } => Some((
                &*identity,
                Headers {
                    kind: "Item".into(),
                    stack_class: *stack_class,
                    client_projection: *client_projection,
                    materializable: *materializable,
                },
                semantics,
            )),
            _ => None,
        });
    apply_rows(items, ITEM_NUMERIC_MODIFIER_PACKET, &IDS, 50)
}

fn prepare(row: &Row, old: &ReferenceItemSemantics) -> Result<ReferenceItemSemantics, String> {
    use ReferenceItemField::{Known, Unknown};
    use ReferenceModifierParameter::{RationalPercent, SignedPoints};
    use ReferenceSkillModifierKind as K;
    if row.modifiers.is_empty()
        || row.expected_name.trim().is_empty()
        || !matches!(&old.presentation, Known(p) if matches!(&p.name, Known(n) if n.trim().eq_ignore_ascii_case(row.expected_name.trim())))
    {
        return Err("numeric modifier normal current name guard".into());
    }
    let mut new_kind = false;
    for entry in &row.modifiers {
        if entry.target_domain != Unknown
            || entry.evaluation_phase != Unknown
            || entry.priority != Unknown
            || !matches!(entry.parameter, Known(_))
        {
            return Err("numeric modifier context/parameter must remain explicit".into());
        }
        let parameter = match &entry.parameter {
            Known(p) => p,
            _ => return Err("missing parameter".into()),
        };
        match entry.kind {
            K::ReflectDamage | K::PerfectShotDamage | K::MagicShieldCapacityFlat => {
                new_kind = true;
                if !matches!(parameter,SignedPoints(v) if v.0 >= 0) {
                    return Err("numeric damage/flat signed-point unit".into());
                }
            }
            K::CleavePercent | K::MagicShieldCapacityPercent => {
                new_kind = true;
                if !matches!(parameter,RationalPercent(v) if v.denominator == 1 && (0..=100).contains(&v.numerator))
                {
                    return Err("numeric percent requires explicit percentage points".into());
                }
            }
            K::PerfectShotRange => {
                new_kind = true;
            }
            _ => {}
        }
    }
    if !new_kind {
        return Err("closed numeric vector has no qualified new parameter".into());
    }
    // Native37 validates closed enum order, uniqueness, capacity and each kind's exact typed parameter.
    let value = Known(ReferenceItemSkillModifiers {
        modifiers: Known(row.modifiers.clone()),
    });
    validate_item_definition(&ReferenceItemDefinition {
        physical_class: ReferenceItemPhysicalClass::Unknown,
        materializable: false,
        stack_class: ReferenceItemStackClass::Unknown,
        legal_destinations: vec![],
        semantics: ReferenceItemSemantics {
            skill_modifiers: value.clone(),
            ..Default::default()
        },
    })
    .map_err(|e| e.to_string())?;
    match &old.skill_modifiers {
        Unknown => {}
        Known(group)
            if group.modifiers == Unknown || group.modifiers == Known(row.modifiers.clone()) => {}
        _ => return Err("whole numeric modifier vector blocked/conflicting".into()),
    }
    let mut next = old.clone();
    next.skill_modifiers = value;
    Ok(next)
}

fn apply_rows<'a>(
    items: impl Iterator<
        Item = (
            &'a DefinitionIdentityDocument,
            Headers,
            &'a mut ReferenceItemSemantics,
        ),
    >,
    bytes: &[u8],
    ids: &[u32],
    atoms: usize,
) -> Result<usize, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let expected: BTreeSet<_> = ids.iter().copied().collect();
    if packet.schema != "OTERYN_ITEM_NUMERIC_MODIFIER_PROMOTION/v1"
        || expected.len() != ids.len()
        || packet.counts.items != ids.len()
        || packet.counts.vectors != ids.len()
        || packet.counts.atoms != atoms
        || packet.promotions.len() != ids.len()
        || packet
            .promotions
            .iter()
            .map(|r| r.modifiers.len())
            .sum::<usize>()
            != atoms
        || packet
            .promotions
            .iter()
            .map(|r| r.source_item_id)
            .collect::<BTreeSet<_>>()
            != expected
    {
        return Err("closed numeric17/50 source scope drift".into());
    }
    let mut native = BTreeMap::new();
    for (identity, headers, semantics) in items {
        if native
            .insert(identity.key.as_str(), (identity, headers, semantics))
            .is_some()
        {
            return Err("duplicate native Item identity".into());
        }
    }
    let mut pending = BTreeMap::new();
    for row in packet.promotions {
        let (identity, headers, semantics) = native
            .get(row.target.key.as_str())
            .ok_or("numeric Item missing")?;
        if *identity != &row.target
            || row.target.family != "Item"
            || row.target.revision != "definition-r1"
            || row.target.key != format!("oteryn:item.tibia.i{}", row.source_item_id)
            || &row.headers != headers
            || row.headers.kind != "Item"
            || pending.contains_key(&row.target.key)
        {
            return Err("numeric full target/structural headers drift".into());
        }
        pending.insert(row.target.key.clone(), prepare(&row, semantics)?);
    }
    // Commit only after every row has a fully prepared successor; siblings/owners never change.
    let mut changed = 0;
    for (key, next) in pending {
        let (_, _, old) = native
            .get_mut(key.as_str())
            .ok_or("validated numeric Item missing")?;
        if **old != next {
            **old = next;
            changed += 1;
        }
    }
    Ok(changed)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ReferenceItemField::{Conflict, Known, NotApplicable, Unknown};
    fn rows() -> serde_json::Value {
        let mut p: serde_json::Value =
            serde_json::from_slice(ITEM_NUMERIC_MODIFIER_PACKET).expect("source packet");
        p["promotions"] =
            serde_json::json!([p["promotions"][0].clone(), p["promotions"][1].clone()]);
        p["counts"] = serde_json::json!({"items":2,"vectors":2,"atoms":p["promotions"].as_array().expect("rows").iter().map(|r|r["modifiers"].as_array().expect("atoms").len()).sum::<usize>()});
        p
    }
    fn fixture(
        p: &serde_json::Value,
    ) -> Vec<(DefinitionIdentityDocument, Headers, ReferenceItemSemantics)> {
        p["promotions"].as_array().expect("rows").iter().map(|r| (
            serde_json::from_value(r["target"].clone()).expect("target"),
            serde_json::from_value(r["headers"].clone()).expect("headers"),
            serde_json::from_value(serde_json::json!({"presentation":{"state":"KNOWN","value":{"name":{"state":"KNOWN","value":r["expected_name"]},"description":{"state":"CONFLICT"}}},"temporal":{"state":"CONFLICT"}})).expect("native"),
        )).collect()
    }
    fn run(
        p: &serde_json::Value,
        items: &mut [(DefinitionIdentityDocument, Headers, ReferenceItemSemantics)],
    ) -> Result<usize, String> {
        let ids: Vec<_> = p["promotions"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|r| r["source_item_id"].as_u64().expect("id") as u32)
            .collect();
        apply_rows(
            items.iter_mut().map(|(i, h, s)| (&*i, h.clone(), s)),
            &serde_json::to_vec(p).expect("packet"),
            &ids,
            p["counts"]["atoms"].as_u64().expect("atoms") as usize,
        )
    }
    #[test]
    fn whole_vector_idempotence_and_siblings_are_preserved() {
        let p = rows();
        let mut items = fixture(&p);
        let before: Vec<_> = items.iter().map(|x| x.2.clone()).collect();
        let mut outside = fixture(
            &serde_json::from_slice(ITEM_NUMERIC_MODIFIER_PACKET).expect("complete source"),
        );
        let sibling = outside.remove(2);
        items.push(sibling.clone());
        assert_eq!(run(&p, &mut items).expect("first"), 2);
        let known = items.clone();
        assert_eq!(run(&p, &mut items).expect("repeat"), 0);
        assert_eq!(items, known);
        assert_eq!(items.pop().expect("outside scoped cohort"), sibling);
        for ((_, _, s), mut original) in items.into_iter().zip(before) {
            original.skill_modifiers = s.skill_modifiers.clone();
            assert_eq!(s, original);
        }
    }
    #[test]
    fn late_conflict_and_target_headers_roll_back_every_row() {
        let p = rows();
        for state in [
            Conflict,
            NotApplicable,
            Known(ReferenceItemSkillModifiers {
                modifiers: Conflict,
            }),
            Known(ReferenceItemSkillModifiers {
                modifiers: Known(vec![]),
            }),
        ] {
            let mut items = fixture(&p);
            items[1].2.skill_modifiers = state;
            let before = items.clone();
            assert!(run(&p, &mut items).is_err());
            assert_eq!(items, before);
        }
        let mut items = fixture(&p);
        items[1].0.revision = "wrong".into();
        let before = items.clone();
        assert!(run(&p, &mut items).is_err());
        assert_eq!(items, before);
        let mut items = fixture(&p);
        items[1].1.materializable = !items[1].1.materializable;
        let before = items.clone();
        assert!(run(&p, &mut items).is_err());
        assert_eq!(items, before);
        let mut items = fixture(&p);
        items.pop();
        let before = items.clone();
        assert!(run(&p, &mut items).is_err());
        assert_eq!(items, before);
    }
    #[test]
    fn wrong_typed_units_order_context_and_name_are_atomic() {
        for (field, value) in [
            (
                "parameter",
                serde_json::json!({"state":"KNOWN","value":{"kind":"ELEMENT","value":"ENERGY"}}),
            ),
            ("priority", serde_json::json!({"state":"KNOWN","value":1})),
            ("parameter", serde_json::json!({"state":"UNKNOWN"})),
        ] {
            let mut p = rows();
            p["promotions"][1]["modifiers"][0][field] = value;
            let mut items = fixture(&p);
            let before = items.clone();
            assert!(run(&p, &mut items).is_err());
            assert_eq!(items, before);
        }
        let mut p = rows();
        p["promotions"][1]["modifiers"][0]["parameter"]["value"]["value"] =
            serde_json::json!({"numerator":3,"denominator":100});
        let mut items = fixture(&p);
        let before = items.clone();
        assert!(run(&p, &mut items).is_err());
        assert_eq!(items, before);
        let p = rows();
        let mut items = fixture(&p);
        items[1].2.presentation = Unknown;
        let before = items.clone();
        assert!(run(&p, &mut items).is_err());
        assert_eq!(items, before);
        let mut p = rows();
        let mut items = fixture(&p);
        p["promotions"][1]["modifiers"]
            .as_array_mut()
            .expect("entries")
            .reverse();
        let before = items.clone();
        assert!(run(&p, &mut items).is_err());
        assert_eq!(items, before);
    }
}
