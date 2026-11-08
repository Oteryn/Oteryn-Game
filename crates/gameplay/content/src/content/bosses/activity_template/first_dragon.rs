//! Explicit chosen s5 adapter; the full Spirit of Fertility source capture stays closed.
//! Crystal9f5a72c creaturescripts_death_dragon_essence.lua:11 and death_fallen.lua:12
//! supply the final creature and portal landing; actions_lever.lua:6 supplies cooldown.
use super::{ActivityTemplateRefusal, BossLeverCandidate};
use crate::content::native_gameplay::NativeGameplayState;
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub(super) const PATH: &str = "content/encounters/bosses/selected-first-dragon.json";
const QUEST: &str = "oteryn:quest.the_first_dragon_quest";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    path: String,
    sha256: String,
    revision: String,
    url: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
    quest: String,
    stage: String,
    basis: String,
    recipe_sha256: String,
    sources: Vec<Source>,
    candidate: BossLeverCandidate,
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn candidate() -> Result<BossLeverCandidate, ActivityTemplateRefusal> {
    let invalid = || ActivityTemplateRefusal::InvalidSourceCapture;
    let bytes =
        include_bytes!("../../../../../../../content/encounters/bosses/selected-first-dragon.json");
    let document: Document = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    let mut raw: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    raw.get_mut("candidate")
        .and_then(serde_json::Value::as_object_mut)
        .and_then(|candidate| candidate.remove("sha256"))
        .ok_or_else(invalid)?;
    raw.sort_all_objects();
    let digest = hex(&Sha256::digest(
        serde_json::to_vec(&raw).map_err(|_| invalid())?,
    ));
    if document.schema != "OTERYN_SELECTED_QUEST_ACTIVITY/v1"
        || document.quest != QUEST
        || document.stage != "s5"
        || document.basis != "CHOSEN_OTERYN_APPROXIMATION"
        || !super::hex_digest(&document.recipe_sha256)
        || document.candidate.path != PATH
        || document.candidate.sha256 != digest
        || document.candidate.creature_key.as_deref() != Some("oteryn:creature.the_first_dragon")
        || document.sources.len() != 4
        || document.sources.iter().any(|source| {
            source.revision != "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
                || !source
                    .path
                    .starts_with("data-global/scripts/quests/the_first_dragon/")
                || !super::hex_digest(&source.sha256)
                || source.url
                    != format!(
                        "https://github.com/zimbadev/crystalserver/blob/{}/{}",
                        source.revision, source.path
                    )
        })
    {
        return Err(invalid());
    }
    Ok(document.candidate)
}

pub(super) struct Closure;
impl Closure {
    pub(super) fn implements(&self, hook: &str) -> bool {
        hook == "SelectedFirstDragonDeath"
    }
}
pub(super) fn qualify(
    native: &NativeGameplayState,
    selected: &BossLeverCandidate,
) -> Option<Closure> {
    if candidate().ok().as_ref() != Some(selected) {
        return None;
    }
    // Execution belongs to the existing native death/QuestState owners. The
    // activity runtime validates that compiled consumer before it creates actors.
    let fingerprint = native.creature_profile_fingerprint("oteryn:creature.the_first_dragon")?;
    if selected.native_profile_sha256.as_deref() != Some(hex(&fingerprint).as_str()) {
        return None;
    }
    Some(Closure)
}
