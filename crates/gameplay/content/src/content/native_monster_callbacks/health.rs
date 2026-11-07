//! Attached W01 HEALTHCHANGE source data. It grants no current HP authority.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeHealthEventKind {
    HealthForgotten,
    RuptureHeal,
    SoulcatcherSummon,
}

pub fn requires_native_health_event(key: &str) -> bool {
    matches!(
        key,
        "oteryn:creature.lady_tenebris"
            | "oteryn:creature.mounted_thorn_knight"
            | "oteryn:creature.the_enraged_thorn_knight"
            | "oteryn:creature.the_shielded_thorn_knight"
            | "oteryn:creature.rupture"
            | "oteryn:creature.soulcatcher"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedNativeHealthEvent {
    parent: Ref,
    content: [u8; 32],
    kind: NativeHealthEventKind,
    registration_index: u16,
}
impl QualifiedNativeHealthEvent {
    pub(crate) fn bind_outer(&mut self, digest: [u8; 32]) {
        self.content = digest;
    }
    pub fn kind(&self) -> NativeHealthEventKind {
        self.kind
    }
    pub fn registration_index(&self) -> u16 {
        self.registration_index
    }
    pub fn matches(&self, digest: [u8; 32], key: &str, revision: &str) -> bool {
        self.content == digest && self.parent.key == key && self.parent.revision == revision
    }
}

pub(crate) fn policies(
    document: &CreatureProfilesDocument,
    content: [u8; 32],
) -> Vec<QualifiedNativeHealthEvent> {
    document
        .records
        .iter()
        .filter_map(|record| {
            qualify(document, content, &record.profile.target)
                .ok()
                .flatten()
        })
        .collect()
}

impl NativeGameplayState {
    pub fn health_event_source(
        &self,
        parent: &Ref,
    ) -> Result<Option<QualifiedNativeHealthEvent>, NativeCallbackError> {
        qualify(self.creature_profiles(), self.source_digest(), parent)
    }
}

fn qualify(
    document: &CreatureProfilesDocument,
    content: [u8; 32],
    parent: &Ref,
) -> Result<Option<QualifiedNativeHealthEvent>, NativeCallbackError> {
    use NativeHealthEventKind as K;
    let (kind, name, path, hash) = match parent.key.as_str() {
        "oteryn:creature.lady_tenebris"
        | "oteryn:creature.mounted_thorn_knight"
        | "oteryn:creature.the_enraged_thorn_knight"
        | "oteryn:creature.the_shielded_thorn_knight" => (
            K::HealthForgotten,
            "HealthForgotten",
            "data-otservbr-global/scripts/quests/forgotten_knowledge/creaturescripts_healthchange_forgotten.lua",
            "3bf777975003746480e6fa7c1da5fc0b2351e61a7602c49076c9b1d44114cde7",
        ),
        "oteryn:creature.rupture" => (
            K::RuptureHeal,
            "RuptureHeal",
            "data-otservbr-global/scripts/quests/heart_of_destruction/creaturescripts_rupture_heal.lua",
            "93a333558e110b20234d961a18f351b38ca9d15a24bf57a8c0d6ba9785b3dbe5",
        ),
        "oteryn:creature.soulcatcher" => (
            K::SoulcatcherSummon,
            "SoulcatcherSummon",
            "data-otservbr-global/scripts/quests/forgotten_knowledge/creaturescripts_soulcatcher_summon.lua",
            "4a88ea5b5b51a184f90ff45eb30fc8e0e3de3a2551984f1846c48e174a149e1d",
        ),
        _ => return Ok(None),
    };
    native_callback_creature_document(document, parent)?;
    let witness = witness()?;
    let mut events = witness
        .source_events
        .iter()
        .filter(|event| event.name == name);
    let event = events.next().ok_or(NativeCallbackError::InvalidSource)?;
    if events.next().is_some()
        || event.repository != "opentibiabr/canary"
        || event.revision != "47dfd51f45280a59a1d3e50ba7edd573d7234446"
        || event.path != path
        || event.sha256 != hash
    {
        return Err(NativeCallbackError::InvalidSource);
    }
    let registration_index =
        crate::content::native_w01_death_events::registered_event_index(document, parent, name)
            .ok_or(NativeCallbackError::InvalidSource)?;
    // Canary game.cpp:8365-8378,8465-8478 invokes the target's attached
    // HEALTHCHANGE vector independently of its own attack schedule. The exact
    // native Creature body, literal registration and loaded Lua body above are
    // the relevant closure; this token grants no unrelated Ability execution.
    Ok(Some(QualifiedNativeHealthEvent {
        parent: parent.clone(),
        content,
        kind,
        registration_index,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_w01_health_events_require_literal_native_registration_and_body()
    -> Result<(), Box<dyn std::error::Error>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), String> {
                let world = crate::content::native_gameplay_test_world_id()
                    .map_err(|e| format!("{e:?}"))?;
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../content/spells.manifest.json");
                let native = crate::content::native_gameplay::retained_callback_native_fixture_at(
                    world, &path,
                );
                for (key, kind) in [
                    ("lady_tenebris", NativeHealthEventKind::HealthForgotten),
                    (
                        "mounted_thorn_knight",
                        NativeHealthEventKind::HealthForgotten,
                    ),
                    (
                        "the_enraged_thorn_knight",
                        NativeHealthEventKind::HealthForgotten,
                    ),
                    (
                        "the_shielded_thorn_knight",
                        NativeHealthEventKind::HealthForgotten,
                    ),
                    ("rupture", NativeHealthEventKind::RuptureHeal),
                    ("soulcatcher", NativeHealthEventKind::SoulcatcherSummon),
                ] {
                    let parent = Ref {
                        family: crate::content::ProjectV2Family::Creature,
                        key: format!("oteryn:creature.{key}"),
                        revision: "definition-r1".into(),
                    };
                    assert!(
                        native
                            .creature_profiles()
                            .records
                            .iter()
                            .any(|record| record.profile.target == parent)
                    );
                    let proof = native
                        .health_event_source(&parent)
                        .map_err(|e| format!("{e:?}"))?
                        .ok_or_else(|| {
                            format!("missing actual registered HEALTHCHANGE for {}", parent.key)
                        })?;
                    assert_eq!(proof.kind(), kind);
                    assert_eq!(native.policies().native_health_event(&parent.key, &parent.revision), Some(&proof), "activation must bind the same qualified event into the actual runtime policy table");
                    assert_eq!(
                        proof.registration_index(),
                        if kind == NativeHealthEventKind::SoulcatcherSummon {
                            0
                        } else {
                            1
                        }
                    );
                    assert!(proof.matches(native.source_digest(), &parent.key, &parent.revision));
                    assert!(!proof.matches([0; 32], &parent.key, &parent.revision));
                    assert!(!proof.matches(native.source_digest(), &parent.key, "wrong-revision"));
                }
                let document = native.creature_profiles();
                let parent = document
                    .records
                    .iter()
                    .find(|record| record.profile.target.key == "oteryn:creature.lady_tenebris")
                    .ok_or("actual Lady Tenebris source")?
                    .profile
                    .target
                    .clone();
                let mut changed = document.clone();
                let other = document
                    .records
                    .iter()
                    .find(|record| record.profile.target.key == "oteryn:creature.rupture")
                    .ok_or("actual alternate native creature body")?
                    .profile
                    .data
                    .clone();
                let record = changed
                    .records
                    .iter_mut()
                    .find(|record| record.profile.target == parent)
                    .ok_or("actual original source slot")?;
                record.profile.data = other;
                assert!(
                    qualify(&changed, native.source_digest(), &parent).is_err(),
                    "unchanged root cannot bless changed native body"
                );
                let mut wrong_family = parent.clone();
                wrong_family.family = crate::content::ProjectV2Family::Ability;
                assert!(qualify(document, native.source_digest(), &wrong_family).is_err());
                let mut duplicated = document.clone();
                duplicated.records.push(
                    document
                        .records
                        .iter()
                        .find(|record| record.profile.target == parent)
                        .ok_or("real parent duplicate control")?
                        .clone(),
                );
                assert!(qualify(&duplicated, native.source_digest(), &parent).is_err());
                let unrelated = Ref {
                    key: "oteryn:creature.gaz_haragoth".into(),
                    ..parent
                };
                assert!(
                    native
                        .health_event_source(&unrelated)
                        .map_err(|e| format!("{e:?}"))?
                        .is_none(),
                    "actual different registered event is not a HEALTHCHANGE alias"
                );
                Ok(())
            })?
            .join()
            .map_err(|_| "native W01 health source worker panicked")?
            .map_err(Into::into)
    }
}
