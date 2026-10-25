#[test]
fn carrier_loss_does_not_release_same_generation_claim() {
    let mut continuity = continuity(50, 9);
    let carrier =
        ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1).expect("first bootstrap");
    drop(carrier);
    assert_eq!(
        ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1),
        Err(CarrierError::NamespaceAlreadyClaimed)
    );
}

#[test]
fn claimed_namespace_rejects_before_the_allocation_sentinel() {
    let mut continuity = continuity(55, 3);
    let carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1)
        .expect("first bootstrap claims namespace");
    drop(carrier);

    // This huge capacity is arithmetically valid and would hit the test
    // allocation-failure sentinel if authority preflight did not win.
    assert_eq!(
        ChannelActorCarrier::bootstrap_pre_production(
            &mut continuity,
            TEST_ALLOCATION_FAILURE_CAPACITY,
        ),
        Err(CarrierError::NamespaceAlreadyClaimed)
    );
}

#[test]
fn allocation_failure_does_not_consume_namespace_claim() {
    let mut continuity = continuity(56, 3);
    assert_eq!(
        ChannelActorCarrier::bootstrap_pre_production(
            &mut continuity,
            TEST_ALLOCATION_FAILURE_CAPACITY,
        ),
        Err(CarrierError::AllocationFailed)
    );
    assert!(!continuity.current_generation_claimed);

    let mut carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1)
        .expect("retry after allocation failure");
    assert!(continuity.current_generation_claimed);
    assert!(carrier.admit(&continuity, ActorState(8)).is_ok());
}

#[test]
fn advancing_live_continuity_immediately_fences_all_old_operations() {
    let mut continuity = continuity(60, 4);
    let mut carrier =
        ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1).expect("bootstrap");
    let actor_ref = carrier.admit(&continuity, ActorState(7)).expect("admit");

    continuity.advance(grant(60, 5)).expect("strictly newer");
    let slots_before = carrier.slots.clone();

    assert_eq!(
        carrier.admit(&continuity, ActorState(9)),
        Err(CarrierError::WrongScope)
    );
    assert_eq!(
        carrier.lookup(&continuity, actor_ref),
        Err(CarrierError::WrongScope)
    );
    assert_eq!(
        carrier.remove(&continuity, actor_ref),
        Err(CarrierError::WrongScope)
    );
    assert_eq!(carrier.slots, slots_before);
}

#[test]
fn stale_equal_and_cross_scope_advances_leave_guard_unchanged() {
    let mut continuity = continuity(70, 8);
    let before = (
        continuity.world_id,
        continuity.channel_id,
        continuity.current_generation,
        continuity.current_generation_claimed,
    );

    assert_eq!(
        continuity.advance(grant(70, 8)),
        Err(CarrierError::ContinuityGenerationNotNewer)
    );
    assert_eq!(
        continuity.advance(grant(70, 7)),
        Err(CarrierError::ContinuityGenerationNotNewer)
    );
    assert_eq!(
        continuity.advance(grant(80, 9)),
        Err(CarrierError::WrongScope)
    );
    assert_eq!(
        (
            continuity.world_id,
            continuity.channel_id,
            continuity.current_generation,
            continuity.current_generation_claimed,
        ),
        before
    );
}

#[test]
fn strictly_newer_advance_permits_one_fresh_namespace_and_keeps_old_fenced() {
    let mut continuity = continuity(90, 2);
    let mut old_carrier =
        ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1).expect("old bootstrap");
    let old_ref = old_carrier
        .admit(&continuity, ActorState(1))
        .expect("old admit");

    continuity.advance(grant(90, 3)).expect("advance");
    let mut new_carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1)
        .expect("fresh generation bootstrap");

    assert_eq!(new_carrier.scope_generation.get(), 3);
    let new_ref = new_carrier
        .admit(&continuity, ActorState(2))
        .expect("fresh carrier admits under live continuity");
    assert_eq!(new_carrier.lookup(&continuity, new_ref), Ok(&ActorState(2)));
    assert_eq!(
        old_carrier.lookup(&continuity, old_ref),
        Err(CarrierError::WrongScope)
    );
    assert_eq!(
        ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1),
        Err(CarrierError::NamespaceAlreadyClaimed)
    );
}

/// VIS-3: `visible_entities` reads committed positioned players and live creatures under the
/// pinned context, and nothing else; it changes nothing.
#[test]
fn visible_entities_are_the_pinned_players_and_live_creatures() {
    let mut runtime = runtime(6);
    let at = |x: i32| MovementLocalPosition { x, y: 5, floor: 7 };
    let player = |runtime: &mut ChannelRuntimeV1, raw: u64, x: i32| {
        let reservation = runtime
            .reserve_fresh_session(session(raw))
            .expect("reserve");
        let actor = runtime.commit_fresh_session(reservation).expect("commit");
        runtime
            .initialize_movement_test_position(actor, at(x))
            .expect("position");
        actor
    };
    let shown = player(&mut runtime, 30, 1);
    // Positioned under another context: never shown.
    let other_context = player(&mut runtime, 31, 2);
    // Reserved but not committed: never shown.
    runtime.reserve_fresh_session(session(32)).expect("reserve");
    let live = runtime.admit_test_creature(at(3)).expect("creature");
    // Dead: never shown.
    runtime.admit_test_creature(at(4)).expect("creature");
    let pinned = runtime.pinned_position_context();
    for slot in runtime.carrier.slots.iter_mut() {
        match slot {
            Slot::Occupied {
                position: Some(version),
                ..
            }
            | Slot::CreatureOccupied {
                position: Some(version),
                ..
            } if version.position.x != 2 => version.context = pinned,
            _ => {}
        }
        if let Slot::CreatureOccupied {
            health,
            position: Some(version),
            ..
        } = slot
            && version.position.x == 4
        {
            *health = 0;
        }
    }
    let before = runtime.carrier.slots.clone();

    let visible = runtime.visible_entities();
    assert_eq!(
        visible
            .players
            .iter()
            .map(|entry| entry.actor)
            .collect::<Vec<_>>(),
        [shown]
    );
    assert_ne!(shown, other_context);
    assert_eq!(visible.players[0].position, at(1));
    assert_eq!(
        visible.players[0].generation,
        shown.0.actor_local_generation.0
    );
    assert_eq!(
        visible
            .creatures
            .iter()
            .map(|entry| entry.actor)
            .collect::<Vec<_>>(),
        [live]
    );
    assert_eq!(visible.creatures[0].position, at(3));
    // Read-only.
    assert_eq!(runtime.carrier.slots, before);
    assert_eq!(runtime.visible_entities(), visible);
}
