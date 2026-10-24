#![allow(
    clippy::panic,
    reason = "test-only native outcome destructuring asserts impossible fixture variants; same scope as adjacent canonical owner tests"
)]
use super::*;
use oteryn_gameplay_domain::domain::condition::ConditionValues;
use oteryn_gameplay_domain::domain::condition::DotElement;
use oteryn_gameplay_foundation::foundation::condition::ApplicationFacts;
use oteryn_gameplay_foundation::foundation::condition::ConditionDefinition;
use oteryn_gameplay_foundation::foundation::condition::DamageSchedule;
use oteryn_gameplay_foundation::foundation::condition::DamageSegment;
use oteryn_gameplay_foundation::foundation::condition::TickFacts;
fn ready(
    tag: u8,
) -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    ExactActorRef,
    GameSessionId,
) {
    let (mut r, a, g) = super::tests::runtime_with_player(tag);
    assert!(r.initialize_first_entry_position(a).is_ok());
    let mut s = ChannelSpellStates::default();
    assert!(
        s.initialize(
            &r,
            a,
            g,
            super::tests::FACTS,
            (0, 0),
            SemanticTimeMicros::from_micros(0)
        )
        .is_some()
    );
    (r, s, a, g)
}
fn install(
    s: &mut ChannelSpellStates,
    r: &ChannelRuntimeV1,
    a: ExactActorRef,
    g: GameSessionId,
    defs: &[ConditionDefinition],
) {
    let root = GameplayDecisionRoot::from_bytes([111; 32]);
    let facts = ApplicationFacts {
        now: 0,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([111; 16]),
    };
    let Some(before) = s.get(r, a, g) else {
        panic!("native state")
    };
    let Some(next) = before.stage_owned_test_conditions(defs, &facts) else {
        panic!("typed fixture")
    };
    assert!(s.commit(r, a, g, next));
}
fn poison(amount: u32) -> ConditionDefinition {
    let Some(d) = ConditionDefinition::new_damage_schedule(
        "test.source.poison",
        1,
        DotElement::Poison,
        DamageSchedule::Fixed {
            delayed: true,
            segments: vec![DamageSegment {
                count: 3,
                interval_ms: 1000,
                amount,
            }],
        },
    ) else {
        panic!("source schedule")
    };
    d
}
// TEST_PARAMETERS: real captured monster regeneration count is zero, NOT_APPLICABLE.
fn regen() -> ConditionDefinition {
    let Some(d) = ConditionDefinition::new(
        "test.typed.native.regeneration",
        1,
        ConditionValues::SpellRegeneration {
            duration_ms: 10000,
            sub_id: 0,
            health_gain: 5,
            health_interval_ms: 1000,
            mana_gain: 7,
            mana_interval_ms: 2000,
        },
    ) else {
        panic!("typed regeneration")
    };
    d
}
fn pass(
    s: &mut ChannelSpellStates,
    r: &mut ChannelRuntimeV1,
    a: ExactActorRef,
    g: GameSessionId,
    t: u64,
    f: TickFacts,
) -> Option<NativeOwnedPlayerTickReceipt> {
    s.tick_source_player_conditions(
        r,
        a,
        g,
        DecisionOccurrenceId::from_bytes([112; 16]),
        f,
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(t),
    )
}
#[test]
fn actual_creature_field_periodic_routes_to_death_owner_and_failed_mint_keeps_cursor() {
    let (mut runtime, mut states, actor, session) = ready(0xb8);
    install(&mut states, &runtime, actor, session, &[poison(200)]);
    let before = states.get(&runtime, actor, session).cloned();
    let mint = states.mint_death;
    states.mint_death = || None;
    let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1_000_000);
    assert_eq!(
        states.tick_current_creature_field_damage(
            &mut runtime,
            actor,
            session,
            now,
            TickFacts::default(),
            None
        ),
        Some(None)
    );
    assert_eq!(states.get(&runtime, actor, session), before.as_ref());
    assert!(states.deaths.is_empty());
    states.mint_death = mint;
    let result = states.tick_current_creature_field_damage(
        &mut runtime,
        actor,
        session,
        now,
        TickFacts::default(),
        None,
    );
    assert!(matches!(
        result,
        Some(Some((_, ActorVitals { health: 0, .. })))
    ));
    assert!(states.player_death(&runtime, actor, session).is_some());
    assert!(
        states
            .get(&runtime, actor, session)
            .is_some_and(|s| s.owned_conditions().instances().is_empty())
    );
    let retained = states.player_death(&runtime, actor, session);
    assert!(
        states
            .tick_current_creature_field_damage(
                &mut runtime,
                actor,
                session,
                now,
                TickFacts::default(),
                None
            )
            .is_none()
    );
    assert_eq!(states.player_death(&runtime, actor, session), retained);
}
#[test]
fn actual_lethal_creature_agony_routes_to_the_death_owner() {
    let (mut runtime, mut states, actor, session) = ready(0xbb);
    let Some(agony) = ConditionDefinition::new(
        "test.source.agony",
        1,
        ConditionValues::Agony {
            total_min: 100_000,
            total_max: 100_000,
            per_tick: 100_000,
            interval_ms: 1_000,
            delayed: true,
        },
    ) else {
        panic!("agony fixture")
    };
    install(&mut states, &runtime, actor, session, &[agony]);
    let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1_000_000);
    let result = states.tick_current_creature_field_damage(
        &mut runtime,
        actor,
        session,
        now,
        TickFacts::default(),
        None,
    );
    assert!(matches!(
        result,
        Some(Some((_, ActorVitals { health: 0, .. })))
    ));
    assert!(states.player_death(&runtime, actor, session).is_some());
    assert!(
        states
            .get(&runtime, actor, session)
            .is_some_and(|s| s.owned_conditions().instances().is_empty())
    );
}
#[test]
fn actual_malformed_creature_field_origin_refuses_without_hp_or_cursor_write() {
    let (mut runtime, mut states, actor, session) = ready(0xba);
    let before = states.get(&runtime, actor, session).cloned();
    let Some(state) = before.as_ref() else {
        panic!("actual player owner");
    };
    let root = GameplayDecisionRoot::from_bytes([113; 32]);
    let facts = ApplicationFacts {
        now: 0,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([113; 16]),
    };
    let Some((next, _)) = state.stage_creature_field_contact(
        "creature:field:{malformed}".into(),
        &poison(10),
        &facts,
        DotElement::Poison,
    ) else {
        panic!("TEST_PARAMETERS malformed historical source in actual canonical store");
    };
    assert!(states.commit(&runtime, actor, session, next));
    let before = states.get(&runtime, actor, session).cloned();
    let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1_000_000);
    assert_eq!(
        states.tick_current_creature_field_damage(
            &mut runtime,
            actor,
            session,
            now,
            TickFacts::default(),
            None
        ),
        Some(None)
    );
    assert_eq!(states.get(&runtime, actor, session), before.as_ref());
    assert!(states.deaths.is_empty());
}
#[test]
fn actual_creature_field_pz_preserves_hp_then_due_residual_uses_same_store() {
    let (mut runtime, mut states, actor, session) = ready(0xb9);
    install(&mut states, &runtime, actor, session, &[poison(10)]);
    let health = states
        .get(&runtime, actor, session)
        .map(|s| s.vitals().health);
    let pz = TickFacts {
        in_protection_zone: true,
        standing_on_field: None,
    };
    let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1_000_000);
    assert!(
        states
            .tick_current_creature_field_damage(&mut runtime, actor, session, now, pz, None)
            .is_some()
    );
    assert_eq!(
        states
            .get(&runtime, actor, session)
            .map(|s| s.vitals().health),
        health
    );
    let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(2_000_000);
    assert!(
        states
            .tick_current_creature_field_damage(
                &mut runtime,
                actor,
                session,
                now,
                TickFacts::default(),
                None
            )
            .is_some()
    );
    assert!(
        states
            .get(&runtime, actor, session)
            .is_some_and(|s| Some(s.vitals().health) < health)
    );
    assert!(
        runtime
            .actor_conditions(actor, Some(session))
            .is_ok_and(|s| s.instances().is_empty())
    );
}

