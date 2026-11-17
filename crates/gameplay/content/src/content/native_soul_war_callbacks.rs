//! Exact bounded W11 source closure, shared by Content templates and native consumers.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceWitness {
    schema: String,
    mechanics_source_revision: String,
    source_definitions_sha256: String,
    source_bindings: Vec<crate::content::ProjectV2SourceIdentityBinding>,
    callbacks: std::collections::BTreeMap<String, String>,
    records: Vec<crate::content::native_gameplay::CreatureProfileRecord>,
}

fn source_witness() -> Option<&'static SourceWitness> {
    static WITNESS: std::sync::OnceLock<Option<SourceWitness>> = std::sync::OnceLock::new();
    WITNESS
        .get_or_init(|| {
            serde_json::from_slice(include_bytes!(
                "../../../../../tools/content-schema/native-gameplay/srx3-soul-war-witness.json"
            ))
            .ok()
        })
        .as_ref()
}

pub(super) fn callback_source_sha(path: &str) -> Option<&'static str> {
    source_witness()?.callbacks.get(path).map(String::as_str)
}

pub fn qualifies_document(
    document: &crate::content::native_gameplay::CreatureProfilesDocument,
) -> bool {
    use crate::content::ProjectV2SourceIdentityDisposition;
    let Some(witness) = source_witness() else {
        return false;
    };
    if witness.schema != "OTERYN_SRX3_SOUL_WAR_CALLBACK_WITNESS/v1"
        || witness.mechanics_source_revision != "201189f871d23597b82caf88fd611454a2ddefb8"
        || document.source_definitions_sha256.as_deref()
            != Some(witness.source_definitions_sha256.as_str())
        || witness.source_bindings.len() != 52
        || witness.records.len() != 52
        || witness.callbacks.len() != 56
    {
        return false;
    }
    // The bounded witness is generated from the real canonical native inputs,
    // bound to the loader's full canonical source hash. Every consumed record
    // includes its exact Creature, Behavior and complete attack closure; this is
    // implementation source data, never a manufactured WorldProject or grant.
    witness.records.iter().all(|expected| {
        let mut matches = document
            .records
            .iter()
            .filter(|record| record.profile.target == expected.profile.target);
        matches.next() == Some(expected) && matches.next().is_none()
    }) && witness.source_bindings.iter().all(|binding| {
        binding.target.revision == "definition-r1"
            && binding.source_key == "oteryn:source.canary"
            && binding.source_revision == "47dfd51f45280a59a1d3e50ba7edd573d7234446"
            && binding.identity_namespace == "canary/monster-file"
            && binding.disposition == ProjectV2SourceIdentityDisposition::Exact
            && witness
                .records
                .iter()
                .any(|record| record.profile.target == binding.target)
    })
}

/// Owner ANSWER6839182665 selects retained Fandom page94059/revision1002465,
/// SHA0a78bf485aeef1dd4eeeec5f487ae513e14ce2203a45b1173e0280eba6cfc79b,
/// line6 raw `15.000`, as15000. Raw compiled15 and donor10000 remain evidence.
pub const SOUL_SPHERE_MAXIMUM_HEALTH: i64 = 15_000;

pub(super) fn effective_creature_health(
    document: &crate::content::native_gameplay::CreatureProfilesDocument,
    record: &crate::content::native_gameplay::CreatureProfileRecord,
    raw_health: i64,
) -> Result<i64, crate::content::ContentError> {
    if record.profile.target.key != "oteryn:creature.soul_sphere" {
        return Ok(raw_health);
    }
    // Full witness equality binds the exact canonical source definition hash,
    // source revision, Creature/Behavior/attacks and their immutable raw values.
    // A new source/revision/raw health needs a new explicit qualification.
    if record.profile.target.revision != "definition-r1"
        || raw_health != 15
        || !qualifies_document(document)
    {
        return Err(crate::content::ContentError::InvalidArtifact(
            "Soul Sphere accepted source health projection",
        ));
    }
    Ok(SOUL_SPHERE_MAXIMUM_HEALTH)
}

#[cfg(test)]
mod accepted_sphere_health_tests {
    use super::*;
    use crate::content::ContentError;

