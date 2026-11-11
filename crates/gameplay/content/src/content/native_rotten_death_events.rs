//! Six exact entry sources; the wider dynamically registered Rotten closure is separate.
//! These declared mechanics callbacks precede the shared personal-reward owner;
//! an empty copied vector does not describe the generic dynamic BossDeath hook.
use super::native_creature_death_events::{QualifiedNativeDeathEvent, QualifiedNativeDeathEvents};
use super::native_gameplay::CreatureProfilesDocument;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::{Arc, OnceLock};

const COVERED_KEYS: &[&str] = &[
    "oteryn:creature.bakragore",
    "oteryn:creature.chagorz",
    "oteryn:creature.ichgahal",
    "oteryn:creature.murcion",
    "oteryn:creature.mushroom",
    "oteryn:creature.vemiath",
];
const FROZEN_SOURCE_SHA256: &str =
    "fd21849dd13decf03b015c7d64692ae6380e50b45f5c61e59e00060c45c7309d";
const RAW: &[u8] = include_bytes!(
    "../../../../../tools/content-schema/native-gameplay/srx3-rotten-death-events.json"
);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Capture {
    schema: String,
    source_repository: String,
    source_revision: String,
    source_definitions_sha256: String,
    #[serde(rename = "registry_proof")]
    _registry_proof: serde_json::Value,
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

fn qualified_capture(raw: &[u8]) -> Option<Capture> {
    let value: serde_json::Value = serde_json::from_slice(raw).ok()?;
    let mut source = value.clone();
    source
        .as_object_mut()?
        .remove("source_definitions_sha256")?;
    // The generator emits recursively sorted JSON. This immutable source proof
    // covers every copied index and the full-corpus unloaded-name check; only
    // the independently requalified native definition pin can be refreshed.
    let actual: [u8; 32] = Sha256::digest(serde_json::to_vec(&source).ok()?).into();
    if actual != digest(FROZEN_SOURCE_SHA256)? {
        return None;
    }
    serde_json::from_value(value).ok()
}

fn capture() -> Option<&'static Capture> {
    static CAPTURE: OnceLock<Option<Capture>> = OnceLock::new();
    CAPTURE.get_or_init(|| qualified_capture(RAW)).as_ref()
}

pub(crate) fn requires_native_death_events(key: &str) -> bool {
    COVERED_KEYS.contains(&key)
}

pub(crate) fn death_event_policies(
    document: &CreatureProfilesDocument,
    outer: [u8; 32],
) -> Vec<Arc<QualifiedNativeDeathEvents>> {
    let Some(capture) = capture() else {
        return Vec::new();
    };
    if outer == [0; 32]
        || !super::native_rotten_blood_callbacks::qualifies_document(document)
        || capture.schema != "SRX3_ROTTEN_ENTRY_ORDERED_DEATH_EVENTS/v1"
        || capture.source_repository != "zimbadev/crystalserver"
        || capture.source_revision != "201189f871d23597b82caf88fd611454a2ddefb8"
        || document.source_definitions_sha256.as_deref()
            != Some(capture.source_definitions_sha256.as_str())
        || capture.records.len() != COVERED_KEYS.len()
        || capture.sources.len() != 9
        || !capture
            .records
            .iter()
            .map(|row| row.creature_key.as_str())
            .eq(COVERED_KEYS.iter().copied())
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
    fn actual_rotten_entry_vectors_keep_loaded_index_and_proved_empty_sources() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/creatures/definitions/spell-native-profiles.json");
        let raw = std::fs::read(path).expect("actual native content");
        let mut document: CreatureProfilesDocument =
            serde_json::from_slice(&raw).expect("native profiles");
        let policies = death_event_policies(&document, [7; 32]);
        assert_eq!(policies.len(), 6);
        for policy in &policies {
            let expected = match policy.creature().key.as_str() {
                "oteryn:creature.bakragore" => Some("BakragoreMechanicsDeath"),
                "oteryn:creature.chagorz" => Some("ChagorzMechanicsDeath"),
                "oteryn:creature.vemiath" => Some("VemiathMechanicsDeath"),
                _ => None,
            };
            assert_eq!(policy.events().len(), usize::from(expected.is_some()));
            if let Some(expected) = expected {
                assert_eq!(policy.events()[0].name(), expected);
                assert_eq!(policy.events()[0].registration_index(), 1);
            }
        }
        assert!(!requires_native_death_events(
            "oteryn:creature.elder_bloodjaw"
        ));
        assert!(death_event_policies(&document, [0; 32]).is_empty());
        document.source_definitions_sha256 = Some("0".repeat(64));
        assert!(death_event_policies(&document, [7; 32]).is_empty());
    }

    #[test]
    fn changed_source_registration_or_empty_vector_cannot_gain_native_admission() {
        assert!(qualified_capture(RAW).is_some());
        let mut value: serde_json::Value = serde_json::from_slice(RAW).expect("capture");
        value["records"][0]["on_death"] = serde_json::json!([]);
        assert!(qualified_capture(&serde_json::to_vec(&value).expect("changed capture")).is_none());
        let mut value: serde_json::Value = serde_json::from_slice(RAW).expect("capture");
        value["records"][0]["on_death"][0]["registration_index"] = serde_json::json!(0);
        assert!(qualified_capture(&serde_json::to_vec(&value).expect("changed capture")).is_none());
    }
}
