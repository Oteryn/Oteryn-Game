//! W11-only source callback schedule ownership; no casting-flow authority.
use super::native_gameplay::{CreatureProfileRecord, NativeGameplayState};
use super::{
    ProjectV2AbilityAuthoring, ProjectV2AbilitySchedule, ProjectV2AuthoringProfile,
    ProjectV2AuthoringProfileData as Data, ProjectV2BehaviorAuthoring, ProjectV2DefinitionRef,
    ProjectV2SourceIdentityBinding,
};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum QualifiedSoulWarAttackKind {
    CrueltyElemental,
    MegalomaniaElemental,
    MegalomaniaBlue,
}

/// Construction remains private to the pinned W11 source qualifier. These are
/// complete source schedules, including the original native entries in order.
#[derive(Debug, Clone)]
pub struct QualifiedSoulWarSourceAttacks {
    behavior: ProjectV2BehaviorAuthoring,
    abilities: BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
    kind: QualifiedSoulWarAttackKind,
}
impl QualifiedSoulWarSourceAttacks {
    pub fn behavior(&self) -> &ProjectV2BehaviorAuthoring {
        &self.behavior
    }
    pub fn abilities(&self) -> &BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring> {
        &self.abilities
    }
    pub fn accepts_ability(&self, ability: &ProjectV2DefinitionRef) -> bool {
        self.abilities.contains_key(ability)
    }
    pub fn kind(&self, ability: &ProjectV2DefinitionRef) -> Option<QualifiedSoulWarAttackKind> {
        self.accepts_ability(ability).then_some(self.kind)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceAttackRecord {
    native: CreatureProfileRecord,
    source_binding: ProjectV2SourceIdentityBinding,
    index: usize,
    kind: QualifiedSoulWarAttackKind,
    schedule: ProjectV2AbilitySchedule,
    ability: ProjectV2AuthoringProfile,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceAttackWitness {
    schema: String,
    source_revision: String,
    source_definitions_sha256: String,
    callbacks: BTreeMap<String, String>,
    records: Vec<SourceAttackRecord>,
}
fn source_witness() -> Option<&'static SourceAttackWitness> {
    static SOURCE: std::sync::OnceLock<Option<SourceAttackWitness>> = std::sync::OnceLock::new();
    SOURCE
        .get_or_init(|| {
            let source: SourceAttackWitness = serde_json::from_slice(include_bytes!(
                "../../../../../tools/content-schema/native-gameplay/srx3-soul-war-attacks.json"
            ))
            .ok()?;
            (source.schema == "OTERYN_SOUL_WAR_SOURCE_ATTACK_OVERLAY/v1"
                && source.source_revision == "201189f871d23597b82caf88fd611454a2ddefb8"
                && source.records.len() == 4
                && source.callbacks.len() == 8)
                .then_some(source)
        })
        .as_ref()
}

impl NativeGameplayState {
    /// Descriptive implementation data only: projection and physical execution
    /// must independently fence their current Instance, actor, map and content.
    pub fn soul_war_source_attacks(
        &self,
        creature: &ProjectV2DefinitionRef,
    ) -> Option<QualifiedSoulWarSourceAttacks> {
        let source = source_witness()?;
        if self
            .creature_profiles()
            .source_definitions_sha256
            .as_deref()
            != Some(source.source_definitions_sha256.as_str())
        {
            return None;
        }
        let mut expected = source
            .records
            .iter()
            .filter(|row| &row.native.profile.target == creature);
        let row = expected.next()?;
        if expected.next().is_some() {
            return None;
        }
        let mut current = self
            .creature_profiles()
            .records
            .iter()
            .filter(|row| &row.profile.target == creature);
        if current.next()? != &row.native || current.next().is_some() {
            return None;
        }
        let (file, index, interval, chance, kind) = match creature.key.as_str() {
            "oteryn:creature.goshnar_s_cruelty" => (
                "goshnars_cruelty",
                5,
                7000,
                500_000,
                QualifiedSoulWarAttackKind::CrueltyElemental,
            ),
            "oteryn:creature.goshnar_s_megalomania_green" => (
                "goshnar's_megalomania_green",
                7,
                7000,
                500_000,
                QualifiedSoulWarAttackKind::MegalomaniaElemental,
            ),
            "oteryn:creature.goshnar_s_megalomania_purple" => (
                "goshnar's_megalomania_purple",
                7,
                7000,
                500_000,
                QualifiedSoulWarAttackKind::MegalomaniaElemental,
            ),
            "oteryn:creature.goshnar_s_megalomania_blue" => (
                "goshnar's_megalomania_blue",
                1,
                6000,
                1_000_000,
                QualifiedSoulWarAttackKind::MegalomaniaBlue,
            ),
            _ => return None,
        };
        if row.index != index
            || row.kind != kind
            || row.schedule.interval_ms != interval
            || row.schedule.chance_ppm != chance
            || row.schedule.magnitude.is_some()
            || row.schedule.range_tiles.is_some()
            || row.schedule.ability != row.ability.target
            || row.source_binding.disposition != super::ProjectV2SourceIdentityDisposition::Exact
            || row.source_binding.target != *creature
            || row.source_binding.source_key != "oteryn:source.canary"
            || row.source_binding.source_revision != "47dfd51f45280a59a1d3e50ba7edd573d7234446"
            || row.source_binding.identity_namespace != "canary/monster-file"
            || row.source_binding.external_id != format!("quests/soul_war/{file}")
        {
            return None;
        }
        let Data::Behavior(mut behavior) = row.native.behavior.as_ref()?.data.clone() else {
            return None;
        };
        let Data::Ability(ability) = &row.ability.data else {
            return None;
        };
        let expected: ProjectV2AbilityAuthoring = serde_json::from_value(serde_json::json!({
            "details": {"kind": "Spell", "range_tiles": 0, "needs_target": false,
                "needs_direction": kind != QualifiedSoulWarAttackKind::MegalomaniaBlue}
        }))
        .ok()?;
        if ability != &expected
            || row.ability.target.family != super::ProjectV2Family::Ability
            || row.ability.target.revision != "crystal-201189f"
            || row.ability.target.key
                != format!(
                    "oteryn:ability.srx3.soul-war.{}",
                    creature.key.strip_prefix("oteryn:creature.")?
                )
        {
            return None;
        }
        if behavior
            .attacks
            .iter()
            .any(|entry| entry.ability.key == row.schedule.ability.key)
            || row
                .native
                .attack_definitions
                .as_ref()?
                .profiles
                .iter()
                .any(|profile| profile.target.key == row.schedule.ability.key)
            || index > behavior.attacks.len()
        {
            return None;
        }
        behavior.attacks.insert(index, row.schedule.clone());
        Some(QualifiedSoulWarSourceAttacks {
            behavior,
            abilities: BTreeMap::from([(row.ability.target.clone(), ability.clone())]),
            kind,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn srx3_soul_war_missing_custom_attack_schedules_are_restored_to_actual_native_vectors()
    -> Result<(), Box<dyn std::error::Error>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), String> {
                let world = oteryn_protocol_oteryn::WorldId::decode(&[
                    1, 144, 0, 0, 0, 8, 112, 0, 128, 0, 0, 0, 0, 0, 0, 8,
                ])
                .map_err(|e| format!("{e:?}"))?;
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../content/spells.manifest.json");
                let native = super::super::native_gameplay::retained_callback_native_fixture_at(
                    world, &path,
                );
                for (slug, index, interval, chance) in [
                    ("goshnar_s_cruelty", 5, 7_000, 500_000),
                    ("goshnar_s_megalomania_green", 7, 7_000, 500_000),
                    ("goshnar_s_megalomania_purple", 7, 7_000, 500_000),
                    ("goshnar_s_megalomania_blue", 1, 6_000, 1_000_000),
                ] {
                    let key = format!("oteryn:creature.{slug}");
                    let record = native
                        .creature_profiles()
                        .records
                        .iter()
                        .find(|r| r.profile.target.key == key)
                        .ok_or("actual source creature")?;
                    let overlay = native
                        .soul_war_source_attacks(&record.profile.target)
                        .ok_or_else(|| format!("actual native source omits {slug} callback"))?;
                    let super::super::ProjectV2AuthoringProfileData::Behavior(before) = &record
                        .behavior
                        .as_ref()
                        .ok_or("actual native behavior")?
                        .data
                    else {
                        return Err("actual behavior kind".into());
                    };
                    let mut restored = overlay.behavior().attacks.clone();
                    let added = restored.remove(index);
                    assert_eq!(
                        restored, before.attacks,
                        "all original source slots retain their order"
                    );
                    assert_eq!((added.interval_ms, added.chance_ppm), (interval, chance));
                    assert!(overlay.accepts_ability(&added.ability));
                    assert_eq!(overlay.abilities().len(), 1);
                }
                Ok(())
            })?
            .join()
            .map_err(|_| "source schedule test thread panicked")??;
        Ok(())
    }
}