    fn document() -> Result<crate::content::native_gameplay::CreatureProfilesDocument, ContentError>
    {
        let witness =
            source_witness().ok_or(ContentError::InvalidArtifact("actual pinned witness"))?;
        Ok(crate::content::native_gameplay::CreatureProfilesDocument {
            schema: "OTERYN_NATIVE_CREATURE_PROFILES/v1".into(),
            records: witness.records.clone(),
            source_definitions_sha256: Some(witness.source_definitions_sha256.clone()),
            prey_rules: None,
        })
    }

    #[test]
    fn selected_sphere_health_requires_exact_raw_source_record() -> Result<(), ContentError> {
        let original = document()?;
        let index = original
            .records
            .iter()
            .position(|record| record.profile.target.key == "oteryn:creature.soul_sphere")
            .ok_or(ContentError::InvalidArtifact("actual Soul Sphere record"))?;
        assert_eq!(
            effective_creature_health(&original, &original.records[index], 15)?,
            15_000
        );
        for raw in [0, 14, 16, 10_000, 15_000] {
            assert!(effective_creature_health(&original, &original.records[index], raw).is_err());
        }
        let mut changed = original.clone();
        changed.records[index].profile.target.revision = "definition-r2".into();
        assert!(effective_creature_health(&changed, &changed.records[index], 15).is_err());
        let mut changed = original.clone();
        changed.source_definitions_sha256 = Some("00".repeat(32));
        assert!(effective_creature_health(&changed, &changed.records[index], 15).is_err());
        let mut changed = original.clone();
        let crate::content::ProjectV2AuthoringProfileData::Creature(profile) =
            &mut changed.records[index].profile.data
        else {
            return Err(ContentError::InvalidArtifact("actual Creature"));
        };
        profile.health = Some(10_000);
        assert!(effective_creature_health(&changed, &changed.records[index], 15).is_err());
        assert_eq!(
            effective_creature_health(&original, &original.records[0], 550_000)?,
            550_000
        );
        Ok(())
    }
}

impl crate::content::native_gameplay::NativeGameplayState {
    /// Review-engine DATA only. These declarations are not production event
    /// attachments; only an explicit test fixture may install this sealed tag.
    #[cfg(any(test, feature = "test-support"))]
    pub fn declared_w01_prepare_death_for_review(
        &self,
        key: &str,
        revision: &str,
    ) -> Option<QualifiedSoulWarHealthPolicy> {
        w01_prepare_death::policies(self.creature_profiles(), self.source_digest())
            .into_iter()
            .find(|policy| policy.matches(self.source_digest(), key, revision))
    }
    pub fn qualifies_soul_war_callbacks(&self, active: [u8; 32]) -> bool {
        if self.source_digest() != active {
            return false;
        }
        let Some(witness) = source_witness() else {
            return false;
        };
        if self
            .creature_profiles()
            .source_definitions_sha256
            .as_deref()
            != Some(witness.source_definitions_sha256.as_str())
        {
            return false;
        }
        // This sealed token is issued only by health_policies after the full
        // exact-record/source-binding witness check. Both this document and its
        // compiled table belong to the same immutable NativeGameplayState
        // generation; no caller-provided boolean or mutable cache can issue it.
        // The loader still performs full validation before actor admission.
        const KEY: &str = "oteryn:creature.mirror_image";
        self.policies()
            .soul_war_health_policy(KEY, "definition-r1")
            .is_some_and(|policy| {
                policy.kind() == SoulWarHealthCallbackKind::MirrorImage
                    && policy.matches(active, KEY, "definition-r1")
            })
    }
}

