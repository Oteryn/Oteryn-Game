//! Copied source event order for the existing W11 native participant closure.
use super::native_creature_death_events::{QualifiedNativeDeathEvent, QualifiedNativeDeathEvents};
use super::native_gameplay::CreatureProfilesDocument;
use serde::Deserialize;
use std::sync::{Arc, OnceLock};
include!("native_soul_war_death_keys.rs");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Capture {
    schema: String,
    source_repository: String,
    source_revision: String,
    source_definitions_sha256: String,
    sources: Vec<Source>,
    records: Vec<Parent>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    path: String,
    git_blob: String,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Parent {
    creature_key: String,
    creature_revision: String,
    parent_path: String,
    parent_git_blob: String,
    monster_sha256: String,
    registered_events: Vec<Registration>,
    on_death: Vec<Death>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    registration_index: u16,
    name: String,
    line: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Death {
    registration_index: u16,
    name: String,
    line: u32,
    source_path: String,
    source_sha256: String,
}
fn capture() -> Option<&'static Capture> {
    static CAPTURE: OnceLock<Option<Capture>> = OnceLock::new();
    CAPTURE
        .get_or_init(|| {
            serde_json::from_slice(include_bytes!(
                "../../../../../tools/content-schema/native-gameplay/srx3-soul-war-death-events.json"
            ))
            .ok()
        })
        .as_ref()
}
fn digest(raw: &str) -> Option<[u8; 32]> {
    if raw.len() != 64 || !raw.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut result = [0; 32];
    for (index, slot) in result.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&raw[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(result)
}
/// This guard is independent of parsing success. Malformed source metadata
/// cannot silently turn a covered donor type into an ordinary native actor.
pub(crate) fn requires_native_death_events(key: &str) -> bool {
    COVERED_KEYS.contains(&key)
}
fn source_valid(source: &Source) -> bool {
    source.git_blob.len() == 40
        && source.git_blob.bytes().all(|byte| byte.is_ascii_hexdigit())
        && digest(&source.sha256).is_some()
        && super::native_soul_war_callbacks::callback_source_sha(&source.path)
            == Some(source.sha256.as_str())
}
pub(crate) fn death_event_policies(
    document: &CreatureProfilesDocument,
    outer: [u8; 32],
) -> Vec<Arc<QualifiedNativeDeathEvents>> {
    let Some(capture) = capture() else {
        return Vec::new();
    };
    if outer == [0; 32]
        || !super::native_soul_war_callbacks::qualifies_document(document)
        || capture.schema != "SRX3_SOUL_WAR_ORDERED_DEATH_EVENTS/v1"
        || capture.source_repository != "zimbadev/crystalserver"
        || capture.source_revision != "201189f871d23597b82caf88fd611454a2ddefb8"
        || document.source_definitions_sha256.as_deref()
            != Some(capture.source_definitions_sha256.as_str())
        || capture.records.len() != COVERED_KEYS.len()
        || capture.sources.len() != 54
        || capture
            .records
            .iter()
            .filter(|row| !row.on_death.is_empty())
            .count()
            != 22
        || !capture.sources.iter().all(source_valid)
        || capture.sources.iter().enumerate().any(|(index, source)| {
            capture.sources[..index]
                .iter()
                .any(|old| old.path == source.path)
        })
        || COVERED_KEYS.iter().any(|key| {
            capture
                .records
                .iter()
                .filter(|row| row.creature_key == *key)
                .count()
                != 1
        })
    {
        return Vec::new();
    }
    let mut result = Vec::new();
    for parent in &capture.records {
        let Some(source) = capture
            .sources
            .iter()
            .find(|source| source.path == parent.parent_path)
        else {
            return Vec::new();
        };
        if parent.creature_revision != "definition-r1"
            || source.git_blob != parent.parent_git_blob
            || source.sha256 != parent.monster_sha256
            || parent
                .registered_events
                .iter()
                .enumerate()
                .any(|(index, event)| {
                    usize::from(event.registration_index) != index
                        || event.line == 0
                        || event.name.is_empty()
                })
        {
            return Vec::new();
        }
        let Some(record) = document.records.iter().find(|record| {
            record.profile.target.key == parent.creature_key
                && record.profile.target.revision == parent.creature_revision
        }) else {
            return Vec::new();
        };
        let mut events = Vec::new();
        for event in &parent.on_death {
            let Some(registration) = parent
                .registered_events
                .get(usize::from(event.registration_index))
            else {
                return Vec::new();
            };
            let Some(body) = capture
                .sources
                .iter()
                .find(|source| source.path == event.source_path)
            else {
                return Vec::new();
            };
            if registration.name != event.name
                || registration.line != event.line
                || body.sha256 != event.source_sha256
                || event.source_path != "data-global/scripts/quests/soul_war/soul_war_mechanics.lua"
                || !matches!(
                    event.name.as_str(),
                    "SoulWarBossesDeath"
                        | "GreedMonsterDeath"
                        | "SoulSphereDeath"
                        | "SoulCageDeath"
                        | "MirroredNightmareBossAccess"
                        | "MegalomaniaDeath"
                        | "SoulWarAspectOfPowerDeath"
                        | "HazardousPhantomDeath"
                        | "NecromanticFocusDeath"
                )
            {
                return Vec::new();
            }
            let Some(sha) = digest(&event.source_sha256) else {
                return Vec::new();
            };
            events.push(QualifiedNativeDeathEvent::from_qualified_source(
                event.registration_index,
                event.name.clone(),
                event.source_path.clone(),
                sha,
            ));
        }
        let Some(policy) = QualifiedNativeDeathEvents::from_qualified_profile(
            record.profile.clone(),
            outer,
            events,
        ) else {
            return Vec::new();
        };
        result.push(Arc::new(policy));
    }
    result
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    #[test]
    fn srx3_soul_war_death_descriptors_keep_effective_source_order_and_empty_parents() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/creatures/definitions/spell-native-profiles.json");
        let raw = std::fs::read(path).expect("actual native content checkout");
        let mut document: CreatureProfilesDocument =
            serde_json::from_slice(&raw).expect("actual complete native source records");
        let policies = death_event_policies(&document, [7; 32]);
        assert_eq!(policies.len(), 52);
        assert_eq!(
            policies
                .iter()
                .filter(|policy| policy.events().is_empty())
                .count(),
            30
        );
        let purple = policies
            .iter()
            .find(|policy| policy.creature().key == "oteryn:creature.goshnar_s_megalomania_purple")
            .expect("source-covered Purple");
        assert!(
            purple.events().is_empty(),
            "Purple has no onDeath registration"
        );
        let green = policies
            .iter()
            .find(|policy| policy.creature().key == "oteryn:creature.goshnar_s_megalomania_green")
            .expect("actual Green event vector");
        assert_eq!(green.events().len(), 1);
        assert_eq!(green.events()[0].registration_index(), 1);
        assert_eq!(green.events()[0].name(), "MegalomaniaDeath");
        let mirror = policies
            .iter()
            .find(|policy| policy.creature().key == "oteryn:creature.mirror_image")
            .expect("source-covered effective last assignment");
        assert!(mirror.events().is_empty());
        assert!(green.matches([7; 32], &green.creature().key, "definition-r1"));
        assert!(!green.matches([8; 32], &green.creature().key, "definition-r1"));
        let record = document
            .records
            .iter_mut()
            .find(|record| {
                record.profile.target.key == "oteryn:creature.goshnar_s_megalomania_green"
            })
            .expect("actual qualified native parent");
        record.profile.target.revision = "changed-source-parent".into();
        assert!(death_event_policies(&document, [7; 32]).is_empty());
        assert!(requires_native_death_events(
            "oteryn:creature.goshnar_s_megalomania_green"
        ));
        assert!(!requires_native_death_events("oteryn:creature.rat"));
    }
}
