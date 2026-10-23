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
    let Some(Data::Behavior(behavior)) = record.behavior.as_ref().map(|p| &p.data) else {
        return Err(ProfileRefusal::Missing);
    };
    let mut abilities = BTreeMap::new();
    for profile in &record.source_item_abilities {
        let Data::Ability(ability) = &profile.data else {
            return Err(ProfileRefusal::Invalid);
        };
        if abilities
            .insert(profile.target.clone(), ability.clone())
            .is_some()
        {
            return Err(ProfileRefusal::Invalid);
        }
    }
    // The legacy native schema admits an explicit melee approximation but no full executable
    // closure for those attacks. Preserve its existing swing owner and target-only projection;
    // never manufacture an Ability body from a ref, presentation, or attack magnitude.
    let mut projected = behavior.clone();
    if projected
        .attacks
        .iter()
        .chain(&projected.defenses)
        .any(|entry| !abilities.contains_key(&entry.ability))
    {
        projected.attacks.clear();
        projected.defenses.clear();
    }
    CreatureBehaviourProfile::project_with_summons(Some(&projected), &abilities)
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
        self.admit(state, now);
        Ok(())
    }
}
