#[test]
fn spell_rl_01_matches_the_registered_row() {
    let registry: serde_json::Value =
        serde_json::from_str(oteryn_protocol_oteryn::RESOURCE_LIMITS_REGISTRY_JSON)
            .expect("registry");
    let row = registry["entries"]
        .as_array()
        .and_then(|entries| entries.iter().find(|entry| entry["id"] == "SPELL-RL-01"))
        .and_then(|entry| entry["hard_maximum"].as_u64());
    assert_eq!(
        row,
        u64::try_from(SPELL_RL_01_CAST_INPUTS_PER_WORK_CYCLE).ok()
    );
}

#[test]
fn a_cast_commits_once_and_a_repeated_proposal_never_pays_twice() {
    let (runtime, actor, session) = runtime_with_player(0x63);
    let mut states = ChannelSpellStates::default();
    states
        .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
        .expect("initialized");
    let full = ActorVitals {
        health: 185,
        max_health: 185,
        mana: 90,
        max_mana: 90,
        soul: 100,
        harmony: 0,
        serene: false,
    };
    assert_eq!(
        observe_vitals(&runtime, &states, actor, session),
        Some((1, full))
    );
    let cast = cast_at(&runtime, &mut states, actor, session, 1, 0);
    assert_eq!(cast.disposition, SpellCastDisposition::Cast);
    let (revision, vitals) = cast.vitals.expect("committed vitals");
    assert_eq!((revision, vitals.mana, vitals.health), (2, 70, 185));
    // A second input while cooling down is answered and pays nothing.
    assert_eq!(
        cast_at(&runtime, &mut states, actor, session, 2, 500),
        SpellCastOutcome {
            disposition: SpellCastDisposition::CoolingDown,
            vitals: None,
        }
    );
    assert_eq!(
        observe_vitals(&runtime, &states, actor, session),
        Some((2, vitals))
    );
    // The committed state proposed again is not a successor: compare-commit refuses it.
    let committed = states.get(&runtime, actor, session).expect("state").clone();
    assert!(!states.commit(&runtime, actor, session, committed));
    assert_eq!(
        observe_vitals(&runtime, &states, actor, session),
        Some((2, vitals))
    );
}

#[test]
fn reinitializing_a_present_actor_keeps_its_vitals_and_cooldowns() {
    let (runtime, actor, session) = runtime_with_player(0x64);
    let mut states = ChannelSpellStates::default();
    states
        .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
        .expect("initialized");
    cast_at(&runtime, &mut states, actor, session, 1, 0);
    // A same-GameSession reconnect initializes again: no refill, no cooldown reset.
    let kept = states
        .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
        .expect("kept")
        .clone();
    assert_eq!((kept.revision(), kept.vitals().mana), (2, 70));
    assert_eq!(
        cast_at(&runtime, &mut states, actor, session, 2, 1),
        SpellCastOutcome {
            disposition: SpellCastDisposition::CoolingDown,
            vitals: None,
        }
    );
}

/// CHAR-VITALS-1: a newly created actor takes its saved vitals capped at its maxima, with no
/// revision; a present actor keeps its own; a saved full actor (`None`) and a zero health
/// leave the maxima; the save reads `None` at the maxima and the live values otherwise.
#[test]
fn an_admitted_actor_restores_its_saved_vitals_only_when_created() {
    let (runtime, actor, session) = runtime_with_player(0x7a);
    let mut states = ChannelSpellStates::default();
    let created = states
        .initialize_admitted(
            &runtime,
            actor,
            session,
            FACTS,
            (0, 0),
            now(0),
            Some((40, 500)),
            None,
        )
        .expect("created")
        .clone();
    assert_eq!(
        (
            created.revision(),
            created.vitals().health,
            created.vitals().mana
        ),
        (1, 40, 90)
    );
    assert_eq!(
        states.saved_vitals(&runtime, actor, session),
        Some(Some((40, 90)))
    );
    // A retry or a same-GameSession reconnect keeps the live vitals.
    wound(&mut states, actor, session, 50);
    let kept = states
        .initialize_admitted(
            &runtime,
            actor,
            session,
            FACTS,
            (0, 0),
            now(0),
            Some((7, 7)),
            None,
        )
        .expect("kept")
        .clone();
    assert_eq!((kept.vitals().health, kept.vitals().mana), (50, 90));

    for (seed, saved) in [(0x7b, None), (0x7c, Some((0, 10)))] {
        let (runtime, actor, session) = runtime_with_player(seed);
        let mut states = ChannelSpellStates::default();
        let fresh = states
            .initialize_admitted(&runtime, actor, session, FACTS, (0, 0), now(0), saved, None)
            .expect("created")
            .clone();
        assert_eq!(
            (fresh.vitals().health, fresh.vitals().mana),
            (FACTS.max_health, FACTS.max_mana),
            "{saved:?}"
        );
        assert_eq!(states.saved_vitals(&runtime, actor, session), Some(None));
    }
    // Another session's actor has no spell state: the stored vitals are kept.
    let other = GameSessionId::decode(&uuid_v7(0x7d)).expect("session");
    assert_eq!(states.saved_vitals(&runtime, actor, other), None);
}

