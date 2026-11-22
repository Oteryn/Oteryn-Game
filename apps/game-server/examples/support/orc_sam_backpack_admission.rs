//! Canonical Creature admission for the Venore raid's regular Orc.
use super::*;
use oteryn_game_server::content::ProjectV2AuthoringProfileData;
use serde::Deserialize;

const PACKET: &[u8] = include_bytes!(
    "../../../../tools/content-schema/native-gameplay/orc-sam-backpack-admission.json"
);
const SOURCE_REVISION: &str = "201189f871d23597b82caf88fd611454a2ddefb8";
const BATCH: &str = "monster-full-mechanics-orc-sam-backpack-201189f8";
const PROJECT_REVISION: &str = "orc-sam-backpack-raid-r1";
pub(super) const RECORD_DELTA: usize = 4;
pub(super) const IMPORT_DELTA: usize = 1;

#[derive(Deserialize)]
struct Packet {
    source: Source,
    records: [ProjectReferenceRecord; RECORD_DELTA],
    profiles: [ProjectV2AuthoringProfile; 3],
    voice_order: [usize; 3],
}

#[derive(Deserialize)]
struct Source {
    repository: String,
    path: String,
    revision: String,
}

pub(super) fn apply(draft: &mut ProjectV2Draft) -> Result<(), Box<dyn std::error::Error>> {
    let Packet {
        source,
        records,
        mut profiles,
        voice_order,
    } = serde_json::from_slice(PACKET)?;
    if source.repository != "zimbadev/crystalserver"
        || source.revision != SOURCE_REVISION
        || source.path != "data-global/monster/raids/orc_sam_backpack.lua"
        || voice_order != [2, 0, 1]
    {
        return Err("Orc Sam Backpack source identity drifted".into());
    }
    let ProjectReferenceRecord::Creature { identity, .. } = &records[0] else {
        return Err("Orc Sam Backpack Creature record missing".into());
    };
    if identity.key != "oteryn:creature.orc_sam_backpack" {
        return Err("Orc Sam Backpack Creature identity drifted".into());
    }
    let voices = draft
        .state
        .authoring_profiles
        .iter()
        .find_map(|row| match &row.data {
            ProjectV2AuthoringProfileData::Behavior(behavior)
                if row.target.key == "oteryn:behavior.creature.orc" =>
            {
                behavior.voices.as_ref()
            }
            _ => None,
        })
        .ok_or("Orc presentation voice source missing")?;
    let [first, second, third] = voices.entries.as_slice() else {
        return Err("Orc presentation voice source changed".into());
    };
    let ProjectV2AuthoringProfileData::Behavior(behavior) = &mut profiles[1].data else {
        return Err("Orc Sam Backpack Behavior profile missing".into());
    };
    let mut raid_voices = voices.clone();
    raid_voices.entries = vec![third.clone(), first.clone(), second.clone()];
    behavior.voices = Some(raid_voices);

    let revision = format!("crystalserver-raid-creature-1530:{SOURCE_REVISION}");
    let digest = hex_sha256(PACKET);
    let binding = ProjectV2SourceIdentityBinding {
        source_key: "oteryn:source.crystalserver".into(),
        source_revision: revision.clone(),
        identity_namespace: "crystalserver/monster-file".into(),
        external_id: source.path,
        target: profiles[0].target.clone(),
        disposition: ProjectV2SourceIdentityDisposition::Exact,
    };
    draft.core.imports.push(ImportBatch {
        batch_id: BATCH.into(),
        source_repository: source.repository,
        source_revision: revision.clone(),
        source_artifact_sha256: digest.clone(),
        access_disposition: "PENDING".into(),
        source_generation_profile: "crystalserver-raid-creature-1530".into(),
        importer: "orc-sam-backpack-admission".into(),
        mapper: "orc_sam_backpack_admission.rs".into(),
        mapper_revision: SOURCE_REVISION.into(),
        mapper_sha256: hex_sha256(include_bytes!("orc_sam_backpack_admission.rs")),
        candidates: Vec::new(),
        reimport_states: Vec::new(),
    });
    draft.state.sources.push(ProjectV2Source {
        key: binding.source_key.clone(),
        import_batch_id: BATCH.into(),
        revision,
        sha256: digest,
        evidence: ProjectV2EvidenceClass::OtsHypothesisOnly,
    });
    draft.core.project_revision = PROJECT_REVISION.into();
    draft.core.records.extend(records);
    draft.state.authoring_profiles.extend(profiles);
    draft.state.source_identity_bindings.push(binding);
    Ok(())
}
