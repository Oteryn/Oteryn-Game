//! Crystal soul_war_mechanics.lua640–670: each Player-origin health event adds defense.
use super::*;

#[test]
fn srx3_hatred_native_player_periodic_health_event_adds_current_defense_once() {
    exercise_hatred_periodic(true);
}
#[test]
fn srx3_hatred_native_null_periodic_health_event_keeps_defense() {
    exercise_hatred_periodic(false);
}
fn exercise_hatred_periodic(player_owned: bool) {
    use crate::carrier::soul_war_hatred::HatredEmpowerment;
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
    use oteryn_gameplay_foundation::foundation::condition::{ConditionDefinition, TickFacts};
    let (mut runtime, player, session, image, command) = fixture_with_scope(9, true);
    runtime
        .remove_encounter_actor(image)
        .expect("healthy source fixture replacement");
    let key = "oteryn:creature.goshnar_s_hatred";
    let policy = runtime.companion_policy(key).expect("actual native Hatred");
    let target = ExactActorRef(
        runtime
            .admit_encounter_fixture_actor(
                ActorState(2),
                key,
                policy.maximum_health,
                Some(LocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                }),
            )
            .expect("native target"),
    );
    runtime
        .install_creature_policy(target, key)
        .expect("exact source health token");
    assert!(
        runtime
            .soul_war_health_callback(target)
            .unwrap()
            .unwrap()
            .is_empowered_boss()
    );
    let before = runtime.companion_snapshot(target).unwrap();
    let base = runtime
        .effective_creature_defense(&before)
        .expect("canonical source defense");
    runtime
        .change_soul_war_hatred_percent(&before, 0, HatredEmpowerment::Add(10))
        .unwrap();
    runtime
        .install_current_soul_war_player_source(player, session, None, 201_000, &PeriodicSource)
        .expect("actual current condition creator facts");
    let field = runtime_actor_periodic::CreatureFieldEntry {
        definition: ConditionDefinition::new(
            "source.actual-player.hatred-periodic",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Fire,
                total_min: 20,
                total_max: 20,
                per_tick: 10,
                interval_ms: 1000,
                delayed: false,
            },
        )
        .unwrap(),
        source: player_owned.then_some(player),
        lineage: player_owned.then_some(
            oteryn_gameplay_foundation::foundation::condition::ConditionLineage {
                source_character: id(5),
                source_session: *session.as_bytes(),
                source_lease_generation: 1,
                source_actor_placement: player.placement_identity(),
                source_scope_generation: player.scope_generation().get(),
                source_content_digest: runtime.content_pin().server_artifact_digest(),
                source_catalog_digest: *NATIVE_CATALOG.get().expect("actual native catalog"),
                source_creation_command: command.command_id().get(),
            },
        ),
        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(id(121)),
    };
    runtime
        .commit_creature_field_entry(target, 201_000, Some(&field))
        .unwrap();
    let receipt = runtime
        .apply_creature_periodic_turn_with_sources(
            target,
            201_000,
            TickFacts::default(),
            &mut |current, source, _| {
                Ok(current
                    .bound_attacker_lease(source, session)
                    .ok()
                    .map(|_| 100))
            },
        )
        .expect("actual existing periodic owner");
    assert!(
        receipt.health_after < receipt.health_before,
        "HP-positive prerequisite before defense assertion"
    );
    let current = runtime.companion_snapshot(target).unwrap();
    let expected = base + if player_owned { 10 } else { 0 };
    assert_eq!(
        runtime.effective_creature_defense(&current).unwrap(),
        expected,
        "source incoming callback runs once for an actual Player-owned periodic health event"
    );
    runtime
        .apply_creature_periodic_turn_with_sources(
            target,
            201_000,
            TickFacts::default(),
            &mut |current, source, _| {
                Ok(current
                    .bound_attacker_lease(source, session)
                    .ok()
                    .map(|_| 100))
            },
        )
        .expect("consumed exact tick replay");
    let current = runtime.companion_snapshot(target).unwrap();
    assert_eq!(
        runtime.effective_creature_defense(&current).unwrap(),
        expected
    );
}
