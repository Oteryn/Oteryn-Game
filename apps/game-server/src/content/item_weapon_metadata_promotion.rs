//! Closed103 intrinsic source metadata. Native Item semantics and V4 are untouched.
use super::{
    CanonicalProjectDocuments, DefinitionIdentityDocument, ProjectEvidenceLimits,
    ProjectReferenceRecord, ProjectV2Draft, ProjectV2ExactRatio, ProjectV2ItemAuthoring,
    ReferenceItemField, ReferenceItemSemantics, ReferenceSignedPoints, ReferenceWeaponType,
    world_project_sha256,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
pub const ITEM_WEAPON_METADATA_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261002-item-weapon-metadata-promotion-v1.json"
);
pub const ITEM_WEAPON_METADATA_PACKET_SHA256: &str =
    "9986d17a9c023dfe1d7e0052935c1f2ef517332f819477d4946bf1e8a99febbf";
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
    attack_modifier: usize,
    absolute_hit_percent: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    target: DefinitionIdentityDocument,
    official_name: String,
    facts: Facts,
}
// This is a packet reader, not a second authoring owner.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Facts {
    weapon_attack_modifier_points: Option<ReferenceSignedPoints>,
    weapon_absolute_hit_chance_percent: Option<ProjectV2ExactRatio>,
}
fn validate_percent(value: ProjectV2ExactRatio) -> Result<(), String> {
    let (mut a, mut b) = (value.numerator.unsigned_abs(), value.denominator);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    if value.denominator == 0
        || a != 1
        || value.numerator < 0
        || u128::from(value.numerator.unsigned_abs()) > 100 * u128::from(value.denominator)
    {
        return Err(
            "absolute source hit percent must be canonical 0..100 percentage points".into(),
        );
    }
    Ok(())
}
fn merge<T: PartialEq + Copy>(old: &mut Option<T>, incoming: Option<T>) -> Result<usize, String> {
    match (old.as_ref(), incoming) {
        (Some(a), Some(b)) if *a != b => Err("existing intrinsic weapon metadata conflicts".into()),
        (None, Some(value)) => {
            *old = Some(value);
            Ok(1)
        }
        _ => Ok(0),
    }
}
pub fn apply_item_weapon_metadata_promotion_v1(
    draft: &mut ProjectV2Draft,
    limits: ProjectEvidenceLimits,
) -> Result<usize, String> {
    if world_project_sha256(ITEM_WEAPON_METADATA_PACKET) != ITEM_WEAPON_METADATA_PACKET_SHA256 {
        return Err("weapon metadata packet digest drift".into());
    }
    let mut candidate = draft.clone();
    let native = draft.core.records.iter().filter_map(|r| match r {
        ProjectReferenceRecord::Item {
            identity,
            semantics,
            ..
        } => Some((identity, semantics)),
        _ => None,
    });
    let changed = apply_rows(
        native,
        &mut candidate.state.item_authoring,
        ITEM_WEAPON_METADATA_PACKET,
        78,
        25,
    )?;
    CanonicalProjectDocuments::from_v2_draft(candidate.clone(), limits)
        .map_err(|e| e.to_string())?;
    // Only the existing source overlay may change after the entire candidate validates.
    draft.state.item_authoring = candidate.state.item_authoring;
    Ok(changed)
}
fn apply_rows<'a>(
    mut items: impl Iterator<Item = (&'a DefinitionIdentityDocument, &'a ReferenceItemSemantics)>,
    authors: &mut Vec<ProjectV2ItemAuthoring>,
    bytes: &[u8],
    attack_count: usize,
    absolute_count: usize,
) -> Result<usize, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.schema != "OTERYN_ITEM_WEAPON_METADATA_PROMOTION/v1"
        || packet.counts.fields != attack_count + absolute_count
        || packet.counts.items != attack_count + absolute_count
        || packet.counts.attack_modifier != attack_count
        || packet.counts.absolute_hit_percent != absolute_count
        || packet.promotions.len() != attack_count + absolute_count
        || packet
            .promotions
            .iter()
            .filter(|r| r.facts.weapon_attack_modifier_points.is_some())
            .count()
            != attack_count
        || packet
            .promotions
            .iter()
            .filter(|r| r.facts.weapon_absolute_hit_chance_percent.is_some())
            .count()
            != absolute_count
    {
        return Err("closed103 weapon metadata shape/count drift".into());
    }
    let native = items.try_fold(BTreeMap::new(), |mut map, (id, semantics)| {
        if map.insert(id.key.as_str(), (id, semantics)).is_some() {
            return Err("duplicate native Item identity".to_owned());
        }
        Ok(map)
    })?;
    let mut owner_indices = BTreeMap::new();
    for (i, owner) in authors.iter().enumerate() {
        if owner_indices.insert(owner.item.key.clone(), i).is_some() {
            return Err("duplicate Item authoring owner".into());
        }
    }
    let mut updated = authors.clone();
    let mut seen = BTreeSet::new();
    let mut changed = 0;
    for row in packet.promotions {
        let attack = row.facts.weapon_attack_modifier_points;
        let absolute = row.facts.weapon_absolute_hit_chance_percent;
        if !seen.insert(row.target.key.clone())
            || attack.is_some() == absolute.is_some()
            || row.target.family != "Item"
            || row.target.revision != "definition-r1"
            || row.official_name.is_empty()
            || row.official_name.trim() != row.official_name
            || row.official_name.chars().any(char::is_control)
        {
            return Err("duplicate/invalid weapon source row".into());
        }
        let (id, semantics) = native
            .get(row.target.key.as_str())
            .ok_or("source Item missing")?;
        if *id != &row.target {
            return Err("full native weapon source identity drift".into());
        }
        let expected = if attack.is_some() {
            ReferenceWeaponType::Distance
        } else {
            ReferenceWeaponType::Ammunition
        };
        if !matches!(&semantics.weapon, ReferenceItemField::Known(w) if w.weapon_type == ReferenceItemField::Known(expected))
        {
            return Err("native weapon type is not source-qualified".into());
        }
        match &semantics.presentation {
            ReferenceItemField::Unknown => {}
            ReferenceItemField::Known(p) => match &p.name {
                ReferenceItemField::Unknown => {}
                ReferenceItemField::Known(name)
                    if name.trim().to_lowercase() == row.official_name.to_lowercase() => {}
                _ => return Err("native/source weapon name conflict".into()),
            },
            _ => return Err("native presentation blocked".into()),
        }
        if let Some(value) = absolute {
            validate_percent(value)?;
        }
        let target = serde_json::to_value(&row.target).map_err(|e| e.to_string())?;
        let index = match owner_indices.get(&row.target.key) {
            Some(i) => *i,
            None => {
                updated.push(
                    serde_json::from_value(serde_json::json!({"item":target}))
                        .map_err(|e| e.to_string())?,
                );
                let i = updated.len() - 1;
                owner_indices.insert(row.target.key.clone(), i);
                i
            }
        };
        let owner = &mut updated[index];
        if serde_json::to_value(&owner.item).map_err(|e| e.to_string())? != target {
            return Err("existing full authoring target drift".into());
        }
        changed += merge(&mut owner.weapon_attack_modifier_points, attack)?;
        changed += merge(&mut owner.weapon_absolute_hit_chance_percent, absolute)?;
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
    fn native() -> ReferenceItemSemantics {
        ReferenceItemSemantics {
            weapon: serde_json::from_value(serde_json::json!({"state":"KNOWN","value":{"weapon_type":{"state":"KNOWN","value":"DISTANCE"},"attack":{"state":"UNKNOWN"},"defense":{"state":"UNKNOWN"},"extra_defense":{"state":"UNKNOWN"},"range":{"state":"UNKNOWN"},"hit_chance":{"state":"UNKNOWN"},"max_hit_chance":{"state":"UNKNOWN"},"ammunition":{"state":"UNKNOWN"},"elemental":{"state":"UNKNOWN"}}})).expect("weapon"),
            ..ReferenceItemSemantics::default()
        }
    }
    fn packet(rows: Vec<serde_json::Value>) -> Vec<u8> {
        let count = rows.len();
        serde_json::to_vec(&serde_json::json!({"schema":"OTERYN_ITEM_WEAPON_METADATA_PROMOTION/v1","counts":{"fields":count,"items":count,"attack_modifier":count,"absolute_hit_percent":0},"promotions":rows})).expect("packet")
    }
    fn row(key: &str, value: i32) -> serde_json::Value {
        serde_json::json!({"target":id(key),"official_name":"source name","facts":{"weapon_attack_modifier_points":value}})
    }
    #[test]
    fn source_merge_preserves_siblings_and_is_idempotent() {
        let id = id("a");
        let native = native();
        let mut authors = vec![serde_json::from_value(serde_json::json!({"item":id,"forge":{"classification":4,"max_tier":10},"required_magic_level":15,"use_observation":{"mana_cost":13}})).expect("owner")];
        let siblings = serde_json::to_value(&authors[0]).expect("siblings");
        let bytes = packet(vec![row("a", -7)]);
        assert_eq!(
            apply_rows(std::iter::once((&id, &native)), &mut authors, &bytes, 1, 0).expect("merge"),
            1
        );
        let mut after = serde_json::to_value(&authors[0]).expect("after");
        after
            .as_object_mut()
            .expect("object")
            .remove("weapon_attack_modifier_points");
        assert_eq!(after, siblings);
        let before = authors.clone();
        assert_eq!(
            apply_rows(std::iter::once((&id, &native)), &mut authors, &bytes, 1, 0)
                .expect("repeat"),
            0
        );
        assert_eq!(authors, before);
    }
    #[test]
    fn late_conflict_wrong_target_or_type_is_atomic() {
        let (a, b, native) = (id("a"), id("b"), native());
        for wrong in [
            row("b", 2),
            row("a", 1),
            serde_json::json!({"target":{"family":"Item","key":"b","revision":"wrong"},"official_name":"source name","facts":{"weapon_attack_modifier_points":1}}),
        ] {
            let mut authors = vec![
                serde_json::from_value(
                    serde_json::json!({"item":b,"weapon_attack_modifier_points":1}),
                )
                .expect("owner"),
            ];
            let before = authors.clone();
            assert!(
                apply_rows(
                    [(&a, &native), (&b, &native)].into_iter(),
                    &mut authors,
                    &packet(vec![row("a", 1), wrong]),
                    2,
                    0
                )
                .is_err()
            );
            assert_eq!(authors, before);
        }
        let mut authors = vec![];
        assert!(
            apply_rows(
                std::iter::once((&a, &ReferenceItemSemantics::default())),
                &mut authors,
                &packet(vec![row("a", 1)]),
                1,
                0
            )
            .is_err()
        );
        assert!(authors.is_empty());
    }
    #[test]
    fn malformed_signed_values_and_duplicate_owner_are_atomic() {
        let id = id("a");
        let native = native();
        for value in [serde_json::json!(true), serde_json::json!(2147483648_i64)] {
            let mut bad = row("a", 1);
            bad["facts"]["weapon_attack_modifier_points"] = value;
            let mut authors = vec![];
            assert!(
                apply_rows(
                    std::iter::once((&id, &native)),
                    &mut authors,
                    &packet(vec![bad]),
                    1,
                    0
                )
                .is_err()
            );
            assert!(authors.is_empty());
        }
        let owner: ProjectV2ItemAuthoring =
            serde_json::from_value(serde_json::json!({"item":id})).expect("owner");
        let mut authors = vec![owner.clone(), owner];
        let before = authors.clone();
        assert!(
            apply_rows(
                std::iter::once((&id, &native)),
                &mut authors,
                &packet(vec![row("a", 1)]),
                1,
                0
            )
            .is_err()
        );
        assert_eq!(authors, before);
    }
    #[test]
    fn percent_domain_canonicality_and_points_are_distinct() {
        for (n, d) in [(0, 1), (90, 1), (100, 1), (1, 2)] {
            assert!(
                validate_percent(ProjectV2ExactRatio {
                    numerator: n,
                    denominator: d
                })
                .is_ok()
            );
        }
        for (n, d) in [(-1, 1), (101, 1), (2, 2), (0, 2), (1, 0)] {
            assert!(
                validate_percent(ProjectV2ExactRatio {
                    numerator: n,
                    denominator: d
                })
                .is_err()
            );
        }
        let absolute = ProjectV2ExactRatio {
            numerator: 90,
            denominator: 1,
        };
        assert_ne!(
            absolute,
            ProjectV2ExactRatio {
                numerator: 9,
                denominator: 10
            }
        );
    }
}
