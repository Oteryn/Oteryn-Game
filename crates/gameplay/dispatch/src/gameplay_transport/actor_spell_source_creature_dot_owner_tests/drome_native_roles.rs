//! Actual canonical Player condition ownership, with an explicit mechanics-only role grant.
use super::*;
use crate::foundation::owner_timer::SemanticTimeMicros;
use oteryn_gameplay_entities::runtime_actor_carrier::tibiadrome_modifiers::{
    DromeCreatureRole, DromeModifier,
};

fn drome_fixture() -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    ExactActorRef,
    ExactActorRef,
    GameSessionId,
) {
    let (mut runtime, states, player, creature, session) = fixture();
    runtime.set_tibiadrome(player, true).unwrap();
    runtime.scale_tibiadrome_opponent(creature, 0).unwrap();
    let selected = [
        DromeModifier::Somersault,
        DromeModifier::ThatEscalatedQuickly,
    ];
    for actor in [player, creature] {
        runtime
            .bind_tibiadrome_modifiers(actor, [5; 16], selected)
            .unwrap();
    }
    runtime
        .test_bind_drome_creature_role(creature, DromeCreatureRole::Murmillion, 0)
        .unwrap();
    assert_eq!(
        runtime
            .companion_snapshot(creature)
            .unwrap()
            .state
            .policy
            .definition_key,
        "creature:source-healing-target",
        "the test must retain its existing immutable source profile"
    );
    (runtime, states, player, creature, session)
}

#[test]
fn srx3_drome_native_murmillion_commits_five_second_standard_player_paralysis() {
    let (mut runtime, mut states, player, creature, session) = drome_fixture();
    assert_eq!(
        states.apply_drome_murmillion_paralyze(
            &mut runtime,
            player,
            session,
            creature,
            SemanticTimeMicros::from_micros(9_999_999)
        ),
        Some(false)
    );
    assert_eq!(
        states
            .get(&runtime, player, session)
            .unwrap()
            .owned_conditions()
            .speed_delta(),
        0
    );
    let base = states
        .get(&runtime, player, session)
        .unwrap()
        .owned_base_speed();
    let _ = states.apply_drome_murmillion_paralyze(
        &mut runtime,
        player,
        session,
        creature,
        SemanticTimeMicros::from_micros(10_000_000),
    );
    let conditions = states
        .get(&runtime, player, session)
        .unwrap()
        .owned_conditions();
    assert_eq!(
        conditions.speed_delta(),
        i64::from(40u32.min(base)) - i64::from(base)
    );
    assert!(
        conditions
            .instances()
            .iter()
            .any(
                |instance| instance.speed_delta() == conditions.speed_delta()
                    && instance.remaining_duration_ms(10_000_000) == Some(5000)
            )
    );
    assert_eq!(
        states.apply_drome_murmillion_paralyze(
            &mut runtime,
            player,
            session,
            creature,
            SemanticTimeMicros::from_micros(10_000_000)
        ),
        Some(false)
    );
    assert!(!runtime.drome_role_paralyze_due(creature, 19_999_999));
    assert!(runtime.drome_role_paralyze_due(creature, 20_000_000));
}

#[test]
fn srx3_drome_native_murmillion_cannot_paralyze_a_foreign_run_or_departed_player() {
    let (mut runtime, mut states, player, creature, session) = drome_fixture();
    runtime
        .bind_tibiadrome_modifiers(
            player,
            [6; 16],
            [
                DromeModifier::Somersault,
                DromeModifier::ThatEscalatedQuickly,
            ],
        )
        .unwrap();
    assert_eq!(
        states.apply_drome_murmillion_paralyze(
            &mut runtime,
            player,
            session,
            creature,
            SemanticTimeMicros::from_micros(10_000_000)
        ),
        None
    );
    assert_eq!(
        states
            .get(&runtime, player, session)
            .unwrap()
            .owned_conditions()
            .speed_delta(),
        0
    );
    assert!(runtime.drome_role_paralyze_due(creature, 10_000_000));
    runtime.set_tibiadrome(player, false).unwrap();
    assert_eq!(
        states.apply_drome_murmillion_paralyze(
            &mut runtime,
            player,
            session,
            creature,
            SemanticTimeMicros::from_micros(10_000_000)
        ),
        None
    );
}
