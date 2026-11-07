//! FLOOR-EXHAUST-1: genuine bundle steps, live session and the existing cooldown owner.
use super::*;
use oteryn_gameplay_spell_rules::cast::{STAIRHOP_COOLDOWN_KEY, STAIRHOP_MICROS};

fn initialize(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
) {
    states
        .initialize(
            runtime,
            actor,
            session,
            CharacterCastFacts {
                level: 100,
                magic_level: 50,
                ..crate::gameplay_transport::actor_spell::tests::FACTS
            },
            (0, 0),
            SemanticTimeMicros::from_micros(0),
        )
        .unwrap();
}
fn step(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    now: u64,
    direction: CardinalStep,
) -> StepInChannel {
    step_in_channel(
        runtime,
        states,
        cells,
        actor,
        session,
        now,
        direction,
        &std::collections::BTreeSet::new(),
        None,
    )
}
fn until(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
) -> Option<u64> {
    states
        .get(runtime, actor, session)
        .unwrap()
        .cooldowns
        .group_ready_at(STAIRHOP_COOLDOWN_KEY)
        .map(|at| at.get())
}

#[test]
fn floor_exhaust_stairs_holes_refresh_but_plain_steps_do_not_and_movement_stays_allowed() {
    let (mut runtime, mut states, cells, actor, session) = bundle_owner();
    initialize(&runtime, &mut states, actor, session);
    assert_eq!(
        until(&runtime, &states, actor, session),
        None,
        "login adds no movement exhaust"
    );
    step(
        &mut runtime,
        &mut states,
        &cells,
        actor,
        session,
        100,
        CardinalStep::East,
    )
    .unwrap();
    assert_eq!(at(&mut runtime, actor), (2, 0, 7));
    assert_eq!(
        until(&runtime, &states, actor, session),
        Some(100 + STAIRHOP_MICROS)
    );
    // A second floor change inside the window is legal and rearms it.
    step(
        &mut runtime,
        &mut states,
        &cells,
        actor,
        session,
        200,
        CardinalStep::East,
    )
    .unwrap();
    assert_eq!(at(&mut runtime, actor), (3, 0, 8));
    assert_eq!(
        until(&runtime, &states, actor, session),
        Some(200 + STAIRHOP_MICROS)
    );
    step(
        &mut runtime,
        &mut states,
        &cells,
        actor,
        session,
        300,
        CardinalStep::West,
    )
    .unwrap();
    assert_eq!(
        until(&runtime, &states, actor, session),
        Some(200 + STAIRHOP_MICROS)
    );
    // Non-aggressive healing stays usable during the hop.
    let book = crate::spell::cast::v1_spell_book().unwrap();
    assert_eq!(
        crate::gameplay_transport::actor_spell::cast_in_channel(
            &runtime,
            &mut states,
            &book,
            actor,
            session,
            1,
            &crate::gameplay_transport::actor_spell::tests::exura(),
            SemanticTimeMicros::from_micros(400)
        )
        .disposition,
        oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::Cast
    );
}

#[test]
fn floor_exhaust_same_floor_teleport_starts_window_but_failed_and_plain_steps_do_not() {
    let (mut runtime, mut states, cells, actor, session) =
        bundle_owner_of(crate::map::boot::tests::teleport_bundle);
    initialize(&runtime, &mut states, actor, session);
    assert!(
        step(
            &mut runtime,
            &mut states,
            &cells,
            actor,
            session,
            10,
            CardinalStep::North
        )
        .is_err()
    );
    assert_eq!(until(&runtime, &states, actor, session), None);
    step(
        &mut runtime,
        &mut states,
        &cells,
        actor,
        session,
        20,
        CardinalStep::South,
    )
    .unwrap();
    assert_eq!(at(&mut runtime, actor), (5, 1, 8));
    assert_eq!(
        until(&runtime, &states, actor, session),
        Some(20 + STAIRHOP_MICROS)
    );

    let (mut runtime, mut states, room, actor, session) = owner();
    initialize(&runtime, &mut states, actor, session);
    let cells = room.movement_cells();
    step(
        &mut runtime,
        &mut states,
        cells,
        actor,
        session,
        30,
        CardinalStep::East,
    )
    .unwrap();
    assert_eq!(until(&runtime, &states, actor, session), None);
}

#[test]
fn floor_exhaust_overflow_refuses_without_moving_or_changing_cooldowns() {
    let (mut runtime, mut states, cells, actor, session) = bundle_owner();
    initialize(&runtime, &mut states, actor, session);
    let before = states.get(&runtime, actor, session).unwrap().clone();
    assert!(
        step(
            &mut runtime,
            &mut states,
            &cells,
            actor,
            session,
            u64::MAX,
            CardinalStep::East
        )
        .is_err()
    );
    assert_eq!(at(&mut runtime, actor), (0, 0, 8));
    assert_eq!(states.get(&runtime, actor, session), Some(&before));
}
