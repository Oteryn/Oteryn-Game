//! Separate current-source cohort: Crystal201189f cake_golem.lua:1-108 and
//! monsters.hpp:244; historical creature/NPC cohorts retain their exact pins.
use super::*;

include!("../../../../tools/content-schema/native-gameplay/cake_creature_admission_pin.rs");
const PACKET: &[u8] =
    include_bytes!("../../../../tools/content-schema/native-gameplay/cake-creature-admission.json");
const CAPTURE: &[u8] =
    include_bytes!("../../../../tools/content-migration/cake_creature_admission.py");
const STAGE: &[u8] =
    include_bytes!("../../../../tools/content-migration/creature_admission_stage.py");
const REVISION: &str = "crystalserver-creature-1530:201189f871d23597b82caf88fd611454a2ddefb8";
const PROJECT_REVISION: &str = "seasonal-cake-creature-20261107-r1";
const BATCH: &str = "seasonal-cake-creature-crystal-201189f-r1";
pub(super) const RECORD_DELTA: usize = SEASONAL_CAKE_RECORD_COUNT;
pub(super) const IMPORT_DELTA: usize = 1;

type Failure = Box<dyn std::error::Error>;

fn identity(record: &ProjectReferenceRecord) -> &DefinitionIdentityDocument {
    match record {
        ProjectReferenceRecord::Ability { identity, .. }
        | ProjectReferenceRecord::Effect { identity, .. }
        | ProjectReferenceRecord::Formula { identity, .. }
        | ProjectReferenceRecord::Item { identity, .. }
        | ProjectReferenceRecord::Generic { identity, .. }
        | ProjectReferenceRecord::Creature { identity, .. }
        | ProjectReferenceRecord::Loot { identity, .. }
        | ProjectReferenceRecord::LocalObject { identity, .. } => identity,
    }
}

