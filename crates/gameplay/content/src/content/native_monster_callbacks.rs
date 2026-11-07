//! Closed source callback spawn descriptions. These are DATA, not current
//! scope, actor, map, occupancy or physical admission authority.
mod gaz;
mod health;
mod stages;
use super::ProjectV2DefinitionRef;
pub use gaz::QualifiedGazEvents;
pub(crate) use health::policies as health_event_policies;
pub use health::{NativeHealthEventKind, QualifiedNativeHealthEvent, requires_native_health_event};
pub use stages::{QualifiedNativeCallbackStage, source_callback_area};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCallbackList {
    Attack,
    Defence,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCallbackError {
    InvalidSource,
}

#[derive(Debug, Clone)]
pub struct QualifiedNativeCallbackSpawn {
    parent: ProjectV2DefinitionRef,
    ability: ProjectV2DefinitionRef,
    child: ProjectV2DefinitionRef,
    source_digest: [u8; 32],
    extended: bool,
    forced: bool,
}
impl QualifiedNativeCallbackSpawn {
    pub fn parent(&self) -> &ProjectV2DefinitionRef {
        &self.parent
    }
    pub fn ability(&self) -> &ProjectV2DefinitionRef {
        &self.ability
    }
    pub fn child(&self) -> &ProjectV2DefinitionRef {
        &self.child
    }
    pub fn source_digest(&self) -> [u8; 32] {
        self.source_digest
    }
    pub fn extended(&self) -> bool {
        self.extended
    }
    pub fn forced(&self) -> bool {
        self.forced
    }
}
impl NativeGameplayState {
    pub fn callback_spawn_source(
        &self,
        parent: &ProjectV2DefinitionRef,
        index: usize,
        list: NativeCallbackList,
        child: &ProjectV2DefinitionRef,
    ) -> Result<Option<QualifiedNativeCallbackSpawn>, NativeCallbackError> {
        let Some(proof) = qualify_native_callback(self, parent, index, list)? else {
            return Ok(None);
        };
        let Some((extended, forced, children)) =
            spawn_options(&proof.entry.creature.key, &proof.entry.ability.key)
        else {
            return Ok(None);
        };
        let Some(child_name) = child.key.strip_prefix("oteryn:creature.") else {
            return Ok(None);
        };
        if !children.contains(&child_name) {
            return Ok(None);
        }
        native_callback_creature(self, child)?;
        Ok(Some(QualifiedNativeCallbackSpawn {
            parent: proof.entry.creature,
            ability: proof.entry.ability,
            child: child.clone(),
            source_digest: self.source_digest(),
            extended,
            forced,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_actual_native_spawn_options_are_closed_source_rows()
    -> Result<(), Box<dyn std::error::Error>> {
        std::thread::Builder::new().stack_size(64 << 20).spawn(|| -> Result<(), String> {
            let world = crate::content::native_gameplay_test_world_id().map_err(|e| format!("{e:?}"))?;
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/spells.manifest.json");
            let native = super::super::native_gameplay::retained_callback_native_fixture_at(world, &path);
            for (parent, spell, children, extended, forced) in [
                ("gaz_haragoth", "gaz_haragoth_summon", &["minion_of_gaz_haragoth"][..], true, false),
                ("glooth_generator", "glooth_generator_summon", &["energy_pulse"][..], true, true),
                ("lady_tenebris", "tenebris_summon", &["shadow_fiend"][..], false, false),
                ("mazoran", "mazoran_fire", &["rage_of_mazoran"][..], true, true),
                ("mounted_thorn_knight", "thorn_summon", &["thorn_minion"][..], true, false),
                ("plagirath", "plagirath_summon", &["disgusting_ooze"][..], true, true),
                ("professor_maxxen", "generator", &["glooth_generator"][..], true, true),
                ("razzagorn", "razzagorn_summon", &["demon"][..], true, true),
                ("shulgrax", "shulgrax_summon", &["sin_devourer", "damned_soul"][..], true, true),
                ("soulcatcher", "soulcatcher_summon", &["corrupted_soul"][..], true, true),
                ("the_blazing_time_guardian", "time_guardian_lost_time", &["time_waster"][..], false, false),
                ("the_freezing_time_guardian", "time_guardian_lost_time", &["lost_time"][..], false, false),
                ("the_destruction", "destruction_summon", &["disruption"][..], false, true),
                ("the_hunger", "hunger_summon", &["greed"][..], false, true),
                ("the_rage", "rage_summon", &["frenzy"][..], false, true),
                ("world_devourer", "devourer_summon", &["greed", "frenzy", "disruption"][..], false, true),
            ] {
                let key = format!("oteryn:creature.{parent}");
                let ability = format!("oteryn:ability.spell.{spell}");
                let record = native.creature_profiles().records.iter().find(|r| r.profile.target.key == key).ok_or("actual parent")?;
                let crate::content::ProjectV2AuthoringProfileData::Behavior(body) = &record.behavior.as_ref().ok_or("actual behavior")?.data else { return Err("behavior family".into()); };
                let (list, index) = body.attacks.iter().position(|e| e.ability.key == ability).map(|i|(NativeCallbackList::Attack,i))
                    .or_else(||body.defenses.iter().position(|e|e.ability.key == ability).map(|i|(NativeCallbackList::Defence,i))).ok_or("actual source slot")?;
                for child in children {
                    let key = format!("oteryn:creature.{child}");
                    let child = &native.creature_profiles().records.iter().find(|r|r.profile.target.key == key).ok_or("actual source child")?.profile.target;
                    let token = native.callback_spawn_source(&record.profile.target,index,list,child).map_err(|e|format!("{e:?}"))?
                        .ok_or_else(||format!("actual native callback lacks sealed source options: {parent}/{spell}"))?;
                    assert_eq!(token.parent(), &record.profile.target);
                    assert_eq!(token.ability().key, ability);
                    assert_eq!(token.child(), child);
                    assert_eq!((token.extended(),token.forced()),(extended,forced));
                    assert_eq!(token.source_digest(),native.source_digest());
                }
                let unrelated = ProjectV2DefinitionRef { family: crate::content::ProjectV2Family::Creature, key:"oteryn:creature.rat".into(), revision:"definition-r1".into() };
                assert!(native.callback_spawn_source(&record.profile.target,index,list,&unrelated).map_err(|e|format!("{e:?}"))?.is_none(), "source options cannot authorize an unrelated child");
            }
            Ok(())
        })?.join().map_err(|_| "source option test thread panicked")??;
        Ok(())
    }
}

// Immutable source membership for native callback adapters, never runtime authority.
// Generated by build_native_callback_witness.py from the real canonical root and
// existing reviewed callback specifications. No synthetic ProjectV2Draft is created.
use crate::content::{
    ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2SourceIdentityBinding,
    ProjectV2SourceIdentityDisposition,
    native_gameplay::{CreatureProfileRecord, CreatureProfilesDocument, NativeGameplayState},
};
use std::sync::OnceLock;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCallbackEntry {
    pub creature: Ref,
    pub ability: Ref,
    pub encounter: Ref,
    pub index: usize,
    phase: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Witness {
    schema: String,
    source_definitions_sha256: String,
    profiles: Vec<ProjectV2AuthoringProfile>,
    entries: Vec<NativeCallbackEntry>,
    source_identity_bindings: Vec<ProjectV2SourceIdentityBinding>,
    source_events: Vec<SourceEvent>,
    #[serde(default)]
    event_creature_records: Vec<CreatureProfileRecord>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceEvent {
    name: String,
    repository: String,
    revision: String,
    path: String,
    sha256: String,
}

const EVENT_CONTEXT_KEYS: &[&str] = &[
    "oteryn:creature.baby_dragon",
    "oteryn:creature.the_enraged_thorn_knight",
    "oteryn:creature.the_shielded_thorn_knight",
    "oteryn:creature.thorn_steed",
    "oteryn:creature.eradicator2",
    "oteryn:creature.shadow_tentacle",
    "oteryn:creature.possessed_tree",
    "oteryn:creature.unbound_blightwalker",
    "oteryn:creature.unbound_demon",
    "oteryn:creature.unbound_demon_outcast",
    "oteryn:creature.unbound_defiler",
    "oteryn:creature.dragonking_zyrtarch",
    "oteryn:creature.soul_of_dragonking_zyrtarch",
];

fn witness() -> Result<&'static Witness, NativeCallbackError> {
    static WITNESS: OnceLock<Result<Witness, NativeCallbackError>> = OnceLock::new();
    WITNESS
        .get_or_init(|| {
            let witness: Witness =
                serde_json::from_str(include_str!("native_monster_callback_witness.json"))
                    .map_err(|_| NativeCallbackError::InvalidSource)?;
            if witness.schema != "OTERYN_NATIVE_MONSTER_CALLBACKS/v1"
                || witness.entries.len() > 128
                || witness.profiles.len() > 512
                || witness.event_creature_records.len() > EVENT_CONTEXT_KEYS.len()
                || witness.event_creature_records.iter().any(|record| {
                    !EVENT_CONTEXT_KEYS.contains(&record.profile.target.key.as_str())
                        || witness
                            .event_creature_records
                            .iter()
                            .filter(|other| other.profile.target == record.profile.target)
                            .count()
                            != 1
                })
                || witness.source_identity_bindings.len() > 256
                || witness.source_definitions_sha256.len() != 64
                || !witness
                    .source_definitions_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            {
                return Err(NativeCallbackError::InvalidSource);
            }
            Ok(witness)
        })
        .as_ref()
        .map_err(|error| *error)
}

/// This describes a source rule only. The caller must still prove current actor,
/// scope generation, content pin, map facts and occurrence at every mutation.
#[derive(Debug, Clone)]
pub struct QualifiedNativeCallback {
    entry: NativeCallbackEntry,
    content: [u8; 32],
}
impl QualifiedNativeCallback {
    pub fn entry(&self) -> &NativeCallbackEntry {
        &self.entry
    }
    pub fn content(&self) -> [u8; 32] {
        self.content
    }
    /// Immutable canonical Encounter body retained by the bounded source witness.
    /// This carries no actor, map or scope authorization.
    pub fn encounter_profile(
        &self,
    ) -> Result<&'static ProjectV2AuthoringProfile, NativeCallbackError> {
        let mut profiles = witness()?
            .profiles
            .iter()
            .filter(|profile| profile.target == self.entry.encounter);
        let profile = profiles.next().ok_or(NativeCallbackError::InvalidSource)?;
        if profiles.next().is_some() || !matches!(profile.data, Data::Encounter(_)) {
            return Err(NativeCallbackError::InvalidSource);
        }
        Ok(profile)
    }
}

pub fn qualify_native_callback(
    native: &NativeGameplayState,
    creature: &Ref,
    index: usize,
    list: NativeCallbackList,
) -> Result<Option<QualifiedNativeCallback>, NativeCallbackError> {
    qualify_native_callback_document(native.creature_profiles(), creature, index, list).map(
        |entry| {
            entry.map(|entry| QualifiedNativeCallback {
                entry,
                content: native.source_digest(),
            })
        },
    )
}

pub fn qualify_native_callback_document(
    document: &CreatureProfilesDocument,
    creature: &Ref,
    index: usize,
    list: NativeCallbackList,
) -> Result<Option<NativeCallbackEntry>, NativeCallbackError> {
    let witness = witness()?;
    let phase = if list == NativeCallbackList::Attack {
        "attacks"
    } else {
        "defenses"
    };
    let mut entries = witness.entries.iter().filter(|entry| {
        entry.creature == *creature && entry.index == index && entry.phase == phase
    });
    let Some(entry) = entries.next() else {
        return Ok(None);
    };
    if entries.next().is_some()
        || document.schema != "OTERYN_NATIVE_CREATURE_PROFILES/v1"
        || document.source_definitions_sha256.as_deref()
            != Some(witness.source_definitions_sha256.as_str())
    {
        return Err(NativeCallbackError::InvalidSource);
    }
    let mut records = document
        .records
        .iter()
        .filter(|record| record.profile.target == *creature);
    let record = records.next().ok_or(NativeCallbackError::InvalidSource)?;
    if records.next().is_some() {
        return Err(NativeCallbackError::InvalidSource);
    }
    let behavior = record
        .behavior
        .as_ref()
        .ok_or(NativeCallbackError::InvalidSource)?;
    let Data::Behavior(body) = &behavior.data else {
        return Err(NativeCallbackError::InvalidSource);
    };
    let schedule = if list == NativeCallbackList::Attack {
        &body.attacks
    } else {
        &body.defenses
    };
    if schedule
        .get(index)
        .is_none_or(|scheduled| scheduled.ability != entry.ability)
    {
        return Err(NativeCallbackError::InvalidSource);
    }
    let definitions = record
        .attack_definitions
        .as_ref()
        .ok_or(NativeCallbackError::InvalidSource)?;
    let mut abilities = definitions
        .profiles
        .iter()
        .filter(|profile| profile.target == entry.ability);
    let ability = abilities.next().ok_or(NativeCallbackError::InvalidSource)?;
    if abilities.next().is_some() {
        return Err(NativeCallbackError::InvalidSource);
    }
    let Data::Ability(body) = &ability.data else {
        return Err(NativeCallbackError::InvalidSource);
    };
    if body
        .details
        .as_ref()
        .is_none_or(|details| details.encounter.as_ref() != Some(&entry.encounter))
    {
        return Err(NativeCallbackError::InvalidSource);
    }
    for actual in [&record.profile, behavior, ability] {
        let mut expected = witness
            .profiles
            .iter()
            .filter(|profile| profile.target == actual.target);
        if expected.next() != Some(actual) || expected.next().is_some() {
            return Err(NativeCallbackError::InvalidSource);
        }
    }
    for target in [&entry.creature, &entry.encounter] {
        if !witness.source_identity_bindings.iter().any(|binding| {
            binding.target == *target
                && binding.disposition == ProjectV2SourceIdentityDisposition::Exact
                && !binding.source_key.is_empty()
                && !binding.source_revision.is_empty()
        }) {
            return Err(NativeCallbackError::InvalidSource);
        }
    }
    Ok(Some(entry.clone()))
}

/// Actual native immutable Creature body from this bounded source closure.
/// This is data only; caller source specs still choose the permitted children.
pub fn native_callback_creature<'a>(
    native: &'a NativeGameplayState,
    target: &Ref,
) -> Result<&'a crate::content::ProjectV2CreatureAuthoring, NativeCallbackError> {
    native_callback_creature_document(native.creature_profiles(), target)
}
fn native_callback_creature_document<'a>(
    document: &'a CreatureProfilesDocument,
    target: &Ref,
) -> Result<&'a crate::content::ProjectV2CreatureAuthoring, NativeCallbackError> {
    let witness = witness()?;
    if target.family != crate::content::ProjectV2Family::Creature
        || document.source_definitions_sha256.as_deref()
            != Some(witness.source_definitions_sha256.as_str())
    {
        return Err(NativeCallbackError::InvalidSource);
    }
    let mut expected = witness
        .profiles
        .iter()
        .filter(|profile| profile.target == *target);
    let source = expected.next().ok_or(NativeCallbackError::InvalidSource)?;
    if expected.next().is_some() {
        return Err(NativeCallbackError::InvalidSource);
    }
    let mut records = document
        .records
        .iter()
        .filter(|record| record.profile.target == *target);
    let record = records.next().ok_or(NativeCallbackError::InvalidSource)?;
    if records.next().is_some() || record.profile != *source {
        return Err(NativeCallbackError::InvalidSource);
    }
    if EVENT_CONTEXT_KEYS.contains(&target.key.as_str()) {
        let mut frozen = witness
            .event_creature_records
            .iter()
            .filter(|record| record.profile.target == *target);
        if frozen.next() != Some(record) || frozen.next().is_some() {
            return Err(NativeCallbackError::InvalidSource);
        }
    }
    let Data::Creature(creature) = &record.profile.data else {
        return Err(NativeCallbackError::InvalidSource);
    };
    Ok(creature)
}

// Canary47dfd51 source scripts, exact createMonster extended/force options.
// See NATIVE_MONSTER_CALLBACK_SOURCE_V1.md for every source file:line.
fn spawn_options(parent: &str, ability: &str) -> Option<(bool, bool, &'static [&'static str])> {
    let parent = parent.strip_prefix("oteryn:creature.")?;
    let ability = ability.strip_prefix("oteryn:ability.spell.")?;
    Some(match (parent, ability) {
        ("charging_outburst", "outburst_explode") => (false, true, &["outburst"]),
        ("eruption_of_destruction", "eruption_of_destruction_explosion") => {
            (true, true, &["demon"])
        }
        ("gaz_haragoth", "gaz_haragoth_summon") => (true, false, &["minion_of_gaz_haragoth"]),
        ("glooth_generator", "glooth_generator_summon") => (true, true, &["energy_pulse"]),
        ("lady_tenebris", "tenebris_summon") => (false, false, &["shadow_fiend"]),
        ("mazoran", "mazoran_fire") => (true, true, &["rage_of_mazoran"]),
        ("mounted_thorn_knight", "thorn_summon") => (true, false, &["thorn_minion"]),
        ("plagirath", "plagirath_summon") => (true, true, &["disgusting_ooze"]),
        ("professor_maxxen", "generator") => (true, true, &["glooth_generator"]),
        ("razzagorn", "razzagorn_summon") => (true, true, &["demon"]),
        ("shulgrax", "shulgrax_summon") => (true, true, &["sin_devourer", "damned_soul"]),
        ("soulcatcher", "soulcatcher_summon") => (true, true, &["corrupted_soul"]),
        ("the_blazing_time_guardian", "time_guardian_lost_time") => {
            (false, false, &["time_waster"])
        }
        ("the_freezing_time_guardian", "time_guardian_lost_time") => (false, false, &["lost_time"]),
        ("the_destruction", "destruction_summon") => (false, true, &["disruption"]),
        ("the_hunger", "hunger_summon") => (false, true, &["greed"]),
        ("the_rage", "rage_summon") => (false, true, &["frenzy"]),
        ("world_devourer", "devourer_summon") => (false, true, &["greed", "frenzy", "disruption"]),
        _ => return None,
    })
}

