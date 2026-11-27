//! Crystal catalogue facts and the six Creature definitions the base did not carry.
use super::*;

const PACKET: &[u8] =
    include_bytes!("../../../../imports/crystalserver/cyclopedia-creatures-201189f.json");
const MAPPER: &[u8] =
    include_bytes!("../../../../tools/content-migration/creature_admission_stage.py");
const REVISION: &str = "201189f871d23597b82caf88fd611454a2ddefb8";
const BATCH: &str = "cyclopedia-creatures-crystal-201189f-r1";
const SCHEMA: &str = "OTERYN_CRYSTAL_CYCLOPEDIA_CREATURES/v1";
pub(super) const RECORD_DELTA: usize = 78;
pub(super) const IMPORT_DELTA: usize = 1;

pub(super) fn apply(draft: &mut ProjectV2Draft) -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_game_server::content::ProjectV2AuthoringProfileData;

    let packet: Value = serde_json::from_slice(PACKET)?;
    if packet["schema"] != SCHEMA || packet["revision"] != REVISION {
        return Err("Crystal Cyclopedia Creature source mismatch".into());
    }
    let records: Vec<ProjectReferenceRecord> = serde_json::from_value(packet["records"].clone())?;
    let profiles: Vec<ProjectV2AuthoringProfile> =
        serde_json::from_value(packet["profiles"].clone())?;
    let bindings: Vec<ProjectV2SourceIdentityBinding> =
        serde_json::from_value(packet["bindings"].clone())?;
    if records.len() != RECORD_DELTA {
        return Err("Crystal Cyclopedia Creature record count mismatch".into());
    }
    draft.core.records.extend(records);
    draft.state.authoring_profiles.extend(profiles);
    draft.state.source_identity_bindings.extend(bindings);
    let categories: Value = serde_json::from_str(include_str!(
        "../../../../rulesets/progression/bosstiary/categories.json"
    ))?;
    let categories = categories.as_array().ok_or("Bosstiary categories")?;
    for row in packet["sources"]
        .as_array()
        .ok_or("Crystal Cyclopedia sources")?
    {
        let key = row["creature"].as_str().ok_or("Crystal Creature key")?;
        let target = draft
            .state
            .authoring_profiles
            .iter_mut()
            .find(|profile| profile.target.key == key)
            .ok_or("Crystal Cyclopedia Creature profile missing")?;
        let ProjectV2AuthoringProfileData::Creature(profile) = &mut target.data else {
            return Err("Crystal Cyclopedia profile is not a Creature".into());
        };
        let details = profile.details.as_mut().ok_or("Crystal Creature details")?;
        if let Some(category) = row["bosstiary"].as_str() {
            let rule = categories
                .iter()
                .find(|rule| rule["category"] == category)
                .ok_or("Unknown Crystal Bosstiary category")?;
            let boss = serde_json::from_value(serde_json::json!({
                "category": category,
                "prowess_kills": rule["kills"][0],
                "expertise_kills": rule["kills"][1],
                "mastery_kills": rule["kills"][2],
                "prowess_points": rule["points"][0],
                "expertise_points": rule["points"][1],
                "mastery_points": rule["points"][2],
            }))?;
            if details
                .bosstiary
                .as_ref()
                .is_some_and(|existing| existing != &boss)
            {
                return Err("Crystal Bosstiary category conflicts with Creature profile".into());
            }
            details.bosstiary = Some(boss);
        } else {
            details.system_eligibility.exclusive_prey = row["exclusive_prey"]
                .as_bool()
                .ok_or("Crystal exclusive Prey flag")?;
        }
    }
    let digest = hex_sha256(PACKET);
    let source_revision = format!("crystalserver-cyclopedia:{REVISION}");
    draft.core.imports.push(ImportBatch {
        batch_id: BATCH.to_owned(),
        source_repository: "zimbadev/crystalserver".to_owned(),
        source_revision: source_revision.clone(),
        source_artifact_sha256: digest.clone(),
        access_disposition: "PENDING".to_owned(),
        source_generation_profile: SCHEMA.to_owned(),
        importer: SCHEMA.to_owned(),
        mapper: "creature_admission_stage.py".to_owned(),
        mapper_revision: REVISION.to_owned(),
        mapper_sha256: hex_sha256(MAPPER),
        candidates: Vec::new(),
        reimport_states: Vec::new(),
    });
    draft.state.sources.push(ProjectV2Source {
        key: "oteryn:source.crystalserver".to_owned(),
        import_batch_id: BATCH.to_owned(),
        revision: source_revision,
        sha256: digest,
        evidence: ProjectV2EvidenceClass::OtsHypothesisOnly,
    });
    draft.core.project_revision = BATCH.to_owned();
    Ok(())
}
