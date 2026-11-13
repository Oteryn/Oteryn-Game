struct M25PushActors {
    runtime: ChannelRuntimeV1,
    pusher: (ExactActorRef, GameSessionId, MovementPositionSnapshot),
    target: (ExactActorRef, GameSessionId, MovementPositionSnapshot),
}
fn m25_push_actors() -> Result<M25PushActors, CarrierError> {
    let mut runtime = runtime(4);
    let mut actors = Vec::new();
    for seed in [70, 71] {
        let session = session(seed);
        let reservation = runtime.reserve_fresh_session(session)?;
        let actor = runtime.commit_fresh_session(reservation)?;
        runtime.initialize_first_entry_position(actor)?;
        let character = CharacterId::decode(&uuid_v7(seed))
            .map_err(|_| CarrierError::PlayerReservationMismatch)?;
        let lease = CharacterLease::new(character, 1)
            .map_err(|_| CarrierError::PlayerReservationMismatch)?;
        runtime.bind_attacker_lease(actor, session, lease)?;
        actors.push((actor, session, runtime.read_actor_position(actor)?));
    }
    Ok(M25PushActors { runtime, pusher: actors[0], target: actors[1] })
}

#[test]
fn m25_push_guard_never_restores_grace_input_and_drop_revokes_movement() -> Result<(), CarrierError> {
    let mut fixture = m25_push_actors()?;
    let (actor, session, at) = fixture.target;
    let mark = ControlLossMark { epoch: 7, grace_deadline: 100 };
    fixture.runtime.record_control_loss(actor, session, mark)?;
    assert!(!fixture.runtime.player_step_allowed(actor, session, (0, 1)));
    let from = at.position();
    let destination = MovementLocalPosition { y: from.y + 1, ..from };
    let guard = fixture.runtime.prepare_player_push(fixture.pusher, fixture.target, destination)?;
    assert!(fixture.runtime.player_step_allowed(actor, session, (0, 1)));
    assert!(!fixture.runtime.player_step_allowed(actor, session, (1, 0)));
    assert_eq!(fixture.runtime.player_control_facts(actor, session)?.control_loss, Some(mark));
    assert_eq!(fixture.runtime.read_actor_position(actor)?, at);
    drop(guard);
    assert!(!fixture.runtime.player_step_allowed(actor, session, (0, 1)));
    assert_eq!(fixture.runtime.player_control_facts(actor, session)?.control_loss, Some(mark));
    Ok(())
}

#[test]
fn m25_push_guard_rechecks_the_pusher_and_explicit_target_write_fence() -> Result<(), CarrierError> {
    let mut fixture = m25_push_actors()?;
    let (actor, session, at) = fixture.target;
    let mark = ControlLossMark { epoch: 7, grace_deadline: 100 };
    fixture.runtime.record_control_loss(actor, session, mark)?;
    let from = at.position();
    let destination = MovementLocalPosition { y: from.y + 1, ..from };
    let guard = fixture.runtime.prepare_player_push(fixture.pusher, fixture.target, destination)?;
    fixture.runtime.fence_player_writes(actor, session, WriteFenceToken::ControlLoss(7))?;
    assert!(!fixture.runtime.player_step_allowed(actor, session, (0, 1)));
    drop(guard);
    assert!(fixture.runtime.prepare_player_push(fixture.pusher, fixture.target, destination).is_err());
    Ok(())
}

#[test]
fn m25_push_guard_rejects_a_lost_pusher_and_non_cardinal_destination() -> Result<(), CarrierError> {
    let mut fixture = m25_push_actors()?;
    let from = fixture.target.2.position();
    let destination = MovementLocalPosition { y: from.y + 1, ..from };
    assert!(fixture.runtime.prepare_player_push(fixture.pusher, fixture.target,
        MovementLocalPosition { y: from.y + 2, ..from }).is_err());
    let guard = fixture.runtime.prepare_player_push(fixture.pusher, fixture.target, destination)?;
    fixture.runtime.record_control_loss(fixture.pusher.0, fixture.pusher.1,
        ControlLossMark { epoch: 1, grace_deadline: 100 })?;
    assert!(!fixture.runtime.player_step_allowed(fixture.target.0, fixture.target.1, (0, 1)));
    drop(guard);
    assert!(fixture.runtime.prepare_player_push(fixture.pusher, fixture.target, destination).is_err());
    Ok(())
}