/// Pinned CreatureEvent membership. This is immutable source DATA; runtime owners
/// still prove current source actor, actual occurrence, frame and physical map.
#[derive(Debug, Clone)]
pub struct QualifiedNativeOutburstEvents {
    outburst: Ref,
    charging: Ref,
    spark: Ref,
    callback: QualifiedNativeCallback,
}
/// These registered source events do not have synthetic Ability schedule slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOutburstEvent {
    Think,
    ChargingDeath,
    OmrafirBoom,
    OmrafirReform,
}
#[derive(Debug, Clone)]
pub struct QualifiedNativeCreatureEventSpawn {
    parent: Ref,
    child: Ref,
    source_digest: [u8; 32],
    event: NativeOutburstEvent,
}
impl QualifiedNativeCreatureEventSpawn {
    pub fn parent(&self) -> &Ref {
        &self.parent
    }
    pub fn child(&self) -> &Ref {
        &self.child
    }
    pub fn source_digest(&self) -> [u8; 32] {
        self.source_digest
    }
    pub fn event(&self) -> NativeOutburstEvent {
        self.event
    }
    pub fn forced(&self) -> bool {
        true
    }
    pub fn extended(&self) -> bool {
        self.event == NativeOutburstEvent::OmrafirBoom
    }
}
pub(super) fn omrafir_event_spawn(
    proof: &super::native_omrafir::QualifiedOmrafirEvents,
    kind: super::native_omrafir::OmrafirEventBirth,
) -> QualifiedNativeCreatureEventSpawn {
    let (child, event) = match kind {
        super::native_omrafir::OmrafirEventBirth::Hellfire => {
            (proof.hellfire(), NativeOutburstEvent::OmrafirBoom)
        }
        super::native_omrafir::OmrafirEventBirth::NormalReform => {
            (proof.parent(), NativeOutburstEvent::OmrafirReform)
        }
    };
    QualifiedNativeCreatureEventSpawn {
        parent: proof.parent().clone(),
        child: child.clone(),
        source_digest: proof.content(),
        event,
    }
}
impl QualifiedNativeOutburstEvents {
    pub fn spawn_source(
        &self,
        event: NativeOutburstEvent,
        child: &Ref,
    ) -> Option<QualifiedNativeCreatureEventSpawn> {
        let parent = match event {
            NativeOutburstEvent::Think if child == &self.spark || child == &self.charging => {
                &self.outburst
            }
            NativeOutburstEvent::ChargingDeath if child == &self.outburst => &self.charging,
            _ => return None,
        };
        Some(QualifiedNativeCreatureEventSpawn {
            parent: parent.clone(),
            child: child.clone(),
            source_digest: self.source_digest(),
            event,
        })
    }
    pub fn outburst(&self) -> &Ref {
        &self.outburst
    }
    pub fn charging(&self) -> &Ref {
        &self.charging
    }
    pub fn spark(&self) -> &Ref {
        &self.spark
    }
    pub fn callback_source(&self) -> &QualifiedNativeCallback {
        &self.callback
    }
    pub fn source_digest(&self) -> [u8; 32] {
        self.callback.content()
    }
}
impl NativeGameplayState {
    pub fn outburst_events_source(
        &self,
    ) -> Result<QualifiedNativeOutburstEvents, NativeCallbackError> {
        qualify_outburst_events_document(self.creature_profiles(), self.source_digest())
    }
}
fn qualify_outburst_events_document(
    document: &CreatureProfilesDocument,
    digest: [u8; 32],
) -> Result<QualifiedNativeOutburstEvents, NativeCallbackError> {
    let witness = witness()?;
    let expected = [
        (
            "OutburstCharge",
            "creaturescripts_outburst_charge.lua",
            "a8c4ba6e835176fa7d918d5c92fefed9f805d2e3bd2893a6811fec23740174fd",
        ),
        (
            "ChargingOutDeath",
            "creaturescripts_charging_out_death.lua",
            "c91c47b3bb887d7fd55396fe9c9d68d4ee2287557ddac0b8df0a34e7b82391fa",
        ),
    ];
    if expected.iter().any(|(name, file, sha)| {
        witness
            .source_events
            .iter()
            .filter(|event| event.name == *name)
            .count()
            != 1
            || !witness.source_events.iter().any(|event| {
                event.name == *name
                    && event.repository == "opentibiabr/canary"
                    && event.revision == "47dfd51f45280a59a1d3e50ba7edd573d7234446"
                    && event.path
                        == format!(
                            "data-otservbr-global/scripts/quests/heart_of_destruction/{file}"
                        )
                    && event.sha256 == *sha
            })
    }) {
        return Err(NativeCallbackError::InvalidSource);
    }
    let body = |name: &str| -> Result<Ref, NativeCallbackError> {
        let key = format!("oteryn:creature.{name}");
        let mut records = document
            .records
            .iter()
            .filter(|record| record.profile.target.key == key);
        let record = records.next().ok_or(NativeCallbackError::InvalidSource)?;
        if records.next().is_some() {
            return Err(NativeCallbackError::InvalidSource);
        }
        native_callback_creature_document(document, &record.profile.target)?;
        let behavior = record
            .behavior
            .as_ref()
            .ok_or(NativeCallbackError::InvalidSource)?;
        let definitions = record
            .attack_definitions
            .as_ref()
            .ok_or(NativeCallbackError::InvalidSource)?;
        for profile in std::iter::once(behavior).chain(definitions.profiles.iter()) {
            let mut expected = witness
                .profiles
                .iter()
                .filter(|expected| expected.target == profile.target);
            if expected.next() != Some(profile) || expected.next().is_some() {
                return Err(NativeCallbackError::InvalidSource);
            }
        }
        if !witness.source_identity_bindings.iter().any(|binding| {
            binding.target == record.profile.target
                && binding.source_key == "oteryn:source.canary"
                && binding.source_revision == "47dfd51f45280a59a1d3e50ba7edd573d7234446"
                && binding.identity_namespace == "canary/monster-file"
                && binding.external_id == format!("quests/heart_of_destruction/{name}")
                && binding.disposition == ProjectV2SourceIdentityDisposition::Exact
        }) {
            return Err(NativeCallbackError::InvalidSource);
        }
        Ok(record.profile.target.clone())
    };
    let outburst = body("outburst")?;
    let spark = body("spark_of_destruction")?;
    let charging = Ref {
        family: crate::content::ProjectV2Family::Creature,
        key: "oteryn:creature.charging_outburst".into(),
        revision: "definition-r1".into(),
    };
    let entry = witness
        .entries
        .iter()
        .find(|entry| {
            entry.creature == charging
                && entry.ability.key == "oteryn:ability.spell.outburst_explode"
                && entry.phase == "attacks"
        })
        .ok_or(NativeCallbackError::InvalidSource)?;
    let callback_entry = qualify_native_callback_document(
        document,
        &charging,
        entry.index,
        NativeCallbackList::Attack,
    )?
    .ok_or(NativeCallbackError::InvalidSource)?;
    let callback = QualifiedNativeCallback {
        entry: callback_entry,
        content: digest,
    };
    Ok(QualifiedNativeOutburstEvents {
        outburst,
        charging,
        spark,
        callback,
    })
}

