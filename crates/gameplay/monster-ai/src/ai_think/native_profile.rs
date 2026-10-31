//! Admitted native profiles: missing executable ability bodies remain Unknown.
use super::*;
use crate::content::ProjectV2AuthoringProfileData as Data;
use crate::content::native_gameplay::NativeGameplayState;
use crate::foundation::ChannelRuntimeV1;

pub(crate) fn project_native(
    runtime: &ChannelRuntimeV1,
    native: &NativeGameplayState,
    creature: &crate::content::ProjectV2DefinitionRef,
) -> Result<CreatureBehaviourProfile, behaviour_profile::ProfileRefusal> {
    use behaviour_profile::ProfileRefusal;
    if native.source_digest() != runtime.content_pin().server_artifact_digest() {
        return Err(ProfileRefusal::Invalid);
    }
    let mut records = native
        .creature_profiles()
        .records
        .iter()
        .filter(|record| &record.profile.target == creature);
    let record = records.next().ok_or(ProfileRefusal::Missing)?;
    if records.next().is_some() {
        return Err(ProfileRefusal::Invalid);
    }
    project_native_record(record)
}

/// The projection of one admitted native creature record (the source digest already checked).
pub(crate) fn project_native_record(
    record: &crate::content::native_gameplay::CreatureProfileRecord,
) -> Result<CreatureBehaviourProfile, behaviour_profile::ProfileRefusal> {
    use behaviour_profile::ProfileRefusal;
    let Some(Data::Behavior(behavior)) = record.behavior.as_ref().map(|p| &p.data) else {
        return Err(ProfileRefusal::Missing);
    };
    let mut abilities = BTreeMap::new();
    for profile in
        record
            .source_item_abilities
            .iter()
            .chain(record.attack_definitions.iter().flat_map(|definitions| {
                definitions
                    .profiles
                    .iter()
                    .filter(|profile| matches!(profile.data, Data::Ability(_)))
            }))
    {
        let Data::Ability(ability) = &profile.data else {
            return Err(ProfileRefusal::Invalid);
        };
        // Source-item operations may also be present in the complete canonical attack
        // closure. A shared exact body is one profile; conflicting bodies still refuse.
        if abilities
            .insert(profile.target.clone(), ability.clone())
            .is_some_and(|previous| previous != *ability)
        {
            return Err(ProfileRefusal::Invalid);
        }
    }
    // Preserve every source schedule slot, even when another slot's callback has no executable
    // closure. Missing bodies cannot be manufactured from names, presentation or magnitude.
    let mut projected = behavior.clone();
    // A resolved native melee body still uses the existing separately qualified swing
    // owner. Its source bounds supply the magnitude required by the think projection;
    // no Ability body or damage formula is inferred from a reference.
    if let Some(melee) = &record.monster_melee {
        for entry in &mut projected.attacks {
            if entry.ability == melee.ability && abilities.contains_key(&entry.ability) {
                entry.magnitude = Some(crate::content::ProjectV2Magnitude {
                    minimum: u64::from(melee.minimum),
                    maximum: u64::from(melee.maximum),
                });
            }
        }
    }
    CreatureBehaviourProfile::project_native(Some(&projected), &abilities)
}

impl CreatureAiTable {
    pub fn admit_native_creature(
        &mut self,
        runtime: &ChannelRuntimeV1,
        native: &NativeGameplayState,
        actor: ExactActorRef,
        creature: &crate::content::ProjectV2DefinitionRef,
        now: u64,
    ) -> Result<(), behaviour_profile::ProfileRefusal> {
        use crate::monster_summon::{CurrentSourceCreatureRole, current_source_creature_role};
        let role = current_source_creature_role(runtime, actor)
            .map_err(|_| behaviour_profile::ProfileRefusal::Invalid)?;
        if !matches!(
            role,
            CurrentSourceCreatureRole::Ordinary
                | CurrentSourceCreatureRole::IntrinsicMonsterSummon(_)
        ) {
            return Err(behaviour_profile::ProfileRefusal::Invalid);
        }
        let profile = project_native(runtime, native, creature)?;
        if role.is_ordinary() && (!profile.targeting().hostile || !profile.targeting().can_target) {
            return Err(behaviour_profile::ProfileRefusal::Invalid);
        }
        if !runtime.matches_live_creature_identity(actor, creature.key.as_bytes()) {
            return Err(behaviour_profile::ProfileRefusal::Invalid);
        }
        let anchor = runtime
            .read_actor_position(actor)
            .map_err(|_| behaviour_profile::ProfileRefusal::Invalid)?
            .position();
        let mut state = CreatureAiState::new(actor, profile, anchor);
        state.spawn = runtime.creature_spawn_cell(actor);
        state.is_summon = !role.is_ordinary();
        self.admit(state, now);
        Ok(())
    }
}
