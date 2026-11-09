//! Crystal soul_war_mechanics.lua640–670: each Player-origin health event adds defense.
use super::*;

#[test]
fn srx3_hatred_native_player_periodic_health_event_adds_current_defense_once() -> Result<(), String>
{
    exercise_hatred_periodic(true)
}
#[test]
fn srx3_hatred_native_null_periodic_health_event_keeps_defense() -> Result<(), String> {
    exercise_hatred_periodic(false)
}
fn exercise_hatred_periodic(player_owned: bool) -> Result<(), String> {
    use crate::carrier::soul_war_hatred::HatredEmpowerment;
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
    use oteryn_gameplay_foundation::foundation::condition::{ConditionDefinition, TickFacts};
    let (mut runtime, player, session, image, command) = fixture_with_scope(9, true);
    runtime
        .remove_encounter_actor(image)
        .map_err(|error| format!("{error:?}"))?;
    let key = "oteryn:creature.goshnar_s_hatred";
    let policy = runtime
        .companion_policy(key)
        .map_err(|error| format!("{error:?}"))?;
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
            .map_err(|error| format!("{error:?}"))?,
    );
    runtime
        .install_creature_policy(target, key)
        .map_err(|error| format!("{error:?}"))?;
    assert!(
        runtime
            .soul_war_health_callback(target)
            .map_err(|error| format!("{error:?}"))?
            .ok_or("actual empowered health callback")?
            .is_empowered_boss()
    );
    let before = runtime
        .companion_snapshot(target)
        .map_err(|error| format!("{error:?}"))?;
    let base = runtime
        .effective_creature_defense(&before)
        .map_err(|error| format!("{error:?}"))?;
    runtime
        .change_soul_war_hatred_percent(&before, 0, HatredEmpowerment::Add(10))
        .map_err(|error| format!("{error:?}"))?;
    runtime
        .install_current_soul_war_player_source(player, session, None, 201_000, &PeriodicSource)
        .map_err(|error| format!("{error:?}"))?;
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
        .ok_or("actual periodic condition definition")?,
        source: player_owned.then_some(player),
        lineage: player_owned.then_some(
            oteryn_gameplay_foundation::foundation::condition::ConditionLineage {
                source_character: id(5),
                source_session: *session.as_bytes(),
                source_lease_generation: 1,
                source_actor_placement: player.placement_identity(),
                source_scope_generation: player.scope_generation().get(),
                source_content_digest: runtime.content_pin().server_artifact_digest(),
                source_catalog_digest: *NATIVE_CATALOG.get().ok_or("actual native catalog")?,
                source_creation_command: command.command_id().get(),
            },
        ),
        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(id(121)),
    };
    runtime
        .commit_creature_field_entry(target, 201_000, Some(&field))
        .map_err(|error| format!("{error:?}"))?;
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
        .map_err(|error| format!("{error:?}"))?;
    assert!(
        receipt.health_after < receipt.health_before,
        "HP-positive prerequisite before defense assertion"
    );
    let current = runtime
        .companion_snapshot(target)
        .map_err(|error| format!("{error:?}"))?;
    let expected = base + if player_owned { 10 } else { 0 };
    assert_eq!(
        runtime
            .effective_creature_defense(&current)
            .map_err(|error| format!("{error:?}"))?,
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
        .map_err(|error| format!("{error:?}"))?;
    let current = runtime
        .companion_snapshot(target)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        runtime
            .effective_creature_defense(&current)
            .map_err(|error| format!("{error:?}"))?,
        expected
    );
    Ok(())
}
