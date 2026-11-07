//! The actual native policy-registration and periodic-condition facade.
use super::*;
use crate::foundation::owner_timer::SemanticTimeMicros as RuntimeSemanticTimeMicros;
use crate::foundation::{
    ApplicationFacts, ChannelContentPin, ConditionDefinition, ConditionSourceKind, ConditionValues,
    DotElement, MovementLocalPosition, NodeId, RuntimeScopeRefV1, TickFacts, WorldId,
};
use oteryn_gameplay_content::content::native_gameplay::callback_source_native_test_policies;
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;
fn checked<T, E: std::fmt::Debug>(value: Result<T, E>) -> TestResult<T> {
    value.map_err(|error| format!("{error:?}").into())
}
fn id(tag: u8) -> [u8; 16] {
    [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
}
fn native_magnolia() -> TestResult<(ChannelRuntimeV1, ExactActorRef)> {
    let world = checked(WorldId::decode(&id(1)))?;
    let pin = ChannelContentPin::test(world);
    let mut runtime = checked(ChannelRuntimeV1::from_committed_scope_assignment(
        RuntimeScopeRefV1::Instance {
            world_id: world,
            instance_id: id(2),
        },
        checked(NodeId::decode(&id(3)))?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        pin.clone(),
    ))?;
    let creatures = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/creatures/definitions/spell-native-profiles.json"
    ))?;
    let presentations = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/presentations/definitions/spell-native-profiles.json"
    ))?;
    checked(
        runtime.install_companion_policies(checked(callback_source_native_test_policies(
            &creatures,
            &presentations,
            [1; 32],
        ))?),
    )?;
    // This existing admission facade installs the real active native policy.
    // There is no direct call to the proposed Magnolia phase binder here.
    let actual = checked(runtime.realize_native_qualification_spawn(
        pin,
        "oteryn:creature.the_moonsnow_magnolia",
        MovementLocalPosition {
            x: 1,
            y: 0,
            floor: 0,
        },
    ))?;
    assert_eq!((actual.health, actual.maximum_health), (52_000, 52_000));
    Ok((runtime, actual.actor))
}
fn install_field_dot(runtime: &mut ChannelRuntimeV1, actor: ExactActorRef) -> TestResult<()> {
    install_field_damage(runtime, actor, false)
}
fn install_field_damage(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    agony: bool,
) -> TestResult<()> {
    let before = checked(runtime.companion_snapshot(actor))?;
    let mut next = before.state.clone();
    // Controlled overkill input, not an invented Magnolia attack value.
    let values = if agony {
        ConditionValues::Agony {
            total_min: 200_000,
            total_max: 200_000,
            per_tick: 100_000,
            interval_ms: 1000,
            delayed: true,
        }
    } else {
        ConditionValues::DamageOverTime {
            element: DotElement::Fire,
            total_min: 200_000,
            total_max: 200_000,
            per_tick: 100_000,
            interval_ms: 1000,
            delayed: true,
        }
    };
    let dot = ConditionDefinition::new("test.magnolia.periodic", 1, values)
        .ok_or("valid controlled native periodic condition")?;
    let root = GameplayDecisionRoot::from_bytes([8; 32]);
    checked(next.conditions.apply(
        &dot,
        None,
        ConditionSourceKind::Field,
        &[],
        &ApplicationFacts {
            now: 0,
            base_speed: u16::try_from(before.state.policy.base_speed)?,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: false,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes(id(4)),
        },
    ))?;
    checked(runtime.compare_companion_state(&before, next))?;
    Ok(())
}
#[test]
fn srv7_magnolia_native_registration_dot_first_life_final_death_and_retry() -> TestResult<()> {
    let (mut runtime, actor) = native_magnolia()?;
    install_field_dot(&mut runtime, actor)?;
    let states = ChannelSpellStates::default();
    let first = checked(states.tick_source_creature_conditions(
        &mut runtime,
        actor,
        TickFacts::default(),
        RuntimeSemanticTimeMicros::from_micros(1_000_000),
    ))?;
    assert_eq!(
        first
            .impacts
            .iter()
            .map(|impact| impact.damage)
            .sum::<u64>(),
        52_000
    );
    let after = checked(runtime.companion_snapshot(actor))?;
    assert_eq!(
        (after.actor, after.health, after.maximum_health),
        (actor, 60_000, 60_000)
    );
    assert!(
        runtime
            .borrow_combat_death()
            .committed_lethal_receipt(actor)
            .is_err()
    );
    let retry = checked(states.tick_source_creature_conditions(
        &mut runtime,
        actor,
        TickFacts::default(),
        RuntimeSemanticTimeMicros::from_micros(1_000_000),
    ))?;
    assert!(retry.ticks.is_empty());
    assert_eq!(retry.health_after, 60_000);
    let final_hit = checked(states.tick_source_creature_conditions(
        &mut runtime,
        actor,
        TickFacts::default(),
        RuntimeSemanticTimeMicros::from_micros(2_000_000),
    ))?;
    assert_eq!(
        final_hit
            .impacts
            .iter()
            .map(|impact| impact.damage)
            .sum::<u64>(),
        60_000
    );
    assert_eq!(final_hit.health_after, 0);
    let death = checked(
        runtime
            .borrow_combat_death()
            .committed_lethal_receipt(actor),
    )?;
    let projection = checked(
        runtime
            .borrow_combat_death()
            .project_committed_lethal(death),
    )?
    .clone();
    assert_eq!(projection.occurrence.actor(), actor);
    assert!(
        states
            .tick_source_creature_conditions(
                &mut runtime,
                actor,
                TickFacts::default(),
                RuntimeSemanticTimeMicros::from_micros(2_000_000),
            )
            .is_err(),
        "a finalized dead actor cannot be ticked or resurrected"
    );
    let replay = checked(
        runtime
            .borrow_combat_death()
            .committed_lethal_receipt(actor),
    )?;
    let replay_projection = checked(
        runtime
            .borrow_combat_death()
            .project_committed_lethal(replay),
    )?
    .clone();
    assert_eq!(replay_projection, projection);
    Ok(())
}
#[test]
fn srv7_magnolia_native_registration_two_due_dot_roots_heal_between_then_die() -> TestResult<()> {
    let (mut runtime, actor) = native_magnolia()?;
    install_field_dot(&mut runtime, actor)?;
    let states = ChannelSpellStates::default();
    let both = checked(states.tick_source_creature_conditions(
        &mut runtime,
        actor,
        TickFacts::default(),
        RuntimeSemanticTimeMicros::from_micros(2_000_000),
    ))?;
    assert_eq!(both.ticks.len(), 2);
    assert_eq!(
        both.impacts.iter().map(|impact| impact.damage).sum::<u64>(),
        112_000
    );
    assert_eq!(both.health_after, 0);
    let death = checked(
        runtime
            .borrow_combat_death()
            .committed_lethal_receipt(actor),
    )?;
    let projection = checked(
        runtime
            .borrow_combat_death()
            .project_committed_lethal(death),
    )?
    .clone();
    assert_eq!(projection.occurrence.actor(), actor);
    Ok(())
}

#[test]
fn srv7_magnolia_native_registration_agony_catchup_uses_same_health_owner() -> TestResult<()> {
    let (mut runtime, actor) = native_magnolia()?;
    install_field_damage(&mut runtime, actor, true)?;
    let both = checked(
        ChannelSpellStates::default().tick_source_creature_conditions(
            &mut runtime,
            actor,
            TickFacts::default(),
            RuntimeSemanticTimeMicros::from_micros(2_000_000),
        ),
    )?;
    assert_eq!(both.ticks.len(), 2);
    assert_eq!(
        both.impacts.iter().map(|impact| impact.damage).sum::<u64>(),
        112_000
    );
    assert_eq!(both.health_after, 0);
    let death = checked(
        runtime
            .borrow_combat_death()
            .committed_lethal_receipt(actor),
    )?;
    let projection = checked(
        runtime
            .borrow_combat_death()
            .project_committed_lethal(death),
    )?
    .clone();
    assert_eq!(projection.occurrence.actor(), actor);
    assert_eq!(projection.occurrence.health_before(), 60_000);
    Ok(())
}
