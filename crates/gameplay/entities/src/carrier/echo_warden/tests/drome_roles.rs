//! Known native-role mechanics under an explicit fixture grant; no fabricated canonical profiles.
use super::*;
use crate::carrier::tibiadrome_modifiers::{DromeCreatureRole as Role, DromeModifier};
fn role(runtime: &mut ChannelRuntimeV1, kind: Role, x: i32, run: u8) -> TestResult<ExactActorRef> {
    let actor = ordinary(runtime, at(x, 100, 7))?;
    runtime.scale_tibiadrome_opponent(actor, 0).map_err(debug)?;
    runtime
        .bind_tibiadrome_modifiers(
            actor,
            [run; 16],
            [
                DromeModifier::Somersault,
                DromeModifier::ThatEscalatedQuickly,
            ],
        )
        .map_err(debug)?;
    runtime
        .test_bind_drome_creature_role(actor, kind, 0)
        .map_err(debug)?;
    assert_eq!(
        runtime
            .companion_snapshot(actor)
            .map_err(debug)?
            .state
            .policy
            .definition_key,
        "oteryn:creature.rat",
        "the explicit mechanics role must not fabricate a canonical Drome profile"
    );
    Ok(actor)
}
#[test]
fn srx3_drome_native_roles_preserve_all_five_accepted_base_stats() -> TestResult {
    for (role, hp, hit) in [
        (Role::Domestikion, 750, 150),
        (Role::Hoodinion, 800, 300),
        (Role::Mearidion, 850, 250),
        (Role::Scissorion, 950, 220),
        (Role::Murmillion, 1100, 80),
    ] {
        assert_eq!((role.base_health(), role.base_maximum_hit()), (hp, hit));
    }
    assert_eq!(Role::Murmillion.paralyze_duration_ms(), Some(5000));
    Ok(())
}
#[test]
fn srx3_drome_native_domestikion_aura_changes_normal_damage_and_only_living_same_run_sources_count()
-> TestResult {
    let (mut runtime, _, _) = fixture(12)?;
    let caster = role(&mut runtime, Role::Hoodinion, 102, 5)?;
    let buff = role(&mut runtime, Role::Domestikion, 103, 5)?;
    role(&mut runtime, Role::Domestikion, 104, 6)?;
    assert_eq!(
        runtime.tibiadrome_damage(caster, 100).map_err(debug)?,
        Some(110)
    );
    runtime
        .borrow_exact_actor_commit()
        .commit_damage(
            buff,
            OwnerDamageCommand {
                target: b"oteryn:creature.rat",
                occurrence: b"aura-source-death",
                binding: b"aura-source-death\0native-role-test",
                damage: 101,
            },
        )
        .map_err(debug)?;
    assert_eq!(
        runtime.tibiadrome_damage(caster, 100).map_err(debug)?,
        Some(100)
    );
    Ok(())
}
#[test]
fn srx3_drome_native_domestikion_aura_includes_self_and_caps_at_fifty_percent() -> TestResult {
    let (mut runtime, _, _) = fixture(12)?;
    let caster = role(&mut runtime, Role::Domestikion, 102, 5)?;
    assert_eq!(
        runtime.tibiadrome_damage(caster, 100).map_err(debug)?,
        Some(110)
    );
    for x in 103..108 {
        role(&mut runtime, Role::Domestikion, x, 5)?;
    }
    assert_eq!(
        runtime.tibiadrome_damage(caster, 100).map_err(debug)?,
        Some(150)
    );
    Ok(())
}
#[test]
fn srx3_drome_native_murmillion_paralyze_uses_ten_second_clock_without_duplicate_pulse()
-> TestResult {
    let (mut runtime, _, _) = fixture(8)?;
    let actor = role(&mut runtime, Role::Murmillion, 102, 5)?;
    assert!(!runtime.drome_role_paralyze_due(actor, 9_999_999));
    assert!(runtime.drome_role_paralyze_due(actor, 10_000_000));
    runtime
        .drome_role_paralyze_applied(actor, 10_000_000)
        .map_err(debug)?;
    assert!(!runtime.drome_role_paralyze_due(actor, 10_000_000));
    assert!(!runtime.drome_role_paralyze_due(actor, 19_999_999));
    assert!(runtime.drome_role_paralyze_due(actor, 20_000_000));
    Ok(())
}
#[test]
fn srx3_drome_native_scissorion_uses_actual_condition_visibility_for_three_seconds() -> TestResult {
    let (mut runtime, _, _) = fixture(8)?;
    let actor = role(&mut runtime, Role::Scissorion, 102, 5)?;
    runtime
        .drome_scissorion_invisible(actor, 1_000)
        .map_err(debug)?;
    assert!(
        runtime
            .actor_conditions(actor, None)
            .map_err(debug)?
            .invisible_at(3_000_999)
    );
    assert!(
        !runtime
            .actor_conditions(actor, None)
            .map_err(debug)?
            .invisible_at(3_001_000)
    );
    Ok(())
}

