/// A runtime with one positioned player at (10, 10), its vitals at a wounded `health`, and one
/// live creature beside it.
#[cfg(test)]
fn bitten_player(
    tag: u8,
    health: u32,
) -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    ExactActorRef,
    GameSessionId,
    ExactActorRef,
) {
    bitten_actor(tag, health, FACTS, (0, 0))
}

/// [`bitten_player`] with the actor's cast facts and durable monk values.
#[cfg(test)]
fn bitten_actor(
    tag: u8,
    health: u32,
    facts: CharacterCastFacts,
    monk: (u8, u64),
) -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    ExactActorRef,
    GameSessionId,
    ExactActorRef,
) {
    use crate::foundation::MovementLocalPosition;
    let (mut runtime, actor, session) = runtime_with_player(tag);
    let at = |x, y| MovementLocalPosition { x, y, floor: 7 };
    runtime
        .initialize_movement_test_position(actor, at(10, 10))
        .expect("player position");
    let creature = runtime.admit_test_creature(at(11, 10)).expect("creature");
    let mut states = ChannelSpellStates::default();
    states
        .initialize(&runtime, actor, session, facts, monk, now(0))
        .expect("vitals");
    wound(&mut states, actor, session, health);
    (runtime, states, actor, session, creature)
}

#[cfg(test)]
fn bite_at(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    ledger: &mut crate::ability::creature_bite::CreatureBiteLedger,
    (creature, actor, session): (ExactActorRef, ExactActorRef, GameSessionId),
    sequence: u64,
    micros: u64,
) -> Result<crate::ability::creature_bite::AppliedBite, crate::ability::creature_bite::BiteRejection>
{
    use crate::ability::AiAbilityAdapter;
    use crate::ability::creature_bite::{
        CreatureBiteDefinition, ReentryProtection, commit_ai_bite,
    };
    use crate::foundation::owner_timer::SemanticTimeMicros as OwnerTime;
    commit_ai_bite(
        ledger,
        runtime,
        states,
        AiAbilityAdapter::bite(creature, sequence, actor, session),
        CreatureBiteDefinition::new(2_000_000, 8).expect("definition"),
        RevisionSet::new(
            "ruleset:ai-v1",
            "content:1",
            "world:ai-v1",
            "formula:bite-v1",
            "simulation:v1",
        )
        .expect("revisions"),
        ReentryProtection {
            protected_until: None,
        },
        OwnerTime::from_micros(micros),
    )
}

/// DEATH-2 §4.1/§4.2 on the real vitals owner: a creature bite to 0 is lethal, the death
/// occurrence is minted and the death cell recorded in the same vitals revision, and the dead
/// player takes no further bite, cast, credit or Serene change.
#[test]
fn a_lethal_creature_bite_records_the_death_and_the_dead_player_takes_nothing() {
    use crate::ability::creature_bite::{BiteRejection, CreatureBiteLedger, CreatureDamage};
    use crate::foundation::MovementLocalPosition;

    let (mut runtime, mut states, actor, session, creature) = bitten_player(0x31, 5);
    let mut ledger = CreatureBiteLedger::default();
    let actors = (creature, actor, session);
    let first = bite_at(&mut runtime, &mut states, &mut ledger, actors, 0, 0).expect("bite");
    assert_eq!(
        first.damage,
        CreatureDamage {
            applied: 5,
            health_after: 0
        }
    );
    assert_eq!(first.vitals_revision, 2);
    let death = states
        .player_death(&runtime, actor, session)
        .expect("recorded death");
    assert_eq!(first.death, Some(*death.occurrence.as_bytes()));
    assert_eq!(
        death.cell,
        MovementLocalPosition {
            x: 10,
            y: 10,
            floor: 7
        }
    );
    assert!(states.is_dead(actor));
    // The lethal think occurrence replays its first result; a later bite has no target.
    assert_eq!(
        bite_at(&mut runtime, &mut states, &mut ledger, actors, 0, 0),
        Ok(first)
    );
    assert_eq!(
        bite_at(&mut runtime, &mut states, &mut ledger, actors, 1, 2_000_000),
        Err(BiteRejection::StaleTarget)
    );
    assert_eq!(
        cast_at(&runtime, &mut states, actor, session, 7, 3000),
        SpellCastOutcome::rejected()
    );
    assert_eq!(states.apply_health_gain(&runtime, actor, session, 50), None);
    assert_eq!(states.apply_mana_gain(&runtime, actor, session, 5), None);
    assert_eq!(states.tick(&runtime, actor, session, now(4000)), None);
    let (revision, vitals) = observe_vitals(&runtime, &states, actor, session).expect("vitals");
    assert_eq!((revision, vitals.health), (2, 0));
}

