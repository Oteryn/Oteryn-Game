//! Independently source-qualified W01 copied CreatureEvent registrations.
//! The source capture is generated from exact pinned Lua and actual canonical
//! identity bindings; a changed root alone cannot bless changed body semantics.
use super::native_creature_death_events::{QualifiedNativeDeathEvent, QualifiedNativeDeathEvents};
use super::native_gameplay::CreatureProfilesDocument;
use super::{
    ProjectV2AuthoringProfile, ProjectV2Family, ProjectV2SourceIdentityBinding,
    ProjectV2SourceIdentityDisposition,
};
use sha2::{Digest, Sha256};
use std::sync::{Arc, OnceLock};
include!("native_w01_death_events_pin.rs");
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Witness {
    schema: String,
    source_definitions_sha256: String,
    parents: Vec<Parent>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Parent {
    native_profile: ProjectV2AuthoringProfile,
    source_identity_binding: ProjectV2SourceIdentityBinding,
    monster_source: MonsterSource,
    registered_events: Vec<Registration>,
    on_death: Vec<Handler>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MonsterSource {
    repository: String,
    revision: String,
    path: String,
    git_blob: String,
    sha256: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    registration_index: u16,
    name: String,
    line: u32,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Handler {
    registration_index: u16,
    name: String,
    line: u32,
    source_path: String,
    source_sha256: String,
}
fn hex32(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return None;
    }
    let mut result = [0; 32];
    for (index, chunk) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        result[index] = u8::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()?;
    }
    Some(result)
}
fn witness() -> Option<&'static Witness> {
    static SOURCE: OnceLock<Option<Witness>> = OnceLock::new();
    SOURCE
        .get_or_init(|| {
            let raw = include_bytes!("native_w01_death_events_witness.json");
            if Sha256::digest(raw).as_slice() != WITNESS_SHA256.as_slice() {
                return None;
            }
            let result: Witness = serde_json::from_slice(raw).ok()?;
            if result.schema != "OTERYN_W01_ORDERED_NATIVE_DEATH_EVENTS/v1"
                || result.parents.len() != COVERED_KEYS.len()
                || hex32(&result.source_definitions_sha256).is_none()
            {
                return None;
            }
            for parent in &result.parents {
                let target = &parent.native_profile.target;
                if target.family != ProjectV2Family::Creature
                    || !COVERED_KEYS.contains(&target.key.as_str())
                    || result
                        .parents
                        .iter()
                        .filter(|other| other.native_profile.target == *target)
                        .count()
                        != 1
                    || parent.source_identity_binding.target != *target
                    || parent.source_identity_binding.disposition
                        != ProjectV2SourceIdentityDisposition::Exact
                    || parent.registered_events.len() > 32
                    || parent
                        .registered_events
                        .iter()
                        .enumerate()
                        .any(|(index, event)| {
                            usize::from(event.registration_index) != index || event.line == 0
                        })
                    || parent
                        .on_death
                        .windows(2)
                        .any(|pair| pair[0].registration_index >= pair[1].registration_index)
                    || parent.on_death.iter().any(|event| {
                        parent
                            .registered_events
                            .get(usize::from(event.registration_index))
                            .is_none_or(|registered| {
                                registered.name != event.name || registered.line != event.line
                            })
                            || hex32(&event.source_sha256).is_none()
                    })
                {
                    return None;
                }
                let source = &parent.monster_source;
                let known = (source.repository == "opentibiabr/canary"
                    && source.revision == "47dfd51f45280a59a1d3e50ba7edd573d7234446")
                    || (source.repository == "zimbadev/crystalserver"
                        && source.revision == "201189f871d23597b82caf88fd611454a2ddefb8");
                if !known
                    || source.path.is_empty()
                    || source.git_blob.len() != 40
                    || hex32(&source.sha256).is_none()
                {
                    return None;
                }
            }
            Some(result)
        })
        .as_ref()
}
pub(crate) fn requires_native_death_events(key: &str) -> bool {
    COVERED_KEYS.contains(&key)
}
/// Literal effective registration from the same pinned Lua capture, including
/// non-death event kinds. A declaration without a parent attachment is absent.
pub(crate) fn registered_event_index(
    document: &CreatureProfilesDocument,
    target: &super::ProjectV2DefinitionRef,
    name: &str,
) -> Option<u16> {
    let witness = witness()?;
    if document.source_definitions_sha256.as_deref()
        != Some(witness.source_definitions_sha256.as_str())
    {
        return None;
    }
    let mut parents = witness
        .parents
        .iter()
        .filter(|parent| parent.native_profile.target == *target);
    let parent = parents.next()?;
    if parents.next().is_some() {
        return None;
    }
    let mut records = document
        .records
        .iter()
        .filter(|record| record.profile.target == *target);
    if records.next()?.profile != parent.native_profile || records.next().is_some() {
        return None;
    }
    let mut events = parent
        .registered_events
        .iter()
        .filter(|event| event.name == name);
    let event = events.next()?;
    events.next().is_none().then_some(event.registration_index)
}
pub(crate) fn death_event_policies(
    document: &CreatureProfilesDocument,
    digest: [u8; 32],
) -> Vec<Arc<QualifiedNativeDeathEvents>> {
    let Some(witness) = witness() else {
        return Vec::new();
    };
    if document.schema != "OTERYN_NATIVE_CREATURE_PROFILES/v1"
        || document.source_definitions_sha256.as_deref()
            != Some(witness.source_definitions_sha256.as_str())
    {
        return Vec::new();
    }
    witness
        .parents
        .iter()
        .filter_map(|parent| {
            let target = &parent.native_profile.target;
            let mut records = document
                .records
                .iter()
                .filter(|record| record.profile.target == *target);
            let record = records.next()?;
            if records.next().is_some() || record.profile != parent.native_profile {
                return None;
            }
            let events = parent
                .on_death
                .iter()
                .map(|event| {
                    Some(QualifiedNativeDeathEvent::from_qualified_source(
                        event.registration_index,
                        event.name.clone(),
                        event.source_path.clone(),
                        hex32(&event.source_sha256)?,
                    ))
                })
                .collect::<Option<Vec<_>>>()?;
            QualifiedNativeDeathEvents::from_qualified_profile(
                record.profile.clone(),
                digest,
                events,
            )
            .map(Arc::new)
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_w01_ordered_death_metadata_qualifies_actual_native_body_and_empty_registration()
    -> Result<(), Box<dyn std::error::Error>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), String> {
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../content/creatures/definitions/spell-native-profiles.json");
                let document: CreatureProfilesDocument =
                    serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                let rows = death_event_policies(&document, [7; 32]);
                assert_eq!(
                    rows.len(),
                    71,
                    "all actual source parents require ordered or explicit-empty metadata"
                );
                let find = |key: &str| rows.iter().find(|row| row.creature().key == key);
                let rupture = find("oteryn:creature.rupture").ok_or("actual Rupture descriptor")?;
                assert_eq!(
                    rupture
                        .events()
                        .iter()
                        .map(|event| (event.name(), event.registration_index()))
                        .collect::<Vec<_>>(),
                    vec![("HeartBossDeath", 2)]
                );
                let cerebellum =
                    find("oteryn:creature.cerebellum").ok_or("covered empty source descriptor")?;
                assert!(
                    cerebellum.events().is_empty(),
                    "empty is qualified data, not missing metadata"
                );
                Ok(())
            })?
            .join()
            .map_err(|_| "ordered source metadata worker panicked")?
            .map_err(Into::into)
    }
}
