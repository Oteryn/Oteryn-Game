//! Independently qualified W01 prepare-death declarations for explicit review.
//! Neither event is attached by the pinned monster registration lists; the
//! production metadata factory does not install these declarations.
use super::{QualifiedSoulWarHealthPolicy, SoulWarHealthCallbackKind};
use crate::content::native_gameplay::{CreatureProfileRecord, CreatureProfilesDocument};
use crate::content::{ProjectV2SourceIdentityBinding, ProjectV2SourceIdentityDisposition};
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Witness {
    schema: String,
    source_revision: String,
    source_definitions_sha256: String,
    sources: BTreeMap<String, String>,
    events: Vec<Event>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    native: CreatureProfileRecord,
    source_binding: ProjectV2SourceIdentityBinding,
    event: String,
    heal: u32,
    cancel_incoming: bool,
    marker_sub_id: u32,
    marker_duration_ms: u32,
    marker_health_gain: u32,
}
fn witness() -> Option<&'static Witness> {
    static SOURCE: OnceLock<Option<Witness>> = OnceLock::new();
    SOURCE
        .get_or_init(|| serde_json::from_str(include_str!("w01_prepare_death_witness.json")).ok())
        .as_ref()
}
const PIN: &str = "47dfd51f45280a59a1d3e50ba7edd573d7234446";
const SOURCES: [(&str, &str); 4] = [
    (
        "data-otservbr-global/monster/raids/hirintror.lua",
        "9aa9aa3b82f03a0318d068e664ade1993a9e83940dd0cea25459d0179be5b1ea",
    ),
    (
        "data-otservbr-global/monster/raids/white_pale.lua",
        "cc1ac834e8f85d9771640c22d2542dd02749e439f07d24b584c00c36d6dd68eb",
    ),
    (
        "data-otservbr-global/scripts/creaturescripts/monster/hirintror_heal.lua",
        "15316485b3e6cfb4b6fefbaaaa9900a24ff58f22778bf81cdba9f2a6b5eaf5bd",
    ),
    (
        "data-otservbr-global/scripts/creaturescripts/monster/white_pale_heal.lua",
        "c180115e58f68d117195b689d98a301931a382693589e105b178d035712d329c",
    ),
];
pub(super) fn policies(
    document: &CreatureProfilesDocument,
    digest: [u8; 32],
) -> Vec<QualifiedSoulWarHealthPolicy> {
    let Some(witness) = witness() else {
        return Vec::new();
    };
    if witness.schema != "OTERYN_W01_PREPARE_DEATH/v1"
        || witness.source_revision != PIN
        || document.schema != "OTERYN_NATIVE_CREATURE_PROFILES/v1"
        || document.source_definitions_sha256.as_deref()
            != Some(witness.source_definitions_sha256.as_str())
        || witness.events.len() != 2
        || witness.sources.len() != SOURCES.len()
        || SOURCES
            .iter()
            .any(|(path, hash)| witness.sources.get(*path).map(String::as_str) != Some(*hash))
    {
        return Vec::new();
    }
    witness
        .events
        .iter()
        .filter_map(|event| {
            let reference = &event.native.profile.target;
            if reference.family != crate::content::ProjectV2Family::Creature
                || reference.revision != "definition-r1"
                || witness
                    .events
                    .iter()
                    .filter(|other| other.native.profile.target == *reference)
                    .count()
                    != 1
            {
                return None;
            }
            let (name, heal, external, kind) = match reference.key.as_str() {
                "oteryn:creature.hirintror" => (
                    "HirintrorHeal",
                    1000,
                    "raids/hirintror",
                    SoulWarHealthCallbackKind::HirintrorHeal,
                ),
                "oteryn:creature.white_pale" => (
                    "WhitePaleHeal",
                    400,
                    "raids/white_pale",
                    SoulWarHealthCallbackKind::WhitePaleHeal,
                ),
                _ => return None,
            };
            let binding = &event.source_binding;
            if event.event != name
                || event.heal != heal
                || !event.cancel_incoming
                || event.marker_sub_id != 88888
                || event.marker_duration_ms != 900000
                || event.marker_health_gain != 0
                || binding.target != *reference
                || binding.source_revision != PIN
                || binding.source_key != "oteryn:source.canary"
                || binding.identity_namespace != "canary/monster-file"
                || binding.external_id != external
                || binding.disposition != ProjectV2SourceIdentityDisposition::Exact
            {
                return None;
            }
            let mut actual = document
                .records
                .iter()
                .filter(|record| record.profile.target == *reference);
            if actual.next() != Some(&event.native) || actual.next().is_some() {
                return None;
            }
            Some(QualifiedSoulWarHealthPolicy {
                digest,
                key: reference.key.clone(),
                revision: reference.revision.clone(),
                kind,
            })
        })
        .collect()
}
