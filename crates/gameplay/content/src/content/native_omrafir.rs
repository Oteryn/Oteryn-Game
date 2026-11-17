//! Pinned Omrafir callback schedules omitted by the partial Lua authoring pass.
use super::native_gameplay::NativeGameplayState;
use super::{
    ProjectV2AbilityAuthoring, ProjectV2BehaviorAuthoring, ProjectV2CreatureAuthoring,
    ProjectV2DefinitionRef,
};
use std::collections::BTreeMap;
mod beam_stage;
mod events;
pub use beam_stage::QualifiedOmrafirBeamStage;
pub use events::{OmrafirEventBirth, QualifiedOmrafirEvents};

/// Descriptive source rules only; physical execution independently requires the
/// actual current actor, native content pin, source room and scheduler occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmrafirSourceCallbackKind {
    Beam,
    FireHeal,
}

#[derive(Debug, Clone)]
pub struct QualifiedOmrafirSourceAttacks {
    content: [u8; 32],
    creature: ProjectV2CreatureAuthoring,
    behavior: ProjectV2BehaviorAuthoring,
    abilities: BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
}
impl QualifiedOmrafirSourceAttacks {
    pub fn content(&self) -> [u8; 32] {
        self.content
    }
    pub fn kind(&self, ability: &ProjectV2DefinitionRef) -> Option<OmrafirSourceCallbackKind> {
        if !self.accepts_ability(ability) {
            return None;
        }
        match ability.key.as_str() {
            "oteryn:ability.spell.omrafir_beam" => Some(OmrafirSourceCallbackKind::Beam),
            "oteryn:ability.spell.omrafir_healing_2" => Some(OmrafirSourceCallbackKind::FireHeal),
            _ => None,
        }
    }
    pub fn beam_matrix(&self, source_direction: u8) -> Option<&'static [String]> {
        source_witness()?
            .beam_matrices
            .get(usize::from(source_direction))
            .map(Vec::as_slice)
    }
    pub fn creature(&self) -> &ProjectV2CreatureAuthoring {
        &self.creature
    }
    pub fn behavior(&self) -> &ProjectV2BehaviorAuthoring {
        &self.behavior
    }
    pub fn abilities(&self) -> &BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring> {
        &self.abilities
    }
    pub fn accepts_ability(&self, ability: &ProjectV2DefinitionRef) -> bool {
        self.abilities.contains_key(ability)
    }
}
impl NativeGameplayState {
    pub fn omrafir_source_attacks(
        &self,
        creature: &ProjectV2DefinitionRef,
    ) -> Option<QualifiedOmrafirSourceAttacks> {
        qualify_overlay(self, creature)
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OverlayWitness {
    schema: String,
    source_revision: String,
    source_definitions_sha256: String,
    native: super::native_gameplay::CreatureProfileRecord,
    source_binding: super::ProjectV2SourceIdentityBinding,
    callbacks: BTreeMap<String, String>,
    schedules: Vec<OverlaySchedule>,
    abilities: Vec<super::ProjectV2AuthoringProfile>,
    beam_matrices: Vec<Vec<String>>,
    // Read the old generated name only until Native11 replaces it with the
    // truthful exact_ids predicate; source IDs and behavior never change.
    #[serde(default)]
    healing_fire_items: Option<Vec<u16>>,
    beam_delay_ms: u32,
    beam_damage: SourceBeamDamage,
    healing: SourceExactItemHeal,
    regeneration_marker: RegenerationMarker,
    #[serde(default)]
    creature_events: Option<serde_json::Value>,
    #[serde(default)]
    event_children: Option<Vec<OmrafirEventChild>>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OmrafirEventChild {
    native: super::native_gameplay::CreatureProfileRecord,
    source_binding: super::ProjectV2SourceIdentityBinding,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OverlaySchedule {
    list: String,
    index: usize,
    schedule: super::ProjectV2AbilitySchedule,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RegenerationMarker {
    sub_id: u32,
    duration_ms: u32,
    health_gain: u32,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceBeamDamage {
    minimum: u32,
    maximum: u32,
    element: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceExactItemHeal {
    #[serde(default)]
    exact_ids: Option<Vec<u16>>,
    minimum: u32,
    maximum: u32,
    health_below_ppm: u32,
    source_name: String,
    require_tile: bool,
}
fn source_witness() -> Option<&'static OverlayWitness> {
    static WITNESS: std::sync::OnceLock<Option<OverlayWitness>> = std::sync::OnceLock::new();
    WITNESS
        .get_or_init(|| {
            let witness: OverlayWitness = serde_json::from_slice(include_bytes!(
                "../../../../../tools/content-schema/native-gameplay/srx3-omrafir-attacks.json"
            ))
            .ok()?;
            if witness.schema != "OTERYN_OMRAFIR_SOURCE_ATTACK_OVERLAY/v1"
                || witness.source_revision != "47dfd51f45280a59a1d3e50ba7edd573d7234446"
                || witness.schedules.len() != 2
                || witness.abilities.len() != 2
                || witness.callbacks.len() != 3
                || witness.beam_matrices.len() != 4
                || witness
                    .healing
                    .exact_ids
                    .as_deref()
                    .or(witness.healing_fire_items.as_deref())
                    != Some(&[1487, 1492, 1500])
                || witness.beam_delay_ms != 4000
                || witness.beam_damage.minimum != 7000
                || witness.beam_damage.maximum != 10000
                || witness.beam_damage.element != "fire"
                || witness.healing.minimum != 7500
                || witness.healing.maximum != 9000
                || witness.healing.health_below_ppm != 999900
                || witness.healing.source_name != "Omrafir"
                || !witness.healing.require_tile
                || witness.regeneration_marker.sub_id != 88888
                || witness.regeneration_marker.duration_ms != 5000
                || witness.regeneration_marker.health_gain != 0
            {
                return None;
            }
            // Event evidence is retained independently of the attack overlay.
            // It does not grant an event caller or a spawn capability here.
            match (&witness.creature_events, &witness.event_children) {
                (None, None) => {}
                (Some(events), Some(children)) => {
                    if events
                        .get("callback")
                        .and_then(|v| v.get("sha256"))
                        .and_then(|v| v.as_str())
                        != Some("bad7511af294bebc5a9d94345252d0ba03784172a73463d772be6350a467cbcf")
                        || children.len() != 1
                        || children[0].native.profile.target.key
                            != "oteryn:creature.hellfire_fighter"
                        || children[0].source_binding.target != children[0].native.profile.target
                    {
                        return None;
                    }
                }
                _ => return None,
            }
            for (file, sha) in [
                (
                    "data-otservbr-global/monster/quests/roshamuul/omrafir.lua",
                    "d5d600fab74a3e4f56277552b1538f03a46bfcb067443257e4d7120e541fb798",
                ),
                (
                    "data-otservbr-global/scripts/spells/monster/omrafir_beam.lua",
                    "f87b630bdca5cf7ffc6167bc9301833e955f39586c7ff151322e0ae011e3d4ab",
                ),
                (
                    "data-otservbr-global/scripts/spells/monster/omrafir_healing_2.lua",
                    "ba2790a574ae9deba86d903e60cf8271e4ad86e51141256d90ceebb679927036",
                ),
            ] {
                if witness.callbacks.get(file).map(String::as_str) != Some(sha) {
                    return None;
                }
            }
            for (direction, rows) in witness.beam_matrices.iter().enumerate() {
                if rows.len() != 11 {
                    return None;
                }
                for (y, row) in rows.iter().enumerate() {
                    if row.len() != 11 {
                        return None;
                    }
                    for (x, cell) in row.bytes().enumerate() {
                        let expected = match direction {
                            0 if (x, y) == (5, 0) => b'c',
                            0 if x == 5 && (1..=8).contains(&y) => b'x',
                            1 if (x, y) == (10, 5) => b'c',
                            1 if y == 5 && (2..=9).contains(&x) => b'x',
                            2 if (x, y) == (5, 9) => b'c',
                            2 if x == 5 && y <= 8 => b'x',
                            3 if (x, y) == (0, 5) => b'c',
                            3 if y == 5 && (1..=8).contains(&x) => b'x',
                            _ => b'.',
                        };
                        if cell != expected {
                            return None;
                        }
                    }
                }
            }
            Some(witness)
        })
        .as_ref()
}

fn qualify_overlay(
    native: &NativeGameplayState,
    creature: &ProjectV2DefinitionRef,
) -> Option<QualifiedOmrafirSourceAttacks> {
    use super::ProjectV2AuthoringProfileData as Data;
    let witness = source_witness()?;
    if native
        .creature_profiles()
        .source_definitions_sha256
        .as_deref()
        != Some(witness.source_definitions_sha256.as_str())
        || &witness.native.profile.target != creature
        || witness.source_binding.target != *creature
        || witness.source_binding.source_key != "oteryn:source.canary"
        || witness.source_binding.source_revision != witness.source_revision
        || witness.source_binding.identity_namespace != "canary/monster-file"
        || witness.source_binding.external_id != "quests/roshamuul/omrafir"
        || witness.source_binding.disposition != super::ProjectV2SourceIdentityDisposition::Exact
    {
        return None;
    }
    let mut actual = native
        .creature_profiles()
        .records
        .iter()
        .filter(|record| &record.profile.target == creature);
    if actual.next() != Some(&witness.native) || actual.next().is_some() {
        return None;
    }
    let Data::Creature(mut source_creature) = witness.native.profile.data.clone() else {
        return None;
    };
    let Data::Behavior(mut behavior) = witness.native.behavior.as_ref()?.data.clone() else {
        return None;
    };
    let mut abilities = BTreeMap::new();
    for (row, (list, key, chance)) in witness.schedules.iter().zip([
        ("attacks", "oteryn:ability.spell.omrafir_beam", 150_000),
        (
            "defenses",
            "oteryn:ability.spell.omrafir_healing_2",
            200_000,
        ),
    ]) {
        if row.list != list
            || row.index != 2
            || row.schedule.ability.key != key
            || row.schedule.ability.revision != "definition-r1"
            || row.schedule.ability.family != super::ProjectV2Family::Ability
            || row.schedule.interval_ms != 2000
            || row.schedule.chance_ppm != chance
            || row.schedule.range_tiles.is_some()
            || row.schedule.magnitude.is_some()
        {
            return None;
        }
        let mut matching = witness
            .abilities
            .iter()
            .filter(|profile| profile.target == row.schedule.ability);
        let profile = matching.next()?;
        if matching.next().is_some() {
            return None;
        }
        let Data::Ability(ability) = &profile.data else {
            return None;
        };
        let expected: ProjectV2AbilityAuthoring =
            serde_json::from_value(serde_json::json!({"details":{
                "kind":"Spell","range_tiles":0,"needs_target":false,"needs_direction":false,
            }}))
            .ok()?;
        if ability != &expected {
            return None;
        }
        let entries = if list == "attacks" {
            &mut behavior.attacks
        } else {
            &mut behavior.defenses
        };
        if entries
            .iter()
            .any(|entry| entry.ability == row.schedule.ability)
            || source_creature.abilities.contains(&row.schedule.ability)
            || witness
                .native
                .attack_definitions
                .as_ref()?
                .profiles
                .iter()
                .any(|body| body.target == row.schedule.ability)
            || row.index > entries.len()
        {
            return None;
        }
        entries.insert(row.index, row.schedule.clone());
        source_creature.abilities.push(row.schedule.ability.clone());
        if abilities
            .insert(profile.target.clone(), ability.clone())
            .is_some()
        {
            return None;
        }
    }
    source_creature.abilities.sort();
    Some(QualifiedOmrafirSourceAttacks {
        content: native.source_digest(),
        creature: source_creature,
        behavior,
        abilities,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_omrafir_missing_callbacks_restore_complete_source_vectors()
    -> Result<(), Box<dyn std::error::Error>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), String> {
                let world = oteryn_protocol_oteryn::WorldId::decode(&[
                    1, 144, 0, 0, 0, 8, 112, 0, 128, 0, 0, 0, 0, 0, 0, 8,
                ])
                .map_err(|error| format!("{error:?}"))?;
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../content/spells.manifest.json");
                let native = super::super::native_gameplay::retained_callback_native_fixture_at(
                    world, &path,
                );
                let record = native
                    .creature_profiles()
                    .records
                    .iter()
                    .find(|record| record.profile.target.key == "oteryn:creature.omrafir")
                    .ok_or("actual Omrafir native record")?;
                let super::super::ProjectV2AuthoringProfileData::Behavior(before) = &record
                    .behavior
                    .as_ref()
                    .ok_or("actual Omrafir native behavior")?
                    .data
                else {
                    return Err("behavior family".into());
                };
                let restored = native
                    .omrafir_source_attacks(&record.profile.target)
                    .ok_or("actual native source omits Omrafir beam and healing2 callbacks")?;
                // Canary 47dfd51f omrafir.lua:111-120,123-128. The third
                // source attack and third source defense retain their original slots.
                let mut attacks = restored.behavior().attacks.clone();
                let beam = attacks.remove(2);
                assert_eq!(
                    attacks, before.attacks,
                    "all existing attack slots retain exact source order"
                );
                assert_eq!(beam.ability.key, "oteryn:ability.spell.omrafir_beam");
                assert_eq!((beam.interval_ms, beam.chance_ppm), (2000, 150000));
                let mut defenses = restored.behavior().defenses.clone();
                let healing = defenses.remove(2);
                assert_eq!(
                    defenses, before.defenses,
                    "all existing defense slots retain exact source order"
                );
                assert_eq!(
                    healing.ability.key,
                    "oteryn:ability.spell.omrafir_healing_2"
                );
                assert_eq!((healing.interval_ms, healing.chance_ppm), (2000, 200000));
                assert!(restored.accepts_ability(&beam.ability));
                assert!(restored.accepts_ability(&healing.ability));
                assert_eq!(restored.abilities().len(), 2);
                let mut fingerprints = std::collections::BTreeSet::new();
                for direction in 0..4 {
                    let stage = restored
                        .beam_stage(direction)
                        .ok_or("sealed beam descriptor")?;
                    assert_eq!(stage.creature(), &record.profile.target);
                    assert_eq!(stage.ability(), &beam.ability);
                    assert_eq!(
                        (
                            stage.entry_index(),
                            stage.delay_ms(),
                            stage.minimum(),
                            stage.maximum()
                        ),
                        (2, 4000, 7000, 10000)
                    );
                    assert_eq!(stage.source_direction(), direction);
                    assert_eq!(stage.content(), native.source_digest());
                    let super::super::ProjectV2AbilityArea::Matrix { north, diagonal } =
                        stage.details().area.as_ref().ok_or("beam area")?
                    else {
                        return Err("beam matrix shape".into());
                    };
                    assert_eq!(
                        north,
                        restored
                            .beam_matrix(direction)
                            .ok_or("literal source matrix")?
                    );
                    assert!(diagonal.is_empty());
                    assert!(
                        !stage.details().needs_direction,
                        "literal selected matrix must not rotate again"
                    );
                    assert_eq!(
                        stage.details().impact_cue.as_deref(),
                        Some("canary.appearance:effect/firearea")
                    );
                    fingerprints.insert(stage.fingerprint());
                }
                assert_eq!(
                    fingerprints.len(),
                    4,
                    "source directions retain separate immutable descriptors"
                );
                assert!(restored.beam_stage(4).is_none());

                let super::super::ProjectV2AuthoringProfileData::Creature(before_creature) =
                    &record.profile.data
                else {
                    return Err("actual creature family".into());
                };
                let mut complete_creature = restored.creature().clone();
                assert!(complete_creature.abilities.contains(&beam.ability));
                assert!(complete_creature.abilities.contains(&healing.ability));
                complete_creature
                    .abilities
                    .retain(|ability| ability != &beam.ability && ability != &healing.ability);
                assert_eq!(
                    &complete_creature, before_creature,
                    "all original native stats and ability references remain exact"
                );
                let event_source = native
                    .omrafir_events_source(&record.profile.target)
                    .ok_or("exact source Omrafir/Hellfire event closure")?;
                let boom = event_source.spawn_source(OmrafirEventBirth::Hellfire);
                let reform = event_source.spawn_source(OmrafirEventBirth::NormalReform);
                assert!(boom.extended() && boom.forced());
                assert!(!reform.extended() && reform.forced());
                assert_eq!(boom.child(), event_source.hellfire());
                assert_eq!(reform.child(), &record.profile.target);
                assert_eq!(
                    event_source.terminal_source_outcome(),
                    "unknown-monster-nil-then-lua-error"
                );
                let mut changed = native.creature_profiles().clone();
                let child = changed
                    .records
                    .iter_mut()
                    .find(|row| row.profile.target == *event_source.hellfire())
                    .ok_or("actual Hellfire native record")?;
                let super::super::ProjectV2AuthoringProfileData::Creature(child) =
                    &mut child.profile.data
                else {
                    return Err("actual child family".into());
                };
                *child.health.as_mut().ok_or("actual child health")? += 1;
                assert!(
                    events::qualify_events_document(&native, &record.profile.target, &changed)
                        .is_none(),
                    "same canonical root cannot bless changed Hellfire event child"
                );
                Ok(())
            })?
            .join()
            .map_err(|_| "Omrafir source test thread panicked")??;
        Ok(())
    }
}
