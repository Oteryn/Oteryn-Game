//! NPC definitions for the exact Crystal 201189f8 Global XML contacts.
use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Contacts {
    records: Vec<ProjectReferenceRecord>,
    authoring_profiles: Vec<ProjectV2AuthoringProfile>,
    declarations: Vec<ProjectV2Declaration>,
}

pub(super) const RECORD_COUNT: usize = 20;

pub(super) fn apply(draft: &mut ProjectV2Draft) -> Result<(), Box<dyn std::error::Error>> {
    let contacts: Contacts = serde_json::from_slice(include_bytes!(
        "../../../../tools/content-schema/npc-authoring/samples/crystal-world-contacts.json"
    ))?;
    if contacts.records.len() != RECORD_COUNT || contacts.authoring_profiles.len() != RECORD_COUNT {
        return Err("Crystal World NPC profile count drift".into());
    }
    draft.core.records.extend(contacts.records);
    draft
        .state
        .authoring_profiles
        .extend(contacts.authoring_profiles);
    draft.state.declarations.extend(contacts.declarations);
    Ok(())
}
