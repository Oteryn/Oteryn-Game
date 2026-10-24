#[test]
fn the_transport_gate_answers_a_gated_cast_before_any_dispatcher() {
    let book = crate::spell::cast::v1_spell_book().expect("book");
    let (runtime, actor, session) = runtime_with_player(0x7a);
    let mut states = ChannelSpellStates::default();
    states
        .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
        .expect("initialized");
    let gated = |states: &ChannelSpellStates| {
        condition_gated_cast(&runtime, states, &book, actor, session, &exura(), now(1))
    };
    assert_eq!(gated(&states), None);
    // `exura` is not aggressive: `pacified` lets it through, `muted` refuses every spell.
    gate_player(&mut states, actor, session, "pacified");
    assert_eq!(gated(&states), None);
    gate_player(&mut states, actor, session, "muted");
    assert_eq!(gated(&states), Some(ConditionGate::Muted));
    assert_eq!(
        condition_gated_cast(
            &runtime,
            &states,
            &book,
            actor,
            session,
            &exura(),
            now(10_000)
        ),
        None
    );
}

#[test]
fn health_credit_commits_native_vitals_and_preserves_noops() {
    let (runtime, actor, session) = runtime_with_player(0x73);
    let mut states = ChannelSpellStates::default();
    states
        .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
        .expect("initialized");
    wound(&mut states, actor, session, 100);
    assert_eq!(
        states.apply_health_gain(&runtime, actor, session, 0),
        Some((0, 1))
    );
    assert_eq!(
        states.apply_health_gain(&runtime, actor, session, 7),
        Some((7, 2))
    );
    let (revision, vitals) = observe_vitals(&runtime, &states, actor, session).expect("observed");
    assert_eq!(
        (revision, vitals.health, vitals.mana, vitals.soul),
        (2, 107, 90, 100)
    );
    assert_eq!(
        states.apply_health_gain(&runtime, actor, session, u64::MAX),
        Some((78, 3))
    );
    let before = states
        .get(&runtime, actor, session)
        .expect("present")
        .clone();
    assert_eq!(
        states.apply_health_gain(&runtime, actor, session, 1),
        Some((0, 3))
    );
    assert_eq!(states.get(&runtime, actor, session), Some(&before));
}

#[test]
fn mana_credit_commits_native_vitals_and_preserves_noops() {
    let (runtime, actor, session) = runtime_with_player(0x74);
    let mut states = ChannelSpellStates::default();
    states
        .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
        .expect("initialized");
    assert_eq!(
        cast_at(&runtime, &mut states, actor, session, 1, 0).disposition,
        SpellCastDisposition::Cast
    );
    assert_eq!(
        states.apply_mana_gain(&runtime, actor, session, 0),
        Some((0, 2))
    );
    assert_eq!(
        states.apply_mana_gain(&runtime, actor, session, 7),
        Some((7, 3))
    );
    let (revision, vitals) = observe_vitals(&runtime, &states, actor, session).expect("observed");
    assert_eq!(
        (revision, vitals.mana, vitals.health, vitals.soul),
        (3, 77, 185, 100)
    );
    assert_eq!(
        states.apply_mana_gain(&runtime, actor, session, u64::MAX),
        Some((13, 4))
    );
    let before = states
        .get(&runtime, actor, session)
        .expect("present")
        .clone();
    assert_eq!(
        states.apply_mana_gain(&runtime, actor, session, 1),
        Some((0, 4))
    );
    assert_eq!(states.get(&runtime, actor, session), Some(&before));
}

#[test]
fn eaten_food_commits_as_the_direct_successor_revision() {
    let (runtime, actor, session) = runtime_with_player(0x77);
    let mut states = ChannelSpellStates::default();
    assert_eq!(states.apply_food(&runtime, actor, session, 30, 0), None);
    states
        .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
        .expect("initialized");
    assert!(states.spell_idle(&runtime, actor, session));
    let before = states
        .get(&runtime, actor, session)
        .expect("present")
        .revision();
    assert_eq!(
        states.apply_food(&runtime, actor, session, 30, 0),
        Some(before + 1)
    );
    assert_eq!(
        states
            .get(&runtime, actor, session)
            .expect("present")
            .revision(),
        before + 1
    );
    let full = states.apply_food(&runtime, actor, session, 1_200, 0);
    assert_eq!(full, None);
    assert_eq!(
        states
            .get(&runtime, actor, session)
            .expect("present")
            .revision(),
        before + 1
    );
}

#[test]
fn vitals_credit_refuses_foreign_sessions_and_ended_generations() {
    let (mut runtime, actor, session) = runtime_with_player(0x75);
    let other = GameSessionId::decode(&uuid_v7(0x76)).expect("other session");
    let mut states = ChannelSpellStates::default();
    assert!(
        states
            .apply_health_gain(&runtime, actor, session, 7)
            .is_none()
    );
    assert!(
        states
            .apply_mana_gain(&runtime, actor, session, 7)
            .is_none()
    );
    states
        .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
        .expect("initialized");
    assert_eq!(
        cast_at(&runtime, &mut states, actor, session, 1, 0).disposition,
        SpellCastDisposition::Cast
    );
    wound(&mut states, actor, session, 100);
    let before = states.actors.clone();
    assert!(
        states
            .apply_health_gain(&runtime, actor, other, 7)
            .is_none()
    );
    assert!(states.apply_mana_gain(&runtime, actor, other, 7).is_none());
    assert_eq!(states.actors, before);

    runtime
        .remove_terminal_session(session, actor)
        .expect("ended");
    assert!(
        states
            .apply_health_gain(&runtime, actor, session, 7)
            .is_none()
    );
    assert!(
        states
            .apply_mana_gain(&runtime, actor, session, 7)
            .is_none()
    );
    assert_eq!(states.actors, before);

    let reservation = runtime.reserve_fresh_session(other).expect("reserve");
    let successor = runtime
        .commit_fresh_session(reservation)
        .expect("successor");
    assert_ne!(actor, successor);
    states
        .initialize(&runtime, successor, other, FACTS, (0, 0), now(0))
        .expect("fresh state");
    assert!(
        states
            .apply_health_gain(&runtime, actor, session, 7)
            .is_none()
    );
    assert!(
        states
            .apply_mana_gain(&runtime, actor, session, 7)
            .is_none()
    );
    assert_eq!(
        states.apply_health_gain(&runtime, successor, other, 7),
        Some((0, 1))
    );
    assert_eq!(
        states.apply_mana_gain(&runtime, successor, other, 7),
        Some((0, 1))
    );
}