/// DEATH-2 §4.5 (D63): the respawn refills health and mana under exactly one new vitals
/// revision and ends the death; only the recorded occurrence of the exact session respawns.
#[test]
fn the_respawn_refills_health_and_mana_in_one_revision_and_ends_the_death() {
    use crate::ability::creature_bite::CreatureBiteLedger;

    let (mut runtime, mut states, actor, session, creature) = bitten_player(0x32, 8);
    let mut ledger = CreatureBiteLedger::default();
    // A heal spends mana, so the respawn has both pools to refill.
    let healed = cast_at(&runtime, &mut states, actor, session, 1, 0);
    assert_eq!(healed.disposition, SpellCastDisposition::Cast);
    let spent = healed.vitals.expect("vitals").1;
    assert!(spent.mana < FACTS.max_mana);
    wound(&mut states, actor, session, 8);
    bite_at(
        &mut runtime,
        &mut states,
        &mut ledger,
        (creature, actor, session),
        0,
        0,
    )
    .expect("lethal bite");
    let death = states
        .player_death(&runtime, actor, session)
        .expect("death");
    let other = PlayerDeathOccurrence::from_bytes(uuid_v7(0x77)).expect("occurrence");
    assert_eq!(
        states.respawn(&runtime, actor, session, other, now(0)),
        None
    );
    let (_, other_actor, other_session) = runtime_with_player(0x33);
    assert_eq!(
        states.respawn(
            &runtime,
            other_actor,
            other_session,
            death.occurrence,
            now(0)
        ),
        None
    );
    let (revision, vitals) = states
        .respawn(&runtime, actor, session, death.occurrence, now(0))
        .expect("respawned");
    assert_eq!(revision, 4);
    assert_eq!(
        (vitals.health, vitals.mana),
        (FACTS.max_health, FACTS.max_mana)
    );
    assert!(!states.is_dead(actor));
    assert_eq!(states.player_death(&runtime, actor, session), None);
    // The respawn happens once; the living actor takes commands again.
    assert_eq!(
        states.respawn(&runtime, actor, session, death.occurrence, now(0)),
        None
    );
    assert_eq!(
        cast_at(&runtime, &mut states, actor, session, 8, 10_000).disposition,
        SpellCastDisposition::Cast
    );
}

/// DEATH-2 §4.5 with SPELL-D8 §8.2: a monk respawns with Harmony and forced Serene reset, as
/// the death commit writes them, and its Serene evaluation initialized again, so a monk that
/// lost control while dead is playable after the respawn.
#[test]
fn a_monk_respawns_with_harmony_and_forced_serene_reset_and_initialized() {
    use crate::ability::creature_bite::CreatureBiteLedger;

    let monk = CharacterCastFacts {
        vocation: Vocation::Monk,
        ..FACTS
    };
    let (mut runtime, mut states, actor, session, creature) =
        bitten_actor(0x35, 8, monk, (3, 4_000_000));
    bite_at(
        &mut runtime,
        &mut states,
        &mut CreatureBiteLedger::default(),
        (creature, actor, session),
        0,
        0,
    )
    .expect("lethal bite");
    let death = states
        .player_death(&runtime, actor, session)
        .expect("death");
    assert_eq!(
        states.monk_save_values(&runtime, actor, session, now(1000)),
        Some((0, 0))
    );
    let (_, dead_vitals) = observe_vitals(&runtime, &states, actor, session).expect("dead vitals");
    assert_eq!(
        (dead_vitals.health, dead_vitals.harmony, dead_vitals.serene),
        (0, 0, false)
    );
    // Control loss detaches the dead monk; only the respawn initializes it again.
    states.detach(&runtime, actor, session);
    let (_, vitals) = states
        .respawn(&runtime, actor, session, death.occurrence, now(2000))
        .expect("respawned");
    assert_eq!((vitals.harmony, vitals.serene), (0, true));
    assert_eq!(
        states.monk_save_values(&runtime, actor, session, now(2000)),
        Some((0, 0))
    );
    assert_eq!(
        cast_at(&runtime, &mut states, actor, session, 1, 2000).disposition,
        SpellCastDisposition::Cast
    );
}

/// A lethal hit whose death occurrence cannot be minted is refused with nothing written.
#[test]
fn a_lethal_hit_without_an_occurrence_writes_nothing() {
    use crate::ability::creature_bite::{BiteRejection, CreatureBiteLedger};

    let (mut runtime, mut states, actor, session, creature) = bitten_actor(
        0x34,
        8,
        CharacterCastFacts {
            vocation: Vocation::Monk,
            ..FACTS
        },
        (3, 4_000_000),
    );
    states.mint_death = || None;
    let mut ledger = CreatureBiteLedger::default();
    assert_eq!(
        bite_at(
            &mut runtime,
            &mut states,
            &mut ledger,
            (creature, actor, session),
            0,
            0
        ),
        Err(BiteRejection::StaleTarget)
    );
    assert!(!states.is_dead(actor));
    let (revision, vitals) = observe_vitals(&runtime, &states, actor, session).expect("vitals");
    assert_eq!((revision, vitals.health), (1, 8));
    assert_eq!((vitals.harmony, vitals.serene), (3, true));
    assert_eq!(
        states.monk_save_values(&runtime, actor, session, now(1000)),
        Some((3, 3_000_000))
    );
}