pub(super) fn apply(draft: &mut ProjectV2Draft) -> Result<(), Failure> {
    if hex_sha256(PACKET) != SEASONAL_CAKE_PACKET_SHA256 {
        return Err("Cake creature packet digest drifted".into());
    }
    let packet: Value = serde_json::from_slice(PACKET)?;
    if packet["schema"] != "OTERYN_SEASONAL_CREATURE_ADMISSION/v1"
        || packet["crystal_commit"] != "201189f871d23597b82caf88fd611454a2ddefb8"
        || packet["qualification"] != "OTS_HYPOTHESIS_ONLY"
        || packet["source_blob"] != "d95b96f4ed3c5e47d9219a3112c6065ea5de1e2d"
        || packet["capture_tool_sha256"] != hex_sha256(CAPTURE)
        || packet["stage_tool_sha256"] != hex_sha256(STAGE)
    {
        return Err("Cake creature provenance drifted".into());
    }
    let records: Vec<ProjectReferenceRecord> =
        serde_json::from_value(packet["staged"]["records"].clone())?;
    let profiles: Vec<ProjectV2AuthoringProfile> =
        serde_json::from_value(packet["staged"]["authoring_profiles"].clone())?;
    let bindings: Vec<ProjectV2SourceIdentityBinding> =
        serde_json::from_value(packet["staged"]["source_identity_bindings"].clone())?;
    if records.len() != SEASONAL_CAKE_RECORD_COUNT
        || profiles.len() != SEASONAL_CAKE_PROFILE_COUNT
        || bindings.len() != 1
        || bindings[0].source_revision != REVISION
        || bindings[0].target.key != "oteryn:creature.cake_golem"
    {
        return Err("Cake creature cohort shape drifted".into());
    }
    let import = ImportBatch {
        batch_id: BATCH.to_owned(),
        source_repository: "zimbadev/crystalserver".to_owned(),
        source_revision: REVISION.to_owned(),
        source_artifact_sha256: SEASONAL_CAKE_PACKET_SHA256.to_owned(),
        access_disposition: "PENDING".to_owned(),
        source_generation_profile: "OTERYN_SEASONAL_CREATURE_ADMISSION/v1".to_owned(),
        importer: "OTERYN_SEASONAL_CREATURE_ADMISSION/v1".to_owned(),
        mapper: "creature_admission_stage.py".to_owned(),
        mapper_revision: "201189f871d23597b82caf88fd611454a2ddefb8".to_owned(),
        mapper_sha256: hex_sha256(STAGE),
        candidates: Vec::new(),
        reimport_states: Vec::new(),
    };
    let source = ProjectV2Source {
        key: "oteryn:source.crystalserver".to_owned(),
        import_batch_id: BATCH.to_owned(),
        revision: REVISION.to_owned(),
        sha256: SEASONAL_CAKE_PACKET_SHA256.to_owned(),
        evidence: ProjectV2EvidenceClass::OtsHypothesisOnly,
    };
    // Validate the entire cohort before writing any record: partial replays must refuse.
    let mut existing = 0;
    for expected in &records {
        let matches = draft
            .core
            .records
            .iter()
            .filter(|actual| identity(actual) == identity(expected))
            .collect::<Vec<_>>();
        if matches.len() > 1 || matches.first().is_some_and(|actual| *actual != expected) {
            return Err("Cake creature record conflicts with accepted content".into());
        }
        existing += matches.len();
    }
    for expected in &profiles {
        let matches = draft
            .state
            .authoring_profiles
            .iter()
            .filter(|actual| actual.target == expected.target)
            .collect::<Vec<_>>();
        if matches.len() > 1 || matches.first().is_some_and(|actual| *actual != expected) {
            return Err("Cake creature profile conflicts with accepted content".into());
        }
        existing += matches.len();
    }
    for expected in &bindings {
        let matches = draft
            .state
            .source_identity_bindings
            .iter()
            .filter(|actual| {
                actual.source_key == expected.source_key && actual.source_revision == REVISION
            })
            .collect::<Vec<_>>();
        if matches.len() > 1 || matches.first().is_some_and(|actual| *actual != expected) {
            return Err("Cake creature binding conflicts with accepted content".into());
        }
        existing += matches.len();
    }
    let imports = draft
        .core
        .imports
        .iter()
        .filter(|actual| actual.batch_id == BATCH)
        .collect::<Vec<_>>();
    let sources = draft
        .state
        .sources
        .iter()
        .filter(|actual| actual.key == source.key && actual.revision == REVISION)
        .collect::<Vec<_>>();
    if imports.len() > 1
        || imports.first().is_some_and(|actual| *actual != &import)
        || sources.len() > 1
        || sources.first().is_some_and(|actual| *actual != &source)
    {
        return Err("Cake creature source conflicts with accepted content".into());
    }
    existing += imports.len() + sources.len();
    if existing == records.len() + profiles.len() + bindings.len() + 2 {
        draft.core.project_revision = PROJECT_REVISION.to_owned();
        return Ok(());
    }
    if existing != 0 {
        return Err("Cake creature cohort is partially present".into());
    }
    for item in [12141, 12143, 2886] {
        let key = format!("oteryn:item.tibia.i{item}");
        if draft.core.records.iter().filter(|record| matches!(record, ProjectReferenceRecord::Item {identity, ..} if identity.key == key && identity.revision == "definition-r1" && identity.family == "Item")).count() != 1 {
            return Err("Cake creature requires accepted canonical Item dependencies".into());
        }
    }
    draft.core.project_revision = PROJECT_REVISION.to_owned();
    draft.core.records.extend(records);
    draft.state.authoring_profiles.extend(profiles);
    draft.state.source_identity_bindings.extend(bindings);
    draft.core.imports.push(import);
    draft.state.sources.push(source);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ProjectV2Draft {
        let records = [12141, 12143, 2886].into_iter().map(|id| {
            serde_json::from_value(serde_json::json!({
                "kind": "Item", "identity": {"family": "Item", "key": format!("oteryn:item.tibia.i{id}"), "revision": "definition-r1"},
                "client_projection": "ServerOnly", "materializable": true,
                "stack_class": "NonStackable"
            })).expect("canonical Item fixture")
        }).collect();
        ProjectV2Draft {
            core: ProjectDraft {
                project_revision: "base".to_owned(),
                package_key: String::new(),
                semantic_schema_version: String::new(),
                licensing_metadata: String::new(),
                world_id: String::new(),
                coordinate_frame: String::new(),
                records,
                imports: Vec::new(),
                metadata: Vec::new(),
            },
            state: Default::default(),
        }
    }

    #[test]
    fn w05d_cake_cohort_admits_exact_source_and_replays_without_aliases() -> Result<(), Failure> {
        let mut draft = fixture();
        apply(&mut draft)?;
        assert_eq!(draft.core.records.len(), 3 + RECORD_DELTA);
        assert_eq!(
            draft.state.authoring_profiles.len(),
            SEASONAL_CAKE_PROFILE_COUNT
        );
        assert_eq!(
            draft.state.source_identity_bindings[0].target.key,
            "oteryn:creature.cake_golem"
        );
        let snapshot = draft.clone();
        apply(&mut draft)?;
        assert_eq!(draft, snapshot);
        Ok(())
    }

    #[test]
    fn w05d_cake_cohort_refuses_partial_or_substituted_content_atomically() -> Result<(), Failure> {
        let mut admitted = fixture();
        apply(&mut admitted)?;
        for kind in 0..4 {
            let mut draft = admitted.clone();
            match kind {
                0 => {
                    draft.core.records.pop();
                }
                1 => {
                    draft.state.authoring_profiles.pop();
                }
                2 => {
                    draft.state.source_identity_bindings[0].target.key =
                        "oteryn:creature.alias".to_owned();
                }
                _ => {
                    draft.core.imports[0].source_revision = "substituted".to_owned();
                }
            }
            let before = draft.clone();
            assert!(apply(&mut draft).is_err());
            assert_eq!(draft, before);
        }
        let mut missing_dependency = fixture();
        missing_dependency.core.records.pop();
        let before = missing_dependency.clone();
        assert!(apply(&mut missing_dependency).is_err());
        assert_eq!(missing_dependency, before);
        Ok(())
    }
}