#[cfg(test)]
mod outburst_event_tests {
    use super::*;
    #[test]
    fn srx3_outburst_events_qualify_actual_native_bodies_and_refuse_body_drift()
    -> Result<(), Box<dyn std::error::Error>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), String> {
                let world = crate::content::native_gameplay_test_world_id()
                    .map_err(|error| format!("{error:?}"))?;
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../content/spells.manifest.json");
                let native = crate::content::native_gameplay::retained_callback_native_fixture_at(
                    world, &path,
                );
                let proof = native
                    .outburst_events_source()
                    .map_err(|error| format!("{error:?}"))?;
                assert_eq!(proof.callback_source().entry().creature, *proof.charging());
                assert_eq!(proof.source_digest(), native.source_digest());
                for (event, child, parent) in [
                    (NativeOutburstEvent::Think, proof.spark(), proof.outburst()),
                    (
                        NativeOutburstEvent::Think,
                        proof.charging(),
                        proof.outburst(),
                    ),
                    (
                        NativeOutburstEvent::ChargingDeath,
                        proof.outburst(),
                        proof.charging(),
                    ),
                ] {
                    let token = proof
                        .spawn_source(event, child)
                        .ok_or("actual closed event birth")?;
                    assert_eq!(token.parent(), parent);
                    assert_eq!(token.child(), child);
                    assert_eq!(token.event(), event);
                    assert!(!token.extended());
                    assert!(token.forced());
                }
                assert!(
                    proof
                        .spawn_source(NativeOutburstEvent::Think, proof.outburst())
                        .is_none()
                );
                assert!(
                    proof
                        .spawn_source(NativeOutburstEvent::ChargingDeath, proof.spark())
                        .is_none()
                );
                for name in ["outburst", "spark_of_destruction"] {
                    let mut changed = native.creature_profiles().clone();
                    let record = changed
                        .records
                        .iter_mut()
                        .find(|record| {
                            record.profile.target.key == format!("oteryn:creature.{name}")
                        })
                        .ok_or("actual source record")?;
                    let Data::Creature(creature) = &mut record.profile.data else {
                        return Err("actual creature family".into());
                    };
                    *creature
                        .health
                        .as_mut()
                        .ok_or("actual pinned source health")? += 1;
                    assert!(
                        qualify_outburst_events_document(&changed, native.source_digest()).is_err(),
                        "same canonical root cannot bless changed {name} native body"
                    );
                }
                Ok(())
            })?
            .join()
            .map_err(|_| "native Outburst event proof thread panicked")??;
        Ok(())
    }
}