#[test]
fn a_foreign_session_or_an_ended_actor_is_rejected_and_its_state_dropped() {
    let (mut runtime, actor, session) = runtime_with_player(0x65);
    let other = GameSessionId::decode(&uuid_v7(0x66)).expect("session");
    let mut states = ChannelSpellStates::default();
    assert!(
        states
            .initialize(&runtime, actor, other, FACTS, (0, 0), now(0))
            .is_none()
    );
    states
        .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
        .expect("initialized");
    assert_eq!(
        cast_at(&runtime, &mut states, actor, other, 1, 0),
        SpellCastOutcome::rejected()
    );
    assert_eq!(observe_vitals(&runtime, &states, actor, other), None);
    // The actor ends: its state is unreachable and the next initialization drops it.
    runtime
        .remove_terminal_session(session, actor)
        .expect("removed");
    assert_eq!(observe_vitals(&runtime, &states, actor, session), None);
    assert_eq!(
        cast_at(&runtime, &mut states, actor, session, 2, 0),
        SpellCastOutcome::rejected()
    );
    let reservation = runtime.reserve_fresh_session(other).expect("reserve");
    let successor = runtime.commit_fresh_session(reservation).expect("commit");
    let fresh = states
        .initialize(&runtime, successor, other, FACTS, (0, 0), now(0))
        .expect("fresh actor")
        .clone();
    assert_eq!((fresh.revision(), fresh.vitals().mana), (1, 90));
    assert_eq!(states.actors.len(), 1);
}

#[test]
fn without_character_cast_facts_casting_stays_gated() {
    let (runtime, actor, session) = runtime_with_player(0x67);
    let mut states = ChannelSpellStates::default();
    assert_eq!(observe_vitals(&runtime, &states, actor, session), None);
    assert_eq!(
        cast_at(&runtime, &mut states, actor, session, 1, 0),
        SpellCastOutcome::rejected()
    );
}

#[test]
fn the_occurrence_and_its_draws_are_bound_to_the_command() {
    let outcome = |command_id| {
        let (runtime, actor, session) = runtime_with_player(0x68);
        let mut states = ChannelSpellStates::default();
        states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
            .expect("state");
        cast_at(&runtime, &mut states, actor, session, command_id, 0)
    };
    assert_eq!(outcome(7), outcome(7));
}

/// SPELL-D8 §8.2 in the Channel owner: a new monk actor loads the durable values and is
/// Serene from its initialization evaluation; a reconnect keeps Harmony and the forced time,
/// and the actor-end save reads the live values.
#[test]
fn a_monk_actor_loads_its_durable_values_and_keeps_them_across_a_reconnect() {
    let (runtime, actor, session) = runtime_with_player(0x69);
    let monk = CharacterCastFacts {
        vocation: Vocation::Monk,
        ..FACTS
    };
    let mut states = ChannelSpellStates::default();
    assert!(
        states
            .initialize(&runtime, actor, session, monk, (6, 0), now(0))
            .is_none(),
        "a corrupt durable Harmony fails the actor closed"
    );
    let state = states
        .initialize(&runtime, actor, session, monk, (3, 4_000_000), now(1000))
        .expect("monk actor");
    assert_eq!((state.vitals().harmony, state.vitals().serene), (3, true));
    assert_eq!(
        states.monk_save_values(&runtime, actor, session, now(2000)),
        Some((3, 3_000_000))
    );
    // A solo monk's periodic evaluation changes nothing, so nothing is published.
    assert_eq!(states.tick(&runtime, actor, session, now(2000)), None);
    // Control loss detaches the actor: its casts are refused until a recovery.
    states.detach(&runtime, actor, session);
    assert_eq!(
        cast_at(&runtime, &mut states, actor, session, 1, 2500),
        SpellCastOutcome::rejected()
    );
    assert!(states.resume(&runtime, actor, session, now(3000)));
    // A present actor keeps its values: a second initialization reloads nothing.
    let kept = states
        .initialize(&runtime, actor, session, monk, (0, 0), now(3000))
        .expect("kept");
    assert_eq!(kept.vitals().harmony, 3);
    assert_eq!(
        states.monk_save_values(&runtime, actor, session, now(3000)),
        Some((3, 2_000_000))
    );
    assert_eq!(
        cast_at(&runtime, &mut states, actor, session, 2, 3000).disposition,
        SpellCastDisposition::Cast
    );
    // A druid has no monk values to save.
    let (druid_runtime, druid, druid_session) = runtime_with_player(0x6a);
    let mut druids = ChannelSpellStates::default();
    druids
        .initialize(&druid_runtime, druid, druid_session, FACTS, (0, 0), now(0))
        .expect("druid");
    assert_eq!(
        druids.monk_save_values(&druid_runtime, druid, druid_session, now(0)),
        None
    );
}
