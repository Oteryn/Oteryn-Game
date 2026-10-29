#![allow(
    clippy::panic,
    reason = "test-only fixture setup asserts impossible fixture variants"
)]
use super::*;
use crate::gameplay_transport::actor_spell::tests::{FACTS, runtime_with_player};
use oteryn_gameplay_domain::domain::condition::ConditionType;
use oteryn_gameplay_foundation::foundation::condition::TickFacts;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::collections::BTreeSet;

const SECOND: u64 = 1_000_000;

fn ready(
    tag: u8,
) -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    ExactActorRef,
    GameSessionId,
) {
    let (mut runtime, actor, session) = runtime_with_player(tag);
    assert!(runtime.initialize_first_entry_position(actor).is_ok());
    let mut states = ChannelSpellStates::default();
    assert!(
        states
            .initialize(
                &runtime,
                actor,
                session,
                FACTS,
                (0, 0),
                SemanticTimeMicros::from_micros(0),
            )
            .is_some()
    );
    (runtime, states, actor, session)
}

fn drowns(
    states: &ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> bool {
    states.get(runtime, actor, session).is_some_and(|state| {
        state
            .owned_conditions()
            .instances()
            .iter()
            .any(|instance| is_environment_drown(instance.provenance()))
    })
}

fn health(
    states: &ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> u64 {
    states
        .get(runtime, actor, session)
        .map_or(0, |state| u64::from(state.vitals().health))
}

fn tick(
    states: &mut ChannelSpellStates,
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    now: u64,
) -> Option<u32> {
    states
        .tick_source_player_conditions(
            runtime,
            actor,
            session,
            DecisionOccurrenceId::from_bytes([7; 16]),
            TickFacts::default(),
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
        )
        .map(|receipt| receipt.damage)
}

#[test]
fn the_drown_definition_is_five_delayed_global_ticks_of_twenty() {
    let Some(definition) = drown_definition() else {
        panic!("drown definition");
    };
    assert_eq!(
        definition.condition_type(),
        ConditionType::DamageOverTime(DotElement::Drown)
    );
    assert_eq!(
        definition.values(),
        ConditionValues::DamageOverTime {
            element: DotElement::Drown,
            total_min: 100,
            total_max: 100,
            per_tick: 20,
            interval_ms: 4_000,
            delayed: true,
        }
    );
    assert_eq!(UNDERWATER_GROUND_IDS.len(), 12);
}

fn wound(
    states: &mut ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) {
    let Some(state) = states.get_mut(runtime, actor, session) else {
        panic!("state");
    };
    let Ok((next, _)) = crate::spell::actor_conditions::stage_creature_hit(state, 50, 0) else {
        panic!("wound");
    };
    *state = next;
}

#[test]
fn unprotected_entry_drowns_twenty_every_four_seconds_five_times() {
    let (mut runtime, mut states, actor, session) = ready(0xd1);
    // The always-on native regeneration clock also credits health; an identical actor that
    // never enters the water isolates the drown damage from it. Both start below full health,
    // so neither regeneration is capped.
    let (mut dry_runtime, mut dry_states, dry_actor, dry_session) = ready(0xd6);
    wound(&mut states, &runtime, actor, session);
    wound(&mut dry_states, &dry_runtime, dry_actor, dry_session);
    let revision = states.get(&runtime, actor, session).map(|s| s.revision());
    assert!(states.stage_underwater_step(&runtime, actor, session, 0, false, true));
    assert!(drowns(&states, &runtime, actor, session));
    assert_eq!(
        states.get(&runtime, actor, session).map(|s| s.revision()),
        revision.map(|r| r + 1)
    );
    let start = health(&states, &runtime, actor, session);
    assert!(start > 100);
    assert_eq!(
        health(&dry_states, &dry_runtime, dry_actor, dry_session),
        start
    );
    let mut both = |now: u64| {
        let damage = tick(&mut states, &mut runtime, actor, session, now);
        let dry = tick(
            &mut dry_states,
            &mut dry_runtime,
            dry_actor,
            dry_session,
            now,
        );
        assert_eq!(dry, Some(0));
        (
            damage,
            health(&dry_states, &dry_runtime, dry_actor, dry_session)
                - health(&states, &runtime, actor, session),
        )
    };
    // Delayed: nothing is due before four seconds.
    assert_eq!(both(4 * SECOND - 1), (Some(0), 0));
    for n in 1..=5u64 {
        assert_eq!(both(4 * n * SECOND), (Some(20), 20 * n), "tick {n}");
    }
    assert_eq!(both(24 * SECOND), (Some(0), 100));
    assert!(!drowns(&states, &runtime, actor, session));
}

#[test]
fn a_suppressed_helmet_keeps_the_state_unchanged() {
    let (runtime, mut states, actor, session) = ready(0xd2);
    let Some(state) = states.get_mut(&runtime, actor, session) else {
        panic!("state");
    };
    state
        .conditions
        .set_suppressed(BTreeSet::from([ConditionType::DamageOverTime(
            DotElement::Drown,
        )]));
    let before = states.get(&runtime, actor, session).cloned();
    assert!(!states.stage_underwater_step(&runtime, actor, session, 0, false, true));
    assert!(!drowns(&states, &runtime, actor, session));
    assert_eq!(states.get(&runtime, actor, session), before.as_ref());
}

#[test]
fn leaving_removes_drown_and_moving_underwater_restarts_it() {
    let (mut runtime, mut states, actor, session) = ready(0xd3);
    assert!(states.stage_underwater_step(&runtime, actor, session, 0, false, true));
    // Underwater to underwater: StepOut then StepIn, so the first tick moves to 3 s + 4 s.
    assert!(states.stage_underwater_step(&runtime, actor, session, 3 * SECOND, true, true));
    assert_eq!(
        tick(&mut states, &mut runtime, actor, session, 4 * SECOND),
        Some(0)
    );
    assert_eq!(
        tick(&mut states, &mut runtime, actor, session, 7 * SECOND),
        Some(20)
    );
    let revision = states.get(&runtime, actor, session).map(|s| s.revision());
    assert!(states.stage_underwater_step(&runtime, actor, session, 8 * SECOND, true, false));
    assert!(!drowns(&states, &runtime, actor, session));
    assert_eq!(
        states.get(&runtime, actor, session).map(|s| s.revision()),
        revision.map(|r| r + 1)
    );
    // Dry ground to dry ground changes nothing.
    assert!(!states.stage_underwater_step(&runtime, actor, session, 9 * SECOND, false, false));
}

#[test]
fn the_periodic_owner_deals_environment_drown() {
    let (mut runtime, mut states, actor, session) = ready(0xd4);
    assert!(states.stage_underwater_step(&runtime, actor, session, 0, false, true));
    let start = health(&states, &runtime, actor, session);
    let result = states.tick_current_creature_field_damage(
        &mut runtime,
        actor,
        session,
        SemanticTimeMicros::from_micros(4 * SECOND),
        TickFacts::default(),
        None,
    );
    assert!(matches!(result, Some(Some(_))));
    assert_eq!(health(&states, &runtime, actor, session), start - 20);
}

#[test]
fn another_field_source_is_not_environment_drown() {
    let (mut runtime, mut states, actor, session) = ready(0xd5);
    let Some(definition) = drown_definition() else {
        panic!("drown definition");
    };
    let root = GameplayDecisionRoot::from_bytes([1; 32]);
    let facts = ApplicationFacts {
        now: 0,
        base_speed: 1,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
    };
    let Some(state) = states.get_mut(&runtime, actor, session) else {
        panic!("state");
    };
    assert!(
        state
            .conditions
            .apply(
                &definition,
                Some("environment:other".to_owned()),
                ConditionSourceKind::Field,
                &[],
                &facts,
            )
            .is_ok()
    );
    assert!(!drowns(&states, &runtime, actor, session));
    assert_eq!(
        tick(&mut states, &mut runtime, actor, session, 4 * SECOND),
        None
    );
}