#[test]
fn owned_tick_failed_mint_keeps_entire_state_then_lethal_stops_regeneration() {
    let (mut r, mut s, a, g) = ready(0xa1);
    install(&mut s, &r, a, g, &[poison(200), regen()]);
    let before = s.get(&r, a, g).cloned();
    let mint = s.mint_death;
    s.mint_death = || None;
    assert!(pass(&mut s, &mut r, a, g, 1000000, TickFacts::default()).is_none());
    assert_eq!(s.get(&r, a, g), before.as_ref());
    assert!(s.deaths.is_empty());
    s.mint_death = mint;
    let Some(receipt) = pass(&mut s, &mut r, a, g, 1000000, TickFacts::default()) else {
        panic!("owned lethal")
    };
    assert_eq!(receipt.damage, 185);
    assert!(receipt.death.is_some());
    let Some(state) = s.get(&r, a, g) else {
        panic!("nativeHP")
    };
    assert_eq!(state.vitals().health, 0);
    assert!(state.owned_conditions().instances().is_empty());
    assert_eq!(s.deaths.len(), 1);
    assert!(pass(&mut s, &mut r, a, g, 1000000, TickFacts::default()).is_none());
    assert_eq!(s.deaths.len(), 1);
}
#[test]
fn owned_typed_regeneration_distinct_intervals_and_unknown_gain_refuses_without_cursor() {
    let (mut r, mut s, a, g) = ready(0xa2);
    super::tests::wound(&mut s, a, g, 100);
    let Some(state) = s.get(&r, a, g) else {
        panic!("current MP")
    };
    let Some((wounded, _)) = state.after_creature_mana_drain(20) else {
        panic!("MP stage")
    };
    assert!(s.commit(&r, a, g, wounded));
    install(&mut s, &r, a, g, &[regen()]);
    let Some(first) = pass(&mut s, &mut r, a, g, 1000000, TickFacts::default()) else {
        panic!("first interval")
    };
    assert_eq!(first.damage, 0);
    assert_eq!(s.get(&r, a, g).map(|v| v.vitals().health), Some(105));
    assert_eq!(s.get(&r, a, g).map(|v| v.vitals().mana), Some(70));
    let Some(_) = pass(&mut s, &mut r, a, g, 2000000, TickFacts::default()) else {
        panic!("second interval")
    };
    assert_eq!(s.get(&r, a, g).map(|v| v.vitals().health), Some(110));
    assert_eq!(s.get(&r, a, g).map(|v| v.vitals().mana), Some(77));
    let Some(unknown) = ConditionDefinition::new(
        "test.unknown.recovery",
        1,
        ConditionValues::Recovery {
            duration_ms: 10000,
            interval_ms: 1000,
        },
    ) else {
        panic!("unknown source")
    };
    let (mut r, mut s, a, g) = ready(0xa3);
    install(&mut s, &r, a, g, &[unknown]);
    let before = s.get(&r, a, g).cloned();
    assert!(pass(&mut s, &mut r, a, g, 1000000, TickFacts::default()).is_none());
    assert_eq!(s.get(&r, a, g), before.as_ref());
}
#[test]
fn owned_tick_pz_refuses_damage_and_field_retains_compressed_source_budget() {
    let (mut r, mut s, a, g) = ready(0xa4);
    install(&mut s, &r, a, g, &[poison(5)]);
    let Some(receipt) = pass(
        &mut s,
        &mut r,
        a,
        g,
        1000000,
        TickFacts {
            in_protection_zone: true,
            standing_on_field: None,
        },
    ) else {
        panic!("current PZ")
    };
    assert_eq!(receipt.damage, 0);
    assert_eq!(s.get(&r, a, g).map(|v| v.vitals().health), Some(185));
    let (mut r, mut s, a, g) = ready(0xa5);
    install(&mut s, &r, a, g, &[poison(5)]);
    let Some(_) = pass(
        &mut s,
        &mut r,
        a,
        g,
        3000000,
        TickFacts {
            in_protection_zone: false,
            standing_on_field: Some(DotElement::Poison),
        },
    ) else {
        panic!("actual field facts")
    };
    assert_eq!(s.get(&r, a, g).map(|v| v.vitals().health), Some(170));
    assert_eq!(
        s.get(&r, a, g)
            .map(|v| v.owned_conditions().instances().len()),
        Some(1)
    );
}
#[test]
fn owned_old_staged_state_rejected_after_current_revision_commit() {
    let (r, mut s, a, g) = ready(0xa6);
    let Some(state) = s.get(&r, a, g) else {
        panic!("current")
    };
    let Some((stale, _)) = state.after_creature_mana_drain(1) else {
        panic!("staged")
    };
    let current = stale.clone();
    assert!(s.commit(&r, a, g, current));
    let before = s.get(&r, a, g).cloned();
    assert!(!s.commit(&r, a, g, stale));
    assert_eq!(s.get(&r, a, g), before.as_ref());
}
