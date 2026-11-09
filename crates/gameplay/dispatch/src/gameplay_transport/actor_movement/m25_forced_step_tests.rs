#[test]
fn m25_forced_grace_step_uses_the_normal_teleport_and_preserves_control_loss() {
    use crate::foundation::{CharacterId, CharacterLease, ControlLossMark, MovementLocalPosition};
    let (mut runtime, mut states, cells, actor, session) =
        bundle_owner_of(crate::map::boot::tests::teleport_bundle);
    let pusher_session = GameSessionId::decode(&id(7)).unwrap();
    let reservation = runtime.reserve_fresh_session(pusher_session).unwrap();
    let pusher = runtime.commit_fresh_session(reservation).unwrap();
    runtime.initialize_first_entry_position(pusher).unwrap();
    for (actor, session, seed) in [(actor, session, 8), (pusher, pusher_session, 9)] {
        let character = CharacterId::decode(&id(seed)).unwrap();
        runtime
            .bind_attacker_lease(actor, session, CharacterLease::new(character, 1).unwrap())
            .unwrap();
    }
    let loss = ControlLossMark {
        epoch: 1,
        grace_deadline: 100,
    };
    runtime.record_control_loss(actor, session, loss).unwrap();
    let target_at = runtime.read_actor_position(actor).unwrap();
    let pusher_at = runtime.read_actor_position(pusher).unwrap();
    let none = std::collections::BTreeSet::new();
    assert!(matches!(
        step_in_channel(
            &mut runtime,
            &mut states,
            &cells,
            actor,
            session,
            1,
            CardinalStep::South,
            &none,
            None
        ),
        Err(MovementError::NotQualified)
    ));
    let guard = runtime
        .prepare_player_push(
            (pusher, pusher_session, pusher_at),
            (actor, session, target_at),
            MovementLocalPosition {
                x: 0,
                y: 1,
                floor: 8,
            },
        )
        .unwrap();
    assert!(
        step_in_channel(
            &mut runtime,
            &mut states,
            &cells,
            actor,
            session,
            1,
            CardinalStep::South,
            &none,
            None
        )
        .is_ok()
    );
    assert_eq!(at(&mut runtime, actor), (5, 1, 8));
    assert_eq!(
        runtime
            .player_control_facts(actor, session)
            .unwrap()
            .control_loss,
        Some(loss)
    );
    assert!(
        !runtime.forced_player_step_current(actor, session),
        "the committed position spends the proof"
    );
    drop(guard);
    assert!(!runtime.player_step_allowed(actor, session, (0, 1)));
}
