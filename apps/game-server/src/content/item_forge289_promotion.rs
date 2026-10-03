//! Closed source-data import and literal Forge owner pairs; Native records are immutable.
use super::{
    CanonicalProjectDocuments, ImportBatch, ProjectEvidenceLimits, ProjectReferenceRecord,
    ProjectV2Draft, ProjectV2Family, ProjectV2ItemAuthoring, ProjectV2Source,
    ProjectV2SourceIdentityBinding, ProjectV2SourceIdentityDisposition, world_project_sha256,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const ITEM_FORGE289_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261002-item-forge289-promotion-v1.json"
);
pub const ITEM_FORGE289_PACKET_SHA256: &str =
    "18db9c0067f2e1a9a7091fdf3fd38c1cf56ce81b7807a239f745dd4aea962682";
#[derive(Deserialize)]
struct Packet {
    schema: String,
    source: ProjectV2Source,
    import_batch: ImportBatch,
    bindings: Vec<ProjectV2SourceIdentityBinding>,
    promotions: Vec<ProjectV2ItemAuthoring>,
    native_guards: Vec<NativeGuard>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeGuard {
    target: super::ProjectV2DefinitionRef,
    official_name: String,
    headers: Value,
    weapon_type: Value,
    stats: Vec<NativeStat>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeStat {
    group: String,
    property: String,
    value: i32,
}
pub fn apply_item_forge289_promotion_v1(
    draft: &mut ProjectV2Draft,
    limits: ProjectEvidenceLimits,
) -> Result<usize, String> {
    if world_project_sha256(ITEM_FORGE289_PACKET) != ITEM_FORGE289_PACKET_SHA256 {
        return Err("Forge289 packet digest drift".into());
    }
    let mut candidate = draft.clone();
    let changed = apply(&mut candidate, ITEM_FORGE289_PACKET)?;
    CanonicalProjectDocuments::from_v2_draft(candidate.clone(), limits)
        .map_err(|e| e.to_string())?;
    *draft = candidate;
    Ok(changed)
}
fn apply(draft: &mut ProjectV2Draft, bytes: &[u8]) -> Result<usize, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.schema != "OTERYN_ITEM_FORGE289_SOURCE_IMPORT/v1"
        || packet.promotions.len() != 289
        || packet.bindings.len() != 289
        || packet.native_guards.len() != 289
        || packet.source.key != "oteryn:source.tibiawiki"
        || packet.source.revision
            != format!("tibiawiki-br-forge289-snapshot:{}", packet.source.sha256)
        || packet.source.import_batch_id != "g4-item-forge289-br-r1"
        || packet.import_batch.batch_id != packet.source.import_batch_id
        || packet.import_batch.source_revision != packet.source.revision
        || packet.import_batch.source_artifact_sha256 != packet.source.sha256
        || !packet.import_batch.candidates.is_empty()
        || !packet.import_batch.reimport_states.is_empty()
    {
        return Err("Forge289 closed source-data shape drift".into());
    }
    let mut native = BTreeMap::new();
    for record in &draft.core.records {
        if let ProjectReferenceRecord::Item { identity, .. } = record
            && native.insert(identity.key.as_str(), record).is_some()
        {
            return Err("duplicate current native Item".into());
        }
    }
    let mut owners = draft.state.item_authoring.clone();
    let mut owner_indices = BTreeMap::new();
    for (i, owner) in owners.iter().enumerate() {
        if owner_indices.insert(owner.item.key.clone(), i).is_some() {
            return Err("duplicate current source owner".into());
        }
    }
    let mut seen = BTreeSet::new();
    let mut pages = BTreeSet::new();
    let mut changed = 0;
    for ((row, binding), guard) in packet
        .promotions
        .iter()
        .zip(&packet.bindings)
        .zip(&packet.native_guards)
    {
        let forge = row.forge.ok_or("explicit Forge pair absent")?;
        if row.item.family != ProjectV2Family::Item
            || row.item.revision != "definition-r1"
            || !seen.insert(row.item.key.clone())
            || !pages.insert(binding.external_id.clone())
            || !(1..=4).contains(&forge.classification)
            || forge.max_tier == 0
            || serde_json::to_value(row).map_err(|e| e.to_string())?
                != serde_json::json!({"item":row.item,"forge":forge})
            || binding.source_key != packet.source.key
            || binding.source_revision != packet.source.revision
            || binding.identity_namespace != "mediawiki/page_id"
            || binding.target != row.item
            || binding.disposition != ProjectV2SourceIdentityDisposition::Exact
            || guard.target != row.item
            || guard.official_name.is_empty()
            || guard.official_name.trim() != guard.official_name
        {
            return Err("Forge289 invalid pair/fulltarget/binding/guard".into());
        }
        let matching: Vec<_> = draft
            .state
            .source_identity_bindings
            .iter()
            .filter(|b| {
                b.source_key == binding.source_key
                    && (b.target == binding.target || b.external_id == binding.external_id)
            })
            .collect();
        if !matching.is_empty() && matching != vec![binding] {
            return Err("Forge289 existing source identity opposition/duplicate".into());
        }
        let record = native
            .get(row.item.key.as_str())
            .ok_or("Forge289 unique current Item missing")?;
        let value = serde_json::to_value(record).map_err(|e| e.to_string())?;
        let headers = serde_json::json!({"identity":value["identity"],"kind":value["kind"],"stack_class":value["stack_class"],"materializable":value["materializable"],"client_projection":value["client_projection"]});
        if headers != guard.headers
            || value["semantics"]["presentation"]["state"] != "KNOWN"
            || value["semantics"]["presentation"]["value"]["name"]
                != serde_json::json!({"state":"KNOWN","value":guard.official_name})
            || value["semantics"]["weapon"]["value"]["weapon_type"] != guard.weapon_type
        {
            return Err("Forge289 current scoped native headers/name/type drift".into());
        }
        let mut stats = BTreeSet::new();
        for stat in &guard.stats {
            if !matches!(
                (stat.group.as_str(), stat.property.as_str()),
                ("weapon", "attack" | "defense" | "extra_defense" | "range")
                    | ("protection", "armor")
            ) || !stats.insert((&stat.group, &stat.property))
                || value["semantics"][&stat.group]["state"] != "KNOWN"
                || value["semantics"][&stat.group]["value"][&stat.property]
                    != serde_json::json!({"state":"KNOWN","value":stat.value})
            {
                return Err("Forge289 current typed non-name stat drift".into());
            }
        }
        if stats.len() < 2 {
            return Err("Forge289 requires two distinct typed source comparisons".into());
        }
        if let Some(&i) = owner_indices.get(&row.item.key) {
            if owners[i].item != row.item || owners[i].forge.is_some_and(|v| v != forge) {
                return Err("Forge289 existing owner identity/value conflict".into());
            }
            if owners[i].forge.is_none() {
                owners[i].forge = Some(forge);
                changed += 2;
            }
        } else {
            owners.push(row.clone());
            changed += 2;
        }
    }
    let existing_sources: Vec<_> = draft
        .state
        .sources
        .iter()
        .filter(|s| s.key == packet.source.key && s.revision == packet.source.revision)
        .collect();
    let existing_imports: Vec<_> = draft
        .core
        .imports
        .iter()
        .filter(|b| b.batch_id == packet.import_batch.batch_id)
        .collect();
    if (!existing_sources.is_empty() && existing_sources != vec![&packet.source])
        || (!existing_imports.is_empty() && existing_imports != vec![&packet.import_batch])
    {
        return Err("Forge289 existing source/import metadata conflict".into());
    }
    let new_source = existing_sources.is_empty();
    let new_import = existing_imports.is_empty();
    let new_bindings: Vec<_> = packet
        .bindings
        .iter()
        .filter(|b| !draft.state.source_identity_bindings.contains(b))
        .cloned()
        .collect();
    // Every identity, source and owner precondition passed. No Native record write.
    draft.state.item_authoring = owners;
    draft.state.source_identity_bindings.extend(new_bindings);
    if new_source {
        draft.state.sources.push(packet.source);
    }
    if new_import {
        draft.core.imports.push(packet.import_batch);
    }
    Ok(changed)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn draft() -> Result<ProjectV2Draft, Box<dyn std::error::Error>> {
        let proof: Value = serde_json::from_str(include_str!(
            "../../../../docs/agents/evidence/OTV2-20261002-item-forge289-source-qualification-v1.json"
        ))?;
        let records = proof["records"]
            .as_array()
            .ok_or("source rows")?
            .iter()
            .map(|r| serde_json::from_value(r["qualification"]["native_definition"].clone()))
            .collect::<Result<_, _>>()?;
        Ok(ProjectV2Draft {
            core: super::super::ProjectDraft {
                project_revision: "source289-test-r1".into(),
                package_key: "oteryn:content.world-project".into(),
                semantic_schema_version: "reference-schema-v1".into(),
                licensing_metadata: "PENDING".into(),
                world_id: "0123456789ab70cd8ef0123456789abc".into(),
                coordinate_frame: "global-target-2026-09-27".into(),
                records,
                imports: vec![],
                metadata: vec![],
            },
            state: super::super::ProjectV2State {
                item_authoring: serde_json::from_value(proof["current_parent_authoring"].clone())?,
                ..super::super::ProjectV2State::default()
            },
        })
    }
    #[test]
    fn full289_atomic_source_data_idempotence_preserves_native_and_siblings()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut d = draft()?;
        let rows: Packet = serde_json::from_slice(ITEM_FORGE289_PACKET)?;
        let mut sibling = rows.promotions[0].clone();
        sibling.forge = None;
        sibling.required_magic_level = Some(7);
        d.state.item_authoring.push(sibling);
        let native = d.core.records.clone();
        let old = d.state.item_authoring[..411].to_vec();
        assert_eq!(apply(&mut d, ITEM_FORGE289_PACKET)?, 578);
        assert_eq!(d.core.records, native);
        assert_eq!(d.state.item_authoring.len(), 700);
        assert_eq!(d.state.item_authoring[..411], old);
        assert_eq!(d.state.item_authoring[411].required_magic_level, Some(7));
        assert_eq!(d.state.source_identity_bindings.len(), 289);
        assert_eq!(d.state.sources.len(), 1);
        assert_eq!(d.core.imports.len(), 1);
        assert!(d.core.imports[0].candidates.is_empty());
        let before = d.clone();
        assert_eq!(apply(&mut d, ITEM_FORGE289_PACKET)?, 0);
        assert_eq!(d, before);
        Ok(())
    }
    #[test]
    fn late_owner_native_or_binding_conflicts_and_malformed_values_are_atomic()
    -> Result<(), Box<dyn std::error::Error>> {
        let packet: Value = serde_json::from_slice(ITEM_FORGE289_PACKET)?;
        for mode in 0..8 {
            let mut d = draft()?;
            let mut raw = packet.clone();
            match mode {
                0 => {
                    let mut row: ProjectV2ItemAuthoring =
                        serde_json::from_value(raw["promotions"][288].clone())?;
                    row.forge.as_mut().ok_or("forge")?.max_tier += 1;
                    d.state.item_authoring.push(row);
                }
                1 => {
                    let mut b: ProjectV2SourceIdentityBinding =
                        serde_json::from_value(raw["bindings"][288].clone())?;
                    b.target.revision = "other".into();
                    d.state.source_identity_bindings.push(b);
                }
                2 => {
                    if let ProjectReferenceRecord::Item { semantics, .. } = &mut d.core.records[288]
                    {
                        semantics.presentation = super::super::ReferenceItemField::Unknown;
                    }
                }
                3 => raw["promotions"][288]["forge"]["classification"] = serde_json::json!(true),
                4 => raw["promotions"][288]["forge"]["max_tier"] = serde_json::json!(256),
                5 => raw["promotions"][288]["required_magic_level"] = serde_json::json!(7),
                6 => raw["native_guards"][288]["stats"] = serde_json::json!([]),
                _ => {
                    let mut b: ProjectV2SourceIdentityBinding =
                        serde_json::from_value(raw["bindings"][288].clone())?;
                    b.target = serde_json::from_value(raw["promotions"][0]["item"].clone())?;
                    d.state.source_identity_bindings.push(b);
                }
            }
            let before = d.clone();
            assert!(
                apply(&mut d, &serde_json::to_vec(&raw)?).is_err(),
                "mode{mode}"
            );
            assert_eq!(d, before, "mode{mode}");
        }
        Ok(())
    }
    #[test]
    fn unrelated_native_stack_and_description_do_not_become_forge_guards()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut d = draft()?;
        if let ProjectReferenceRecord::Item { semantics, .. } = &mut d.core.records[0] {
            semantics.stack = super::super::ReferenceItemField::Unknown;
            if let super::super::ReferenceItemField::Known(p) = &mut semantics.presentation {
                p.description =
                    super::super::ReferenceItemField::Known("independent document source".into());
            }
        }
        let records = d.core.records.clone();
        assert_eq!(apply(&mut d, ITEM_FORGE289_PACKET)?, 578);
        assert_eq!(d.core.records, records);
        Ok(())
    }
}
