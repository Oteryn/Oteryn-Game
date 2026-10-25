#[test]
fn multiple_explicit_fixture_bounds_enforce_m_and_m_plus_one() {
    prove_boundary(1);
    prove_boundary(2);
    prove_boundary(4);
}

#[test]
fn construction_rejects_zero_and_checked_overflow() {
    assert_eq!(
        ChannelActorCarrier::bootstrap_pre_production(&mut continuity(10, 1), 0),
        Err(CarrierError::InvalidCapacity)
    );
    assert_eq!(
        ChannelActorCarrier::bootstrap_pre_production(&mut continuity(10, 1), usize::MAX),
        Err(CarrierError::CapacityArithmeticOverflow)
    );
}

#[test]
fn exact_lookup_rejects_cross_scope_and_invalid_identity() {
    let (continuity, mut carrier) = carrier(1);
    let actor_ref = carrier.admit(&continuity, ActorState(7)).expect("admit");
    assert_eq!(carrier.lookup(&continuity, actor_ref), Ok(&ActorState(7)));

    let mut wrong_world = actor_ref;
    wrong_world.world_id = grant(20, 1).world_id;
    assert_eq!(
        carrier.lookup(&continuity, wrong_world),
        Err(CarrierError::WrongScope)
    );
    let mut wrong_channel = actor_ref;
    wrong_channel.channel_id = grant(20, 1).channel_id;
    assert_eq!(
        carrier.lookup(&continuity, wrong_channel),
        Err(CarrierError::WrongScope)
    );
    let mut wrong_generation = actor_ref;
    wrong_generation.set_scope_generation(ScopeOwnershipGeneration::new(2).expect("valid"));
    assert_eq!(
        carrier.lookup(&continuity, wrong_generation),
        Err(CarrierError::WrongScope)
    );
    let mut missing = actor_ref;
    missing.actor_local_id = ActorLocalId(2);
    assert_eq!(
        carrier.lookup(&continuity, missing),
        Err(CarrierError::InvalidActorIdentity)
    );
}

#[test]
fn removal_retains_generation_and_reuse_stales_old_reference() {
    let (continuity, mut carrier) = carrier(1);
    let first = carrier
        .admit(&continuity, ActorState(1))
        .expect("first admit");
    assert_eq!(carrier.remove(&continuity, first), Ok(ActorState(1)));
    assert_eq!(
        carrier.lookup(&continuity, first),
        Err(CarrierError::StaleActorGeneration)
    );
    let second = carrier.admit(&continuity, ActorState(2)).expect("reuse");
    assert_eq!(second.actor_local_id, first.actor_local_id);
    assert_eq!(second.actor_local_generation.0, 2);
    assert_eq!(
        carrier.lookup(&continuity, first),
        Err(CarrierError::StaleActorGeneration)
    );
}

#[test]
fn multiple_removed_holes_reuse_in_lifo_removal_order() {
    let (continuity, mut carrier) = carrier(4);
    let refs = (0..4)
        .map(|value| {
            carrier
                .admit(&continuity, ActorState(value))
                .expect("initial admit")
        })
        .collect::<Vec<_>>();

    assert_eq!(carrier.remove(&continuity, refs[1]), Ok(ActorState(1)));
    assert_eq!(carrier.remove(&continuity, refs[3]), Ok(ActorState(3)));

    let first_reuse = carrier
        .admit(&continuity, ActorState(10))
        .expect("first recycled admit");
    let second_reuse = carrier
        .admit(&continuity, ActorState(11))
        .expect("second recycled admit");

    assert_eq!(first_reuse.actor_local_id, refs[3].actor_local_id);
    assert_eq!(second_reuse.actor_local_id, refs[1].actor_local_id);
    assert_eq!(first_reuse.actor_local_generation.0, 2);
    assert_eq!(second_reuse.actor_local_generation.0, 2);
}

#[test]
fn representable_post_selection_failure_rolls_back_everything() {
    let (continuity, mut carrier) = carrier(2);
    let first = carrier
        .admit(&continuity, ActorState(1))
        .expect("unrelated actor");
    let before = carrier.slots.clone();
    let free_head_before = carrier.free_head;
    assert_eq!(
        carrier.admit_inner(&continuity, ActorState(2), None, true, None, true),
        Err(CarrierError::InjectedAdmissionFailure)
    );
    assert_eq!(carrier.slots, before);
    assert_eq!(carrier.free_head, free_head_before);
    assert_eq!(carrier.lookup(&continuity, first), Ok(&ActorState(1)));
}

#[test]
fn exhausted_reuse_marks_only_selected_slot_and_never_reselects_it() {
    let (continuity, mut carrier) = carrier(2);
    assert!(matches!(carrier.slots[0], Slot::VacantReusable { .. }));
    if let Slot::VacantReusable { generation, .. } = &mut carrier.slots[0] {
        *generation = u64::MAX;
    }
    let unrelated = carrier.slots[1].clone();
    assert_eq!(
        carrier.admit(&continuity, ActorState(1)),
        Err(CarrierError::ActorGenerationExhausted)
    );
    assert_eq!(
        carrier.slots[0],
        Slot::Exhausted {
            generation: u64::MAX
        }
    );
    assert_eq!(carrier.slots[1], unrelated);
    let admitted = carrier
        .admit(&continuity, ActorState(2))
        .expect("skip exhausted slot");
    assert_eq!(admitted.actor_local_id, ActorLocalId(2));
}

#[test]
fn ai2_multiple_creatures_coexist_bounded_only_by_general_capacity() {
    // AI-2 (GAME-AI-01 §4.1: the carrier's one-creature limit is lifted to the D57
    // envelope): a second, distinct creature now admits successfully while the first is
    // still live, bounded only by the carrier's own general slot capacity.
    let (continuity, mut carrier) = carrier(2);

    let first = carrier
        .admit_creature(&continuity, ActorState(1), "target:one", 20)
        .expect("first creature");
    let second = carrier
        .admit_creature(&continuity, ActorState(2), "target:two", 20)
        .expect("second creature coexists with the first");
    assert_ne!(first, second);
    assert_eq!(
        carrier.admit_creature(&continuity, ActorState(3), "target:three", 20),
        Err(CarrierError::CapacityExceeded),
        "capacity 2 is exhausted by two live creatures"
    );

    assert_eq!(carrier.remove(&continuity, first), Ok(ActorState(1)));
    carrier
        .admit_creature(&continuity, ActorState(4), "target:four", 20)
        .expect("the freed slot is reusable while the other creature stays live");
    assert!(
        carrier
            .current_owner_exact_lookup(&continuity)
            .contains(ExactActorRef(second))
    );
}
