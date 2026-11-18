//! Crystal 201189f8 Dangerous Depths portal-room captures.
use super::{ActivityTemplateRefusal, BossLeverCandidate, CRYSTAL_REVISION};
use crate::content::native_gameplay::NativeGameplayState;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const PROFILE_KEYS: [&str; 3] = [
    "oteryn:creature.the_baron_from_below",
    "oteryn:creature.the_count_of_the_core",
    "oteryn:creature.the_duke_of_the_depths",
];
const SOURCES: [(&str, &str); 5] = [
    (
        "actions_using_crystals.lua",
        "ce4bf507f6e84b108edc8196887158ebc61e1a554604f8bde2d64b87290874d3",
    ),
    (
        "creaturescripts_bosses_mission_depths.lua",
        "6f4b1af04775b46536143729eeafc870e960e3cc9f3b048bd30155aaf7e18d49",
    ),
    (
        "creaturescripts_fiery_heart.lua",
        "0e216bfe168cefdfa1b41ce3241d0734e2eb73a88467c2b70bbb6b84da9fbcc6",
    ),
    (
        "creaturescripts_snail_slime_kill.lua",
        "2c5a474dcf269fc845d78dd117c6d9a221d5b4b64bda53562b62188a179e168e",
    ),
    (
        "movements_boss_entrance.lua",
        "25b0efc67d1544d737fb98cb4090f5d41e9ecba7b35a05d3cee92356adbaa5b9",
    ),
];
const PREFIX: &str = "data-global/scripts/quests/dangerous_depth/";
const CAPTURES: [(&str, &[u8]); 3] = [
    (
        "content/encounters/bosses/warzone-iv.json",
        include_bytes!("../../../../../../../content/encounters/bosses/warzone-iv.json"),
    ),
    (
        "content/encounters/bosses/warzone-v.json",
        include_bytes!("../../../../../../../content/encounters/bosses/warzone-v.json"),
    ),
    (
        "content/encounters/bosses/warzone-vi.json",
        include_bytes!("../../../../../../../content/encounters/bosses/warzone-vi.json"),
    ),
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Capture {
    schema: String,
    source_revision: String,
    sources: Vec<Source>,
    timeout_exit: [i32; 3],
    reward_destination: [i32; 3],
    candidate: BossLeverCandidate,
}

pub(super) fn is_warzone_creature(key: &str) -> bool {
    PROFILE_KEYS.contains(&key)
}

pub(super) fn candidates() -> Result<Vec<BossLeverCandidate>, ActivityTemplateRefusal> {
    CAPTURES
        .into_iter()
        .zip(PROFILE_KEYS)
        .map(|((path, bytes), creature)| {
            use ActivityTemplateRefusal::InvalidSourceCapture;
            let record: Capture =
                serde_json::from_slice(bytes).map_err(|_| InvalidSourceCapture)?;
            let mut signed: serde_json::Value =
                serde_json::from_slice(bytes).map_err(|_| InvalidSourceCapture)?;
            signed
                .get_mut("candidate")
                .and_then(serde_json::Value::as_object_mut)
                .and_then(|candidate| candidate.remove("sha256"))
                .ok_or(InvalidSourceCapture)?;
            signed.sort_all_objects();
            let digest =
                Sha256::digest(serde_json::to_vec(&signed).map_err(|_| InvalidSourceCapture)?);
            if record.schema != "OTERYN_CRYSTAL_WARZONE_ACTIVITY/v1"
                || record.source_revision != CRYSTAL_REVISION
                || record.candidate.path != path
                || record.candidate.creature_key.as_deref() != Some(creature)
                || record.candidate.sha256
                    != digest
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>()
                || record.sources.len() != SOURCES.len()
                || record
                    .sources
                    .iter()
                    .zip(SOURCES)
                    .any(|(actual, (file, sha))| {
                        actual.path.strip_prefix(PREFIX) != Some(file) || actual.sha256 != sha
                    })
                || record.timeout_exit[2] != 15
                || record.reward_destination[2] != 15
            {
                return Err(InvalidSourceCapture);
            }
            Ok(record.candidate)
        })
        .collect()
}

pub(super) fn implements(
    native: &NativeGameplayState,
    candidate: &BossLeverCandidate,
    hook: &str,
) -> bool {
    let Some(creature) = candidate
        .creature_key
        .as_deref()
        .filter(|key| is_warzone_creature(key))
    else {
        return false;
    };
    let children: &[&str] = match (creature, hook) {
        (_, "DepthWarzoneBossDeath") => &[],
        ("oteryn:creature.the_count_of_the_core", "SnailSlimeDeath" | "SnailSlimeThink") => {
            &["oteryn:creature.snail_slime"]
        }
        ("oteryn:creature.the_duke_of_the_depths", "FieryHeartThink") => &[
            "oteryn:creature.fiery_heart",
            "oteryn:creature.fiery_blood",
            "oteryn:creature.the_fire_empowered_duke",
        ],
        _ => return false,
    };
    children.iter().all(|key| {
        native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target.key == *key)
            .is_some_and(crate::content::bosses::primal_closure::abilities_closed)
    })
}