/// Immutable event registration DATA, never permission to mutate an actor.
#[cfg(any(test, feature = "test-support"))]
mod w01_prepare_death;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoulWarHealthCallbackKind {
    MirrorImage,
    FourthTaint,
    HirintrorHeal,
    WhitePaleHeal,
    /// Source callback changes the attacking Player through its existing HP owner.
    PassiveHealth,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedSoulWarHealthPolicy {
    digest: [u8; 32],
    key: String,
    revision: String,
    kind: SoulWarHealthCallbackKind,
}
impl QualifiedSoulWarHealthPolicy {
    pub fn kind(&self) -> SoulWarHealthCallbackKind {
        self.kind
    }
    /// Literal Soul War mechanics.lua:15-30,37-52. The full native body and
    /// pinned callback closure qualified this immutable target before issue.
    pub fn is_cruelty(&self) -> bool {
        self.kind == SoulWarHealthCallbackKind::PassiveHealth
            && self.key == "oteryn:creature.goshnar_s_cruelty"
    }
    /// Exact effective GoshnarsHatredBuff registration on Hatred and all three
    /// Megalomania native forms; no name-prefix or mutable percentage grants it.
    pub fn is_empowered_boss(&self) -> bool {
        self.kind == SoulWarHealthCallbackKind::PassiveHealth
            && matches!(
                self.key.as_str(),
                "oteryn:creature.goshnar_s_hatred"
                    | "oteryn:creature.goshnar_s_megalomania_purple"
                    | "oteryn:creature.goshnar_s_megalomania_green"
                    | "oteryn:creature.goshnar_s_megalomania_blue"
            )
    }
    pub fn lua_reflection_percent(&self, element: Option<&str>) -> u32 {
        if self.kind != SoulWarHealthCallbackKind::PassiveHealth {
            return 0;
        }
        match self.key.as_str() {
            "oteryn:creature.goshnar_s_malice" if matches!(element, Some("physical" | "death")) => {
                100
            }
            "oteryn:creature.soul_cage" => 10,
            _ => 0,
        }
    }
    pub fn matches(&self, digest: [u8; 32], key: &str, revision: &str) -> bool {
        self.digest == digest && self.key == key && self.revision == revision
    }
    pub(crate) fn bind_outer(&mut self, digest: [u8; 32]) {
        self.digest = digest;
    }
}

const FOURTH_TAINT: [&str; 14] = [
    "bony_sea_devil",
    "brachiodemon",
    "branchy_crawler",
    "capricious_phantom",
    "distorted_phantom",
    "cloak_of_terror",
    "courage_leech",
    "vibrant_phantom",
    "infernal_demon",
    "infernal_phantom",
    "many_faces",
    "mould_phantom",
    "rotten_golem",
    "turbulent_elemental",
];
/// Reserved donor identities cannot silently lose their registered callbacks
/// when a content revision no longer matches the reviewed source closure.
pub fn requires_health_policy(key: &str) -> bool {
    matches!(
        key,
        "oteryn:creature.mirror_image"
            | "oteryn:creature.goshnar_s_malice"
            | "oteryn:creature.soul_cage"
            | "oteryn:creature.goshnar_s_cruelty"
            | "oteryn:creature.goshnar_s_hatred"
            | "oteryn:creature.goshnar_s_megalomania_purple"
            | "oteryn:creature.goshnar_s_megalomania_green"
            | "oteryn:creature.goshnar_s_megalomania_blue"
    ) || key
        .strip_prefix("oteryn:creature.")
        .is_some_and(|slug| FOURTH_TAINT.contains(&slug))
}
pub(crate) fn health_policies(
    document: &crate::content::native_gameplay::CreatureProfilesDocument,
    digest: [u8; 32],
) -> Vec<QualifiedSoulWarHealthPolicy> {
    // Declared Hirintror/White Pale events are absent from both pinned monster
    // registration lists. Production preserves their inactive source behavior.
    soul_war_health_policies(document, digest)
}
fn soul_war_health_policies(
    document: &crate::content::native_gameplay::CreatureProfilesDocument,
    digest: [u8; 32],
) -> Vec<QualifiedSoulWarHealthPolicy> {
    if !qualifies_document(document) {
        return Vec::new();
    }
    document
        .records
        .iter()
        .filter_map(|record| {
            let target = &record.profile.target;
            let kind = if target.key == "oteryn:creature.mirror_image" {
                SoulWarHealthCallbackKind::MirrorImage
            } else if target
                .key
                .strip_prefix("oteryn:creature.")
                .is_some_and(|slug| FOURTH_TAINT.contains(&slug))
            {
                SoulWarHealthCallbackKind::FourthTaint
            } else if matches!(
                target.key.as_str(),
                "oteryn:creature.goshnar_s_malice"
                    | "oteryn:creature.soul_cage"
                    | "oteryn:creature.goshnar_s_cruelty"
                    | "oteryn:creature.goshnar_s_hatred"
                    | "oteryn:creature.goshnar_s_megalomania_purple"
                    | "oteryn:creature.goshnar_s_megalomania_green"
                    | "oteryn:creature.goshnar_s_megalomania_blue"
            ) {
                SoulWarHealthCallbackKind::PassiveHealth
            } else {
                return None;
            };
            Some(QualifiedSoulWarHealthPolicy {
                digest,
                key: target.key.clone(),
                revision: target.revision.clone(),
                kind,
            })
        })
        .collect()
}
impl crate::content::native_gameplay::NativeGameplayState {
    pub fn soul_war_health_policy(
        &self,
        reference: &crate::content::ProjectV2DefinitionRef,
    ) -> Option<&QualifiedSoulWarHealthPolicy> {
        if reference.family != crate::content::ProjectV2Family::Creature {
            return None;
        }
        self.policies()
            .soul_war_health_policy(&reference.key, &reference.revision)
    }
}

#[cfg(test)]
mod health_tests {
    #![allow(clippy::expect_used, clippy::panic)]
    use super::*;
    #[test]
    fn srx3_soul_war_health_callbacks_require_exact_full_native_closure_and_last_event_assignment()
    {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/creatures/definitions/spell-native-profiles.json");
        let raw = std::fs::read(path).expect("actual native content checkout");
        let mut document: crate::content::native_gameplay::CreatureProfilesDocument =
            serde_json::from_slice(&raw).expect("actual native source document");
        let policies = health_policies(&document, [7; 32]);
        assert_eq!(
            policies
                .iter()
                .filter(|p| matches!(
                    p.kind(),
                    SoulWarHealthCallbackKind::MirrorImage | SoulWarHealthCallbackKind::FourthTaint
                ))
                .count(),
            15
        );
        let mirror = policies
            .iter()
            .find(|policy| policy.key == "oteryn:creature.mirror_image")
            .expect("actual Mirror callback");
        assert_eq!(mirror.kind(), SoulWarHealthCallbackKind::MirrorImage);
        assert!(mirror.matches([7; 32], "oteryn:creature.mirror_image", "definition-r1"));
        assert!(!mirror.matches([8; 32], "oteryn:creature.mirror_image", "definition-r1"));
        assert!(!mirror.matches([7; 32], "oteryn:creature.mirror_image", "different"));
        assert_eq!(
            policies
                .iter()
                .filter(|p| p.kind() == SoulWarHealthCallbackKind::FourthTaint)
                .count(),
            14
        );
        assert!(
            !policies
                .iter()
                .any(|p| p.key == "oteryn:creature.goshnar_s_greed")
        );
        let target = document
            .records
            .iter_mut()
            .find(|r| r.profile.target.key == "oteryn:creature.mirror_image")
            .expect("source Mirror");
        let crate::content::ProjectV2AuthoringProfileData::Creature(profile) =
            &mut target.profile.data
        else {
            panic!("actual source Mirror must be a Creature profile");
        };
        profile.health = profile.health.map(|health| health + 1);
        assert!(
            health_policies(&document, [7; 32])
                .iter()
                .all(|p| !matches!(
                    p.kind(),
                    SoulWarHealthCallbackKind::MirrorImage | SoulWarHealthCallbackKind::FourthTaint
                )),
            "same name/root stamp cannot bless altered native source body"
        );
        assert_eq!(
            health_policies(&document, [7; 32])
                .iter()
                .filter(|p| matches!(
                    p.kind(),
                    SoulWarHealthCallbackKind::HirintrorHeal
                        | SoulWarHealthCallbackKind::WhitePaleHeal
                ))
                .count(),
            0,
            "production cannot attach unregistered W01 declarations"
        );
        // Canary47df raids/hirintror.lua:1-128 and white_pale.lua:1-119
        // never attach these events. Their declarations remain review-only.
        assert_eq!(
            w01_prepare_death::policies(&document, [7; 32])
                .iter()
                .filter(|p| matches!(
                    p.kind(),
                    SoulWarHealthCallbackKind::HirintrorHeal
                        | SoulWarHealthCallbackKind::WhitePaleHeal
                ))
                .count(),
            2,
            "an unrelated changed Mirror body cannot disable independently qualified review declarations"
        );
    }
}
