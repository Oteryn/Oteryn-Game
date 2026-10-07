// Included in MonsterCombatLane's module: data rebinding preserves existing timer and schedule ownership.
#[derive(Clone)]
struct TimeProfileBundle {
    summons: Option<crate::monster_summon::NativeSummonCatalog>,
    creature: Ref,
    behavior: ProjectV2BehaviorAuthoring,
    abilities: BTreeMap<Ref, ProjectV2AbilityAuthoring>,
    melee: Vec<MeleeSource>,
    spells: Vec<(usize, SpellSource)>,
    unqualified: Vec<(usize, AttackError)>,
    native_speed: Option<crate::movement::speed::NativeCreatureSpeed>,
    presentations: Vec<(usize, DefensePresentationSource)>,
    callback_spawn_sources: Vec<crate::source_callback_spawn::SourceCallbackSpawn>,
}
impl TimeProfileBundle {
    fn prepare(
        runtime: &ChannelRuntimeV1,
        source: &crate::source_encounter_seven::SevenSource,
        key: &str,
        draft: &ProjectV2Draft,
    ) -> Result<Self, AttackError> {
        if !matches!(
            key,
            "oteryn:creature.the_time_guardian"
                | "oteryn:creature.the_blazing_time_guardian"
                | "oteryn:creature.the_freezing_time_guardian"
        ) {
            return Err(AttackError::InvalidSource);
        }
        let creature = Ref {
            family: ProjectV2Family::Creature,
            key: key.into(),
            revision: "definition-r1".into(),
        };
        let records = source.native_records();
        let profiles = source.native_profiles();
        let behavior = records
            .iter()
            .find_map(|r| match r {
                ProjectReferenceRecord::Creature {
                    identity, behavior, ..
                } if identity.key == key && identity.revision == creature.revision => {
                    Some(behavior)
                }
                _ => None,
            })
            .ok_or(AttackError::InvalidSource)?;
        let Some(Data::Behavior(b)) = profiles
            .iter()
            .find(|p| p.target.key == behavior.key && p.target.revision == behavior.revision)
            .map(|p| &p.data)
        else {
            return Err(AttackError::InvalidSource);
        };
        if b.attacks.len() > 16 || b.defenses.len() > 8 {
            return Err(AttackError::InvalidSource);
        }
        let abilities: BTreeMap<_, _> = profiles
            .iter()
            .filter_map(|p| {
                if let Data::Ability(a) = &p.data {
                    Some((p.target.clone(), a.clone()))
                } else {
                    None
                }
            })
            .collect();
        let mut melee = Vec::new();
        let mut spells = Vec::new();
        let mut unqualified = Vec::new();
        for (index, e) in b.attacks.iter().enumerate() {
            let ability = abilities
                .get(&e.ability)
                .ok_or(AttackError::InvalidSource)?;
            if ability
                .details
                .as_ref()
                .is_some_and(|d| d.kind == ProjectV2AbilityKind::Melee)
            {
                match MeleeSource::from_native(
                    &creature,
                    index,
                    records,
                    profiles,
                    runtime.content_pin().server_artifact_digest(),
                ) {
                    Ok(s) => melee.push(s),
                    Err(e) => unqualified.push((index, e)),
                }
            } else {
                match SpellSource::from_native(
                    &creature,
                    index,
                    records,
                    profiles,
                    runtime.content_pin().server_artifact_digest(),
                ) {
                    Ok(s) => spells.push((index, s)),
                    Err(e) => unqualified.push((index, e)),
                }
            }
        }
        let presentations = b
            .defenses
            .iter()
            .enumerate()
            .filter_map(|(i, _)| {
                DefensePresentationSource::from_native(
                    &creature,
                    i,
                    records,
                    profiles,
                    runtime.content_pin().server_artifact_digest(),
                )
                .ok()
                .map(|s| (i, s))
            })
            .collect();
        let mut callback_spawn_sources = Vec::new();
        for (i, _) in b.defenses.iter().enumerate() {
            if let Some(s) = crate::source_callback_spawn::SourceCallbackSpawn::qualify(
                draft,
                &creature,
                i,
                ScheduleList::Defence,
                runtime.content_pin().server_artifact_digest(),
            )? {
                callback_spawn_sources.push(s)
            }
        }
        let summons = if b.summons.is_some() {
            Some(
                crate::monster_summon::NativeSummonCatalog::from_project(
                    runtime,
                    &creature,
                    draft,
                    runtime.content_pin().server_artifact_digest(),
                )
                .map_err(|_| AttackError::InvalidSource)?,
            )
        } else {
            None
        };
        Ok(Self {
            summons,
            callback_spawn_sources,
            creature: creature.clone(),
            behavior: b.clone(),
            abilities,
            melee,
            spells,
            unqualified,
            presentations,
            native_speed: crate::movement::speed::NativeCreatureSpeed::from_native(
                runtime,
                &creature,
                records,
                profiles,
                runtime.content_pin().server_artifact_digest(),
            ),
        })
    }
    // No fallible work after the actual physical profile commit. Existing ProfileScheduleState,
    // Think lineage, selected target and pacer remain owned by this same registered NativeActor.
    fn publish(self, actor: &mut NativeActor) {
        actor.creature = self.creature;
        actor.behavior = self.behavior;
        actor.abilities = self.abilities;
        actor.melee = self.melee;
        actor.spells = self.spells;
        actor.unqualified = self.unqualified;
        actor.native_speed = self.native_speed;
        actor.presentations = self.presentations;
        actor.callback_spawn_sources = self.callback_spawn_sources;
        actor.defense_variants.clear();
        actor.appearance_defenses.clear();
        actor.composed.clear();
        actor.heals.clear();
        actor.area_heals.clear();
        actor.threshold_heals.clear();
        actor.icicle = None;
        actor.invisible.clear();
        actor.speed.clear();
        actor.defense_summons.clear();
        actor.summons = self.summons;
    }
}
impl NativeActor {
    #[allow(
        clippy::too_many_arguments,
        reason = "Owner ABI keeps independently qualified source, current fence, exact actor and commit facts explicit"
    )]
    fn dispatch_time_phase(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        owner: &mut crate::source_encounter_seven::TimeGuardianOwner,
        source: &crate::source_encounter_seven::SevenSource,
        proposal: &crate::ai_think::profile_schedule::ProfileAbilityProposal,
        now: SemanticTimeMicros,
    ) -> Result<crate::source_encounter_seven::TimePhaseOutcome, AttackError> {
        match owner.prepare_cast(r, f, stamp, source, proposal, now)? {
            Err(retained) => Ok(retained),
            Ok(prepared) => {
                let bundle = self
                    .time_profiles
                    .as_ref()
                    .and_then(|p| {
                        p.get(
                            crate::source_encounter_seven::TimeGuardianOwner::planned_definition(
                                &prepared,
                            ),
                        )
                    })
                    .ok_or(AttackError::InvalidSource)?
                    .clone();
                let receipt = owner.commit_cast(r, f, stamp, prepared)?;
                bundle.publish(self);
                Ok(receipt)
            }
        }
    }
    fn dispatch_time_return(
        &mut self,
        r: &mut ChannelRuntimeV1,
        f: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        owner: &mut crate::source_encounter_seven::TimeGuardianOwner,
        prepared: crate::source_encounter_seven::DueTimeReturn,
    ) -> Result<crate::source_encounter_seven::TimePhaseOutcome, AttackError> {
        let bundle = self
            .time_profiles
            .as_ref()
            .and_then(|p| p.get("oteryn:creature.the_time_guardian"))
            .ok_or(AttackError::InvalidSource)?
            .clone();
        let receipt = owner.commit_return(r, f, stamp, prepared)?;
        bundle.publish(self);
        Ok(receipt)
    }
}
#[allow(clippy::expect_used)]
#[cfg(test)]
mod time_profile_aggregate_tests {
    use super::*;
    #[test]
    fn actual_schedule_source_profile_phase_and_return_preserve_registered_owner_lineage() {
        let (mut r, d, a, mut f, stamp) = crate::source_encounter_seven::tests::fixture();
        let source = crate::source_encounter_seven::tests::source(&r, &d, 0);
        let creature = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.the_time_guardian".into(),
            revision: "definition-r1".into(),
        };
        let mut lane = MonsterCombatLane::new(&r, &f).expect("qualified fixture");
        lane.register_native(
            &r,
            &mut f,
            a,
            &creature,
            &d.core.records,
            &d.state.authoring_profiles,
            r.content_pin().server_artifact_digest(),
            SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
        let mut phases = BTreeMap::new();
        for key in [
            "oteryn:creature.the_time_guardian",
            "oteryn:creature.the_blazing_time_guardian",
            "oteryn:creature.the_freezing_time_guardian",
        ] {
            phases.insert(
                key.into(),
                TimeProfileBundle::prepare(&r, &source, key, &d).expect("qualified fixture"),
            );
        }
        let native = &mut lane.actors[0];
        native.time_profiles = Some(std::sync::Arc::new(phases));
        native.seven_sources = vec![source.clone()];
        let revisions = RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .expect("qualified fixture");
        let root = GameplayDecisionRoot::from_bytes(r.content_pin().server_artifact_digest());
        let mut selected = None;
        for sequence in 0..512 {
            let plan = native
                .schedule
                .prepare(
                    ThinkOccurrence { actor: a, sequence },
                    &native.behavior,
                    &native.abilities,
                    None,
                    &revisions,
                    &root,
                )
                .expect("qualified fixture");
            if let Some(p) = plan
                .proposals
                .into_iter()
                .find(|p| p.ability == source.ability())
            {
                selected = Some((sequence, p));
                break;
            }
        }
        let (sequence, proposal) = selected
            .expect("source 10% defense chance qualified within bounded deterministic census");
        let before = r.companion_snapshot(a).expect("qualified fixture");
        let result = native
            .dispatch_time_phase(
                &mut r,
                &f,
                stamp,
                &mut lane.time_guardian_owner,
                &source,
                &proposal,
                SemanticTimeMicros::from_micros(0),
            )
            .expect("qualified fixture");
        assert_eq!(native.creature.key, result.definition);
        assert_eq!(result.health, before.health);
        assert_eq!(native.callback_spawn_sources.len(), 1);
        // Blazing has real ordinary summons; obtain current counts/role from its prequalified native catalog.
        // No target is selected in this fixture, so no path fact is invented.
        let occurrence = ThinkOccurrence {
            actor: a,
            sequence: sequence + 1,
        };
        let facts = native
            .summons
            .as_ref()
            .map(|catalog| catalog.facts(&r, occurrence, None));
        if let Some(facts) = &facts {
            assert!(!facts.is_summon);
            assert_eq!(facts.total_count, 0);
            let mut missing = ProfileScheduleState::new(a);
            assert!(matches!(
                missing.prepare(
                    occurrence,
                    &native.behavior,
                    &native.abilities,
                    None,
                    &revisions,
                    &root
                ),
                Err(ScheduleError::SummonClockDependency)
            ));
        }
        native
            .schedule
            .prepare_with_summons(
                ProfileScheduleInput {
                    occurrence,
                    behavior: &native.behavior,
                    abilities: &native.abilities,
                    target: None,
                    revisions: &revisions,
                    root: &root,
                },
                facts.as_ref(),
            )
            .expect("qualified fixture");
        let current = r.companion_snapshot(a).expect("qualified fixture");
        let replay = native
            .dispatch_time_phase(
                &mut r,
                &f,
                stamp,
                &mut lane.time_guardian_owner,
                &source,
                &proposal,
                SemanticTimeMicros::from_micros(1),
            )
            .expect("qualified fixture");
        assert_eq!(replay, result);
        assert_eq!(r.companion_snapshot(a).expect("qualified fixture"), current);
        let due = lane
            .time_guardian_owner
            .due_returns(
                &r,
                &f,
                &crate::foundation::owner_timer::VirtualOwnerClock::new(
                    SemanticTimeMicros::from_micros(30_000_000),
                ),
            )
            .expect("qualified fixture")
            .pop()
            .expect("qualified fixture")
            .expect("qualified fixture");
        let restored = native
            .dispatch_time_return(&mut r, &f, stamp, &mut lane.time_guardian_owner, due)
            .expect("qualified fixture");
        assert!(restored.returned);
        assert_eq!(native.creature, creature);
        assert!(native.callback_spawn_sources.is_empty());
        assert_eq!(restored.health, before.health);
        native
            .schedule
            .prepare(
                ThinkOccurrence {
                    actor: a,
                    sequence: sequence + 2,
                },
                &native.behavior,
                &native.abilities,
                None,
                &revisions,
                &root,
            )
            .expect("qualified fixture");
    }
}
