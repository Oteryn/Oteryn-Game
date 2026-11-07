//! One exact-reference qualifier for detached source and admitted outer closures.
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use std::collections::BTreeMap;

impl NativeSummonCatalog {
    pub fn from_native(
        runtime: &ChannelRuntimeV1,
        native: &NativeGameplayState,
        parent: &Ref,
    ) -> Result<Self, SummonError> {
        let digest = runtime.content_pin().server_artifact_digest();
        if native.source_digest() != digest {
            return Err(SummonError::InvalidSource);
        }
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| &record.profile.target == parent)
            .ok_or(SummonError::InvalidSource)?;
        let Some(Data::Behavior(behavior)) = record.behavior.as_ref().map(|b| &b.data) else {
            return Err(SummonError::InvalidSource);
        };
        let summons = behavior
            .summons
            .as_ref()
            .ok_or(SummonError::InvalidSource)?;
        Self::qualify(
            parent,
            summons,
            native
                .creature_profiles()
                .records
                .iter()
                .map(|record| (&record.profile.target, &record.profile.data)),
            digest,
            |child| child.family == crate::content::ProjectV2Family::Creature,
        )
    }

    pub(super) fn qualify<'a>(
        parent: &Ref,
        summons: &crate::content::ProjectV2Summons,
        profiles: impl IntoIterator<Item = (&'a Ref, &'a Data)>,
        digest: [u8; 32],
        member: impl Fn(&Ref) -> bool,
    ) -> Result<Self, SummonError> {
        let mut index = BTreeMap::new();
        for (reference, data) in profiles {
            if index.insert(reference, data).is_some() {
                return Err(SummonError::InvalidSource);
            }
        }
        let mut targets = std::collections::BTreeSet::new();
        if summons.entries.is_empty()
            || summons.entries.len() > 8
            || !(1..=16).contains(&summons.max_summons)
            || summons.entries.iter().any(|entry| {
                !targets.insert(&entry.creature)
                    || entry.interval_ms == 0
                    || entry.chance_ppm > 1_000_000
                    || !(1..=16).contains(&entry.count)
            })
        {
            return Err(SummonError::InvalidSource);
        }
        let entries = summons
            .entries
            .iter()
            .enumerate()
            .map(|(entry_index, entry)| {
                if !member(&entry.creature) {
                    return Err(SummonError::UndefinedTarget);
                }
                let Some(Data::Creature(child)) = index.get(&entry.creature).copied() else {
                    return Err(SummonError::InvalidSource);
                };
                let health = child
                    .health
                    .and_then(|h| i64::try_from(h).ok())
                    .filter(|h| *h > 0)
                    .ok_or(SummonError::InvalidSource)?;
                Ok(NativeSummonSpec {
                    parent: parent.clone(),
                    child: entry.creature.clone(),
                    health,
                    index: entry_index,
                    total: summons.max_summons,
                    count: entry.count,
                    content: digest,
                    admission: NativeSummonAdmissionSpec::qualified(
                        &parent.key,
                        &entry.creature.key,
                        health,
                        entry_index,
                        summons.max_summons,
                        entry.count,
                        digest,
                    )
                    .map_err(SummonError::Carrier)?,
                })
            })
            .collect();
        Ok(Self {
            parent: parent.clone(),
            entries,
            targets: summons
                .entries
                .iter()
                .map(|entry| entry.creature.clone())
                .collect(),
            content: digest,
        })
    }
}

impl NativeSummonCatalog {
    pub fn from_project(
        runtime: &ChannelRuntimeV1,
        parent: &Ref,
        draft: &ProjectV2Draft,
        loader_digest: [u8; 32],
    ) -> Result<Self, SummonError> {
        if !native_draft_world_matches(runtime, draft)
            || loader_digest != runtime.content_pin().server_artifact_digest()
            || !member(draft, parent)
        {
            return Err(SummonError::InvalidSource);
        }
        let record = draft
            .core
            .records
            .iter()
            .find_map(|r| match r {
                ProjectReferenceRecord::Creature {
                    identity, behavior, ..
                } if identity.key == parent.key && identity.revision == parent.revision => {
                    Some(behavior)
                }
                _ => None,
            })
            .ok_or(SummonError::InvalidSource)?;
        let behavior = draft
            .state
            .authoring_profiles
            .iter()
            .find_map(|p| match &p.data {
                Data::Behavior(b)
                    if p.target.key == record.key && p.target.revision == record.revision =>
                {
                    Some(b)
                }
                _ => None,
            })
            .ok_or(SummonError::InvalidSource)?;
        let summons = behavior
            .summons
            .as_ref()
            .ok_or(SummonError::InvalidSource)?;
        Self::qualify(
            parent,
            summons,
            draft
                .state
                .authoring_profiles
                .iter()
                .map(|profile| (&profile.target, &profile.data)),
            loader_digest,
            |child| member(draft, child),
        )
    }

    pub fn facts(
        &self,
        runtime: &ChannelRuntimeV1,
        occurrence: ThinkOccurrence,
        target_with_path: Option<ExactActorRef>,
    ) -> MonsterSummonFacts {
        MonsterSummonFacts {
            occurrence,
            is_summon: current_source_creature_role(runtime, occurrence.actor)
                .map(|role| !role.is_ordinary())
                .unwrap_or(true),
            target_with_path,
            total_count: runtime.native_summon_count(occurrence.actor, None),
            entry_counts: self
                .targets
                .iter()
                .map(|creature| ObservedSummonCount {
                    creature: creature.clone(),
                    count: runtime.native_summon_count(occurrence.actor, Some(&creature.key)),
                })
                .collect(),
        }
    }
}

impl NativeSummonCatalog {
    pub fn for_actor(
        runtime: &ChannelRuntimeV1,
        native: &NativeGameplayState,
        actor: ExactActorRef,
    ) -> Result<Self, SummonError> {
        let snapshot = runtime
            .companion_snapshot(actor)
            .map_err(SummonError::Carrier)?;
        let creature = Ref {
            family: crate::content::ProjectV2Family::Creature,
            key: snapshot.state.policy.definition_key.clone(),
            revision: snapshot.state.policy.definition_revision.clone(),
        };
        Self::from_native(runtime, native, &creature)
    }
}