#[test]
fn srx3_drome_native_scissorion_next_attack_clears_actual_owned_visibility() -> TestResult {
    use oteryn_gameplay_domain::domain::condition::ConditionValues;
    use oteryn_gameplay_foundation::foundation::condition::{
        ApplicationFacts, ConditionDefinition, ConditionSourceKind,
    };
    use oteryn_simulation_determinism::SemanticTimeMicros;
    let (mut runtime, _, _) = fixture(8)?;
    let actor = role(&mut runtime, Role::Scissorion, 102, 5)?;
    let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([1; 32]);
    let definition = ConditionDefinition::new(
        "fixture:drome.scissorion.invisible",
        1,
        ConditionValues::Invisible { duration_ms: 3000 },
    )
    .ok_or("invisible definition")?;
    let drunk = ConditionDefinition::new(
        "fixture:drome.unrelated.drunk",
        1,
        ConditionValues::TimedStatus {
            kind: oteryn_gameplay_domain::domain::condition::StatusKind::Drunk,
            duration_ms: 5000,
        },
    )
    .ok_or("independent condition")?;
    let definitions = [definition, drunk];
    let at = SemanticTimeMicros::from_micros(1000);
    let plan = runtime
        .prepare_actor_condition(
            actor,
            None,
            crate::carrier::runtime_actor_conditions::ActorConditionTransition::ApplyBatch {
                definitions: &definitions,
                source: crate::carrier::runtime_actor_conditions::ConditionSource {
                    actor,
                    session: None,
                    kind: ConditionSourceKind::Creature,
                },
                immunities: &[],
                facts: ApplicationFacts {
                    now: 1000,
                    base_speed: 110,
                    mana_shield_capacity: 0,
                    target_reentry_protected: false,
                    source_reentry_protected: false,
                    target_is_player: false,
                    decision_root: &root,
                    occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
                        actor.placement_identity(),
                    ),
                },
            },
            at,
        )
        .map_err(debug)?;
    runtime.commit_actor_condition(&plan, at).map_err(debug)?;
    assert!(
        runtime
            .actor_conditions(actor, None)
            .map_err(debug)?
            .invisible_at(1000)
    );
    let changed = runtime
        .drome_role_attack_started(actor, 1_001_000)
        .map_err(debug)?;
    assert!(
        !runtime
            .actor_conditions(actor, None)
            .map_err(debug)?
            .invisible_at(1_001_000),
        "the next admitted attack must clear actual native visibility"
    );
    assert!(changed);
    assert!(
        runtime
            .actor_conditions(actor, None)
            .map_err(debug)?
            .has_status(
                oteryn_gameplay_domain::domain::condition::StatusKind::Drunk,
                1_001_000,
            ),
        "next-attack visibility removal must retain unrelated conditions"
    );
    Ok(())
}
