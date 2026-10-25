#[allow(clippy::expect_used)]
#[cfg(test)]
mod time_profile_aggregate_tests {
    use crate::ability::RevisionSet;
    use crate::ai_think::{
        ThinkOccurrence,
        profile_schedule::{ProfileScheduleInput, ProfileScheduleState, ScheduleError},
    };
    use crate::content::{ProjectV2DefinitionRef as Ref, ProjectV2Family};
    use crate::foundation::owner_timer::SemanticTimeMicros;
    use crate::monster_combat_lane::*;
    use oteryn_simulation_determinism::GameplayDecisionRoot;
    use std::collections::BTreeMap;
    #[test]
    fn actual_schedule_source_profile_phase_and_return_preserve_registered_owner_lineage() {
        let (mut r, d, a, mut f, stamp) = crate::source_encounter_seven_tests::fixture();
        let source = crate::source_encounter_seven_tests::source(&r, &d, 0);
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
