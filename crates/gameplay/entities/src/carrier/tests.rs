use super::spawn_realization::{SpawnCellState, spawn_points_within_ceilings};
use super::*;

fn uuid_v7(raw: u64) -> [u8; 16] {
    let mut value = [0; 16];
    value[8..].copy_from_slice(&raw.to_be_bytes());
    value[6] = 0x70;
    value[8] = (value[8] & 0x3f) | 0x80;
    value
}

fn grant(seed: u64, generation: u64) -> PreProductionContinuityGrant {
    PreProductionContinuityGrant {
        world_id: WorldId::decode(&uuid_v7(seed)).expect("valid WorldId fixture"),
        channel_id: ChannelId::decode(&uuid_v7(seed + 1)).expect("valid ChannelId fixture"),
        scope_generation: ScopeOwnershipGeneration::new(generation)
            .expect("non-zero generation fixture"),
    }
}

fn continuity(seed: u64, generation: u64) -> NamespaceContinuityGuard {
    NamespaceContinuityGuard::from_pre_production_grant(grant(seed, generation))
}

fn carrier(capacity: usize) -> (NamespaceContinuityGuard, ChannelActorCarrier) {
    let mut continuity = continuity(10, 1);
    let carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, capacity)
        .expect("valid explicit test capacity");
    (continuity, carrier)
}

fn prove_boundary(capacity: usize) {
    let (continuity, mut carrier) = carrier(capacity);
    let mut refs = Vec::new();
    for value in 0..capacity {
        refs.push(
            carrier
                .admit(&continuity, ActorState(value as u64))
                .expect("M succeeds"),
        );
    }
    let before = carrier.slots.clone();
    assert_eq!(
        carrier.admit(&continuity, ActorState(99)),
        Err(CarrierError::CapacityExceeded)
    );
    assert_eq!(carrier.slots, before);
    for actor_ref in refs {
        assert!(carrier.lookup(&continuity, actor_ref).is_ok());
    }
}

pub(super) fn session(raw: u64) -> GameSessionId {
    GameSessionId::decode(&uuid_v7(raw)).expect("valid GameSessionId fixture")
}

fn node(raw: u64) -> NodeId {
    NodeId::decode(&uuid_v7(raw)).expect("valid NodeId fixture")
}

pub(super) fn runtime(capacity: usize) -> ChannelRuntimeV1 {
    ChannelRuntimeV1::from_committed_assignment(
        WorldId::decode(&uuid_v7(20)).expect("world"),
        ChannelId::decode(&uuid_v7(21)).expect("channel"),
        node(22),
        7,
        3,
        11,
        "runtime-scope-assignment:11",
        capacity,
        ChannelContentPin::test(WorldId::decode(&uuid_v7(20)).expect("world")),
    )
    .expect("runtime")
}

#[test]
fn composed_runtime_binds_assignment_and_reserves_before_commit() {
    let mut runtime = runtime(1);
    let binding = runtime.binding();
    assert!(binding.matches_committed_assignment(
        binding.world_id(),
        binding.channel_id(),
        node(22),
        7,
        3,
        11,
        "runtime-scope-assignment:11",
    ));
    assert!(!binding.matches_committed_assignment(
        binding.world_id(),
        binding.channel_id(),
        node(22),
        7,
        4,
        12,
        "runtime-scope-assignment:12",
    ));

    let first_session = session(30);
    let reservation = runtime
        .reserve_fresh_session(first_session)
        .expect("M reservation succeeds");
    assert_eq!(
        runtime.reserve_fresh_session(session(31)),
        Err(CarrierError::CapacityExceeded)
    );
    let actor = runtime
        .commit_fresh_session(reservation)
        .expect("durable success commits actor");
    assert!(runtime.contains_committed_session(first_session, actor));
    assert_eq!(
        runtime.remove_terminal_session(session(31), actor),
        Err(CarrierError::PlayerReservationMismatch)
    );
    runtime
        .remove_terminal_session(first_session, actor)
        .expect("authoritative terminal cleanup");
    assert!(!runtime.contains_committed_session(first_session, actor));
}

#[test]
fn qualification_spawn_uses_current_policy_and_refuses_wrong_pin_occupancy_duplicates_retirement() {
    use runtime_actor_companion::{
        CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
    };
    let mut current = runtime(4);
    let pin = current.content_pin().clone();
    let position = MovementLocalPosition {
        x: 2,
        y: 0,
        floor: 0,
    };
    assert!(
        current
            .realize_native_qualification_spawn(pin.clone(), "fixture:rat", position)
            .is_err()
    );
    let policy = CompiledCreaturePolicy {
        definition_key: "fixture:rat".into(),
        definition_revision: "fixture:1".into(),
        display_name: "fixture Rat".into(),
        maximum_health: 37,
        base_speed: 110,
        outfit_look_type: 21,
        object_look_type: None,
        summonable: false,
        convinceable: false,
        mana_cost: None,
        is_familiar: false,
        condition_immunities: vec![],
        defense: Some(1),
        armor: Some(1),
        mitigation: None,
        resistances: vec![],
        damage_immunities: vec![],
        healing_from_damage: vec![],
        preferred_distance: Some(1),
        reward_boss: Some(false),
        flags: CreatureFlags {
            attackable: true,
            illusionable: false,
            health_hidden: false,
        },
    };
    current
        .install_companion_policies(
            CompiledCreaturePolicies::from_active_artifact(
                pin.server_artifact_digest(),
                vec![policy],
            )
            .expect("qualified fixture table"),
        )
        .expect("current policy owner");
    let reserved = current
        .reserve_fresh_session(session(900))
        .expect("ordinary session reservation");
    let player = current
        .commit_fresh_session(reserved)
        .expect("actual player actor");
    current
        .initialize_first_entry_position(player)
        .expect("actual positioned owner");
    let before = current.positioned_actor_census().expect("current census");
    let other_pin = ChannelContentPin::from_activation(
        pin.world_id(),
        pin.activation_sequence() + 1,
        pin.server_artifact_digest(),
        pin.client_artifact_digest(),
        pin.frame_binding_digest(),
        pin.map_revision_digest(),
        (
            pin.entry_start().x,
            pin.entry_start().y,
            pin.entry_start().floor,
        ),
    );
    assert!(
        current
            .realize_native_qualification_spawn(other_pin, "fixture:rat", position)
            .is_err()
    );
    assert!(
        current
            .realize_native_qualification_spawn(
                pin.clone(),
                "fixture:rat",
                current
                    .read_actor_position(player)
                    .expect("actual player position")
                    .position()
            )
            .is_err()
    );
    assert_eq!(
        current.positioned_actor_census().expect("unchanged census"),
        before
    );
    let actual = current
        .realize_native_qualification_spawn(pin.clone(), "fixture:rat", position)
        .expect("actual spawn primitive");
    assert_eq!(
        (actual.health, actual.maximum_health),
        (37, 37),
        "HP comes from current policy rather than20"
    );
    assert!(actual.state.master.is_none());
    assert_eq!(actual.position, position);
    let spawned = current
        .positioned_actor_census()
        .expect("actual creature census");
    assert_eq!(
        current.realize_native_qualification_spawn(
            pin.clone(),
            "fixture:rat",
            MovementLocalPosition { x: 3, ..position }
        ),
        Err(CarrierError::DuplicateSpawnSource)
    );
    assert_eq!(
        current
            .positioned_actor_census()
            .expect("no partial duplicate admission"),
        spawned
    );
    current.retire_owner_cycle();
    assert!(
        current
            .realize_native_qualification_spawn(pin.clone(), "fixture:rat", position)
            .is_err()
    );
}

#[test]
fn definitely_uncommitted_reservation_rolls_back_but_committed_does_not() {
    let mut runtime = runtime(1);
    let first_session = session(40);
    let reservation = runtime
        .reserve_fresh_session(first_session)
        .expect("reserve");
    runtime
        .rollback_definitely_uncommitted(reservation)
        .expect("definite noncommit rollback");
    let second = runtime
        .reserve_fresh_session(session(41))
        .expect("capacity restored");
    let actor = runtime.commit_fresh_session(second).expect("commit");
    assert_eq!(
        runtime.rollback_definitely_uncommitted(second),
        Err(CarrierError::PlayerReservationMismatch)
    );
    assert!(runtime.contains_committed_session(session(41), actor));
}

#[test]
fn control_loss_is_recorded_once_for_the_exact_committed_player() {
    let mut runtime = runtime(2);
    let reservation = runtime.reserve_fresh_session(session(64)).expect("reserve");
    // An uncommitted reservation has no player facts and cannot lose control.
    let _pending = runtime.reserve_fresh_session(session(65)).expect("reserve");
    let actor = runtime.commit_fresh_session(reservation).expect("commit");
    let mark = ControlLossMark {
        epoch: 1,
        grace_deadline: 160,
    };
    let facts = runtime
        .player_control_facts(actor, session(64))
        .expect("facts");
    assert_eq!(facts.control_loss, None);
    assert_ne!(facts.placement_identity, [0; 16]);
    assert_eq!(facts.placement_revision, actor.0.actor_local_generation.0);
    // Another session never reads or marks this actor.
    assert_eq!(
        runtime.player_control_facts(actor, session(66)),
        Err(CarrierError::PlayerReservationMismatch)
    );
    assert_eq!(
        runtime.record_control_loss(actor, session(66), mark),
        Err(CarrierError::PlayerReservationMismatch)
    );
    assert_eq!(
        runtime.record_control_loss(actor, session(64), ControlLossMark { epoch: 0, ..mark }),
        Err(CarrierError::ControlLossConflict)
    );
    assert_eq!(
        runtime.record_control_loss(actor, session(64), mark),
        Ok(())
    );
    // The identical decision replays; a different one conflicts and changes nothing.
    assert_eq!(
        runtime.record_control_loss(actor, session(64), mark),
        Ok(())
    );
    assert_eq!(
        runtime.record_control_loss(
            actor,
            session(64),
            ControlLossMark {
                grace_deadline: 161,
                ..mark
            }
        ),
        Err(CarrierError::ControlLossConflict)
    );
    let after = runtime
        .player_control_facts(actor, session(64))
        .expect("facts");
    assert_eq!(after.control_loss, Some(mark));
    assert_eq!(after.placement_identity, facts.placement_identity);
    assert_eq!(runtime.player_control_loss_epochs(), vec![1]);
    // The actor stays present: the committed player count is unchanged.
    assert_eq!(runtime.player_slot_counts(), (1, 1));
    // A committed same-session recovery restores control of the same actor; only
    // the exact recorded epoch clears it, and restoring again is idempotent.
    assert_eq!(
        runtime.restore_control(actor, session(64), 2),
        Err(CarrierError::ControlLossConflict)
    );
    assert_eq!(
        runtime.restore_control(actor, session(65), 1),
        Err(CarrierError::PlayerReservationMismatch)
    );
    assert_eq!(runtime.restore_control(actor, session(64), 1), Ok(()));
    assert_eq!(runtime.player_control_loss_epochs(), Vec::<u64>::new());
    assert_eq!(runtime.restore_control(actor, session(64), 1), Ok(()));
    assert_eq!(runtime.player_slot_counts(), (1, 1));
}

#[test]
fn first_entry_position_is_written_once_and_retry_reconciles() {
    let mut runtime = runtime(2);
    let reservation = runtime.reserve_fresh_session(session(60)).expect("reserve");
    let actor = runtime.commit_fresh_session(reservation).expect("commit");
    let outcome = runtime
        .initialize_first_entry_position(actor)
        .expect("initialize");
    assert!(matches!(outcome, FirstEntryPosition::Initialized(_)));
    let first = runtime
        .borrow_movement_position()
        .read(actor)
        .expect("positioned");
    assert_eq!(
        first.position(),
        MovementLocalPosition {
            x: 0,
            y: 0,
            floor: 0
        }
    );
    assert_eq!(first.revision(), 1);
    assert_eq!(runtime.players_positioned_at_entry_start(), 1);
    // Exact retry: same actor generation, same pin -> no second write.
    assert_eq!(
        runtime.initialize_first_entry_position(actor),
        Ok(FirstEntryPosition::Reconciled(first))
    );
    assert_eq!(runtime.players_positioned_at_entry_start(), 1);
}

#[test]
fn first_entry_position_refuses_other_context_and_stale_actor() {
    let mut runtime = runtime(2);
    // An actor positioned under another context never gets a replacement.
    let reservation = runtime.reserve_fresh_session(session(61)).expect("reserve");
    let other = runtime.commit_fresh_session(reservation).expect("commit");
    let synthetic = runtime
        .initialize_movement_test_position(
            other,
            MovementLocalPosition {
                x: 1,
                y: 0,
                floor: 0,
            },
        )
        .expect("synthetic position");
    assert_eq!(
        runtime.initialize_first_entry_position(other),
        Err(CarrierError::PositionAlreadyInitialized)
    );
    assert_eq!(
        runtime.borrow_movement_position().read(other),
        Ok(synthetic)
    );
    // A removed (stale) actor reference gets no write.
    let reservation = runtime.reserve_fresh_session(session(62)).expect("reserve");
    let stale = runtime.commit_fresh_session(reservation).expect("commit");
    runtime
        .remove_terminal_session(session(62), stale)
        .expect("terminal cleanup");
    assert!(runtime.initialize_first_entry_position(stale).is_err());
    assert_eq!(runtime.players_positioned_at_entry_start(), 0);
}

#[test]
fn first_entry_position_refuses_wrong_scope_and_recycled_reference() {
    // A reference from another Channel runtime is refused without a write.
    let mut runtime = runtime(1);
    let mut other = ChannelRuntimeV1::from_committed_assignment(
        WorldId::decode(&uuid_v7(70)).expect("world"),
        ChannelId::decode(&uuid_v7(71)).expect("channel"),
        node(72),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        1,
        ChannelContentPin::test(WorldId::decode(&uuid_v7(70)).expect("world")),
    )
    .expect("other runtime");
    let reservation = other.reserve_fresh_session(session(73)).expect("reserve");
    let foreign = other.commit_fresh_session(reservation).expect("commit");
    assert!(runtime.initialize_first_entry_position(foreign).is_err());
    assert_eq!(runtime.players_positioned_at_entry_start(), 0);

    // A recycled slot: the old reference is refused; the new actor starts
    // unpositioned and gets its own first write.
    let reservation = runtime.reserve_fresh_session(session(74)).expect("reserve");
    let old = runtime.commit_fresh_session(reservation).expect("commit");
    runtime
        .remove_terminal_session(session(74), old)
        .expect("terminal cleanup");
    let reservation = runtime.reserve_fresh_session(session(75)).expect("reserve");
    let recycled = runtime.commit_fresh_session(reservation).expect("commit");
    assert!(runtime.initialize_first_entry_position(old).is_err());
    assert!(matches!(
        runtime.initialize_first_entry_position(recycled),
        Ok(FirstEntryPosition::Initialized(_))
    ));
    assert_eq!(runtime.players_positioned_at_entry_start(), 1);
}

#[test]
fn runtime_rejects_invalid_assignment_identity_and_zero_provenance() {
    let world = WorldId::decode(&uuid_v7(50)).expect("world");
    let channel = ChannelId::decode(&uuid_v7(51)).expect("channel");
    assert!(matches!(
        ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            node(52),
            0,
            1,
            1,
            "runtime-scope-assignment:1",
            1,
            ChannelContentPin::test(world),
        ),
        Err(CarrierError::InvalidAssignmentBinding)
    ));
    assert!(matches!(
        ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            node(52),
            1,
            1,
            2,
            "runtime-scope-assignment:1",
            1,
            ChannelContentPin::test(world),
        ),
        Err(CarrierError::InvalidAssignmentBinding)
    ));
    let other_world = WorldId::decode(&uuid_v7(53)).expect("other world");
    assert!(matches!(
        ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            node(52),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            1,
            ChannelContentPin::test(other_world),
        ),
        Err(CarrierError::ContentPinWorldMismatch)
    ));
    let runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        node(52),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        1,
        ChannelContentPin::test(world),
    )
    .expect("pinned runtime");
    assert_eq!(runtime.content_pin(), &ChannelContentPin::test(world));
    assert_eq!(
        size_of::<Slot>(),
        // D132/D3-3: +8 bytes (one pointer) for CreatureOccupied's boxed
        // `damage_contributors`, the same footprint-preserving pattern already used for
        // `target_identity: Arc<[u8]>` above; every non-creature slot pays this one pointer
        // too, since it is `Slot`'s largest-variant size, not per-variant.
        // D4/D140: `committed` widens from the inline `Option<OwnerCommitRecord>` to a boxed
        // `DamageReceipts` (up to 16 receipts + the owner ordinal live on the heap), which
        // shrinks the largest variant by 32 bytes: 200 -> 168.
        // COND-1c: boxing the player lifecycle replaces the inline control-loss mark with
        // one pointer and reduces the largest variant by another 8 bytes: 168 -> 160.
        // D314: SPELL-BATCH candidate budget of 184 bytes for the boxed actual spell combat
        // state and qualified companion metadata on top of COND-1c; measured 160 -> 176.
        176,
        "spell candidate fixed-slot footprint must stay inside the D314 budget"
    );
}

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

fn spawn_position_context(continuity: &NamespaceContinuityGuard) -> PreProductionPositionContext {
    PreProductionPositionContext {
        world_id: continuity.world_id,
        channel_id: continuity.channel_id,
        scope_generation: continuity.current_generation,
        coordinate_frame_marker: 1,
        map_revision_marker: 1,
        content_generation_marker: 1,
    }
}

#[test]
fn evicting_a_corpse_projection_evicts_its_reward_occurrence() {
    let (continuity, mut carrier) = carrier(4);
    let context = spawn_position_context(&continuity);
    let actor = |local: u32| {
        ActorRef::from_owner_parts(
            continuity.world_id,
            continuity.channel_id,
            continuity.current_generation,
            ActorLocalId(local),
            ActorLocalGeneration(1),
        )
    };
    let projection = |local: u32| RuntimeCorpseProjection {
        death_at_ms: None,
        occurrence: CreatureDeathOccurrenceRef {
            actor: ExactActorRef(actor(local)),
            commit_binding: Box::new([]),
            damage: 1,
            health_before: 1,
        },
        position: VersionedPosition {
            actor_local_id: ActorLocalId(local),
            actor_local_generation: ActorLocalGeneration(1),
            context,
            position: LocalPosition {
                x: 0,
                y: 0,
                floor: 7,
            },
            revision: 1,
            facing: None,
        },
    };
    for local in 0..2 {
        carrier.retain_corpse_projection(projection(local));
        carrier
            .death_reward_occurrences
            .push(DeathRewardOccurrence {
                actor: actor(local),
                character: [1; 16],
                occurrence: [2; 16],
            });
    }
    let max = u32::try_from(MAX_RETAINED_CORPSE_PROJECTIONS).expect("bounded");
    for local in 2..=max {
        carrier.retain_corpse_projection(projection(local));
    }
    assert_eq!(
        carrier.corpse_projections.len(),
        MAX_RETAINED_CORPSE_PROJECTIONS
    );
    assert!(
        carrier
            .death_reward_occurrences
            .iter()
            .all(|reward| reward.actor != actor(0))
    );
    assert!(
        carrier
            .death_reward_occurrences
            .iter()
            .any(|reward| reward.actor == actor(1))
    );
}

fn corpse_actor_at(continuity: &NamespaceContinuityGuard, local: u32) -> ExactActorRef {
    ExactActorRef(ActorRef::from_owner_parts(
        continuity.world_id,
        continuity.channel_id,
        continuity.current_generation,
        ActorLocalId(local),
        ActorLocalGeneration(1),
    ))
}

/// A projected death of local actor `local` in `continuity`'s generation.
fn corpse_projection_at(
    continuity: &NamespaceContinuityGuard,
    local: u32,
) -> RuntimeCorpseProjection {
    RuntimeCorpseProjection {
        death_at_ms: None,
        occurrence: CreatureDeathOccurrenceRef {
            actor: corpse_actor_at(continuity, local),
            commit_binding: Box::new([]),
            damage: 1,
            health_before: 1,
        },
        position: VersionedPosition {
            actor_local_id: ActorLocalId(local),
            actor_local_generation: ActorLocalGeneration(1),
            context: spawn_position_context(continuity),
            position: LocalPosition {
                x: 0,
                y: 0,
                floor: 7,
            },
            revision: 1,
            facing: None,
        },
    }
}

/// Codex 4248192466 (LOOT-LIVE-1): a projection pinned by an accepted kill settlement
/// survives more than 64 newer projections, such as those of kills a full queue refuses,
/// while the unpinned ones keep their FIFO bound. Once the settle is final and the pin is
/// released, the next projection evicts it as the oldest unpinned one.
#[test]
fn a_pinned_corpse_projection_survives_newer_projections_until_unpinned() {
    let (continuity, mut carrier) = carrier(4);
    let pinned = corpse_actor_at(&continuity, 0);
    carrier.retain_corpse_projection(corpse_projection_at(&continuity, 0));
    carrier
        .death_reward_occurrences
        .push(DeathRewardOccurrence {
            actor: pinned.0,
            character: [1; 16],
            occurrence: [2; 16],
        });
    assert_eq!(carrier.pin_corpse_projection_inner(pinned), Ok(()));
    // Pinning again is a no-op.
    assert_eq!(carrier.pin_corpse_projection_inner(pinned), Ok(()));
    assert_eq!(carrier.corpse_projection_pins, vec![pinned]);
    let max = u32::try_from(MAX_RETAINED_CORPSE_PROJECTIONS).expect("bounded");
    for local in 1..=2 * max {
        carrier.retain_corpse_projection(corpse_projection_at(&continuity, local));
    }
    assert!(carrier.retains_corpse_projection(pinned));
    assert!(
        carrier
            .death_reward_occurrences
            .iter()
            .any(|reward| reward.actor == pinned.0)
    );
    assert_eq!(
        carrier.corpse_projections.len(),
        MAX_RETAINED_CORPSE_PROJECTIONS + 1
    );
    assert!(!carrier.retains_corpse_projection(corpse_actor_at(&continuity, max)));
    assert!(carrier.retains_corpse_projection(corpse_actor_at(&continuity, max + 1)));

    carrier.unpin_corpse_projection_inner(pinned);
    assert!(carrier.corpse_projection_pins.is_empty());
    carrier.retain_corpse_projection(corpse_projection_at(&continuity, 2 * max + 1));
    assert!(!carrier.retains_corpse_projection(pinned));
    assert!(
        carrier
            .death_reward_occurrences
            .iter()
            .all(|reward| reward.actor != pinned.0)
    );
    assert_eq!(
        carrier.corpse_projections.len(),
        MAX_RETAINED_CORPSE_PROJECTIONS
    );
}

/// Codex 4248192466: at most `MAX_PINNED_CORPSE_PROJECTIONS` pins, each on a retained
/// projection of this generation, so `corpse_projections` never holds more than 128.
#[test]
fn corpse_projection_pins_are_bounded_and_name_retained_projections() {
    let (continuity, mut carrier) = carrier(4);
    let max = u32::try_from(MAX_PINNED_CORPSE_PROJECTIONS).expect("bounded");
    assert_eq!(
        carrier.pin_corpse_projection_inner(corpse_actor_at(&continuity, 0)),
        Err(CarrierError::CommittedLethalUnavailable),
        "no projection is retained yet"
    );
    for local in 0..max {
        carrier.retain_corpse_projection(corpse_projection_at(&continuity, local));
        assert_eq!(
            carrier.pin_corpse_projection_inner(corpse_actor_at(&continuity, local)),
            Ok(())
        );
    }
    carrier.retain_corpse_projection(corpse_projection_at(&continuity, max));
    assert_eq!(
        carrier.pin_corpse_projection_inner(corpse_actor_at(&continuity, max)),
        Err(CarrierError::CapacityExceeded)
    );
    for local in max + 1..=3 * max {
        carrier.retain_corpse_projection(corpse_projection_at(&continuity, local));
    }
    assert_eq!(
        carrier.corpse_projections.len(),
        MAX_PINNED_CORPSE_PROJECTIONS + MAX_RETAINED_CORPSE_PROJECTIONS
    );
    assert!(
        (0..max)
            .all(|local| carrier.retains_corpse_projection(corpse_actor_at(&continuity, local)))
    );
}

/// D115/D116: one spawn, two placement cells, population 2, 60 s respawn / 5 s retry.
fn d116_definition() -> SpawnDefinition {
    SpawnDefinition::new(
        vec![(1, 0, 7), (2, 0, 7)],
        2,
        60_000_000,
        5_000_000,
        "oteryn:creature/rat".to_string(),
        20,
    )
    .expect("D116 definition is within every registered spawn bound")
}

// SPAWN-1a: activation realization and the §6.3 respawn drive on the composed runtime.
const SPAWN_DELAY: u64 = 60_000_000;
const SPAWN_RETRY: u64 = 5_000_000;
const DEN: MovementLocalPosition = MovementLocalPosition {
    x: 2,
    y: 0,
    floor: 7,
};

/// The reference 18 x 14 view (MOVE-RL-11) on one floor: x -8..=9, y -6..=7.
fn reference_view(observer: MovementLocalPosition, cell: MovementLocalPosition) -> bool {
    (-8..=9).contains(&(cell.x - observer.x)) && (-6..=7).contains(&(cell.y - observer.y))
}

fn den_definition() -> SpawnDefinition {
    SpawnDefinition::new(
        vec![(2, 0, 7), (2, -1, 7)],
        2,
        SPAWN_DELAY,
        SPAWN_RETRY,
        "oteryn:creature/rat".to_string(),
        20,
    )
    .expect("den definition")
}

fn spawned_runtime(definition: SpawnDefinition) -> ChannelRuntimeV1 {
    let mut runtime = runtime(8);
    assert_eq!(runtime.realize_spawn_definitions(vec![definition]), Ok(2));
    runtime
}

fn kill_creature(runtime: &mut ChannelRuntimeV1, actor: ExactActorRef) {
    let index = runtime
        .carrier
        .validate_ref(&runtime.continuity, actor.0)
        .expect("live creature");
    let slot = &mut runtime.carrier.slots[index];
    assert!(
        matches!(slot, Slot::CreatureOccupied { .. }),
        "creature slot"
    );
    if let Slot::CreatureOccupied { health, .. } = slot {
        *health = 0;
    }
}

fn place_creature(runtime: &mut ChannelRuntimeV1, cell: MovementLocalPosition) {
    let context = runtime.pinned_position_context();
    let actor = runtime
        .carrier
        .admit_creature(&runtime.continuity, ActorState(0), "test:blocker", 20)
        .expect("blocker");
    runtime
        .carrier
        .initialize_position(
            &runtime.continuity,
            actor,
            context,
            LocalPosition {
                x: cell.x,
                y: cell.y,
                floor: cell.floor,
            },
        )
        .expect("blocker position");
}

fn place_player(runtime: &mut ChannelRuntimeV1, raw: u64, cell: MovementLocalPosition) {
    let reservation = runtime
        .reserve_fresh_session(session(raw))
        .expect("player reservation");
    let actor = runtime
        .commit_fresh_session(reservation)
        .expect("player commit");
    let context = runtime.pinned_position_context();
    runtime
        .carrier
        .initialize_position(
            &runtime.continuity,
            actor.0,
            context,
            LocalPosition {
                x: cell.x,
                y: cell.y,
                floor: cell.floor,
            },
        )
        .expect("player position");
}

fn den_creature(runtime: &ChannelRuntimeV1) -> Option<ExactActorRef> {
    runtime.spawn_point_creatures()[0].2
}

#[test]
fn activation_realizes_one_creature_per_den_cell_with_its_authored_health() {
    let runtime = spawned_runtime(den_definition());
    let points = runtime.spawn_point_creatures();
    assert_eq!(points.len(), 2);
    let census = runtime.positioned_actor_census().expect("census");
    for ((_, cell, live), expected) in points.iter().zip([(2, 0), (2, -1)]) {
        assert_eq!((cell.x, cell.y), expected);
        let actor = live.expect("live rat");
        let (_, snapshot, session) = census
            .iter()
            .find(|(listed, _, _)| *listed == actor)
            .expect("positioned before admission of players");
        assert_eq!(*session, None);
        assert_eq!(snapshot.position(), *cell);
        let index = runtime
            .carrier
            .validate_ref(&runtime.continuity, actor.0)
            .expect("live");
        assert!(matches!(
            runtime.carrier.slots[index],
            Slot::CreatureOccupied { health: 20, .. }
        ));
    }
}

#[test]
fn activation_realizes_once_and_a_restart_realizes_the_same_set() {
    let mut first = spawned_runtime(den_definition());
    assert_eq!(
        first.realize_spawn_definitions(vec![den_definition()]),
        Err(CarrierError::DuplicateSpawnSource)
    );
    let cells = |runtime: &ChannelRuntimeV1| {
        runtime
            .spawn_point_creatures()
            .iter()
            .map(|(point, cell, live)| (*point, *cell, live.is_some()))
            .collect::<Vec<_>>()
    };
    let restarted = spawned_runtime(den_definition());
    assert_eq!(cells(&first), cells(&restarted));
}

#[test]
fn a_dead_rat_warns_one_full_delay_later_and_is_admitted_4200_ms_after_the_warning() {
    let mut runtime = spawned_runtime(den_definition());
    let dead = den_creature(&runtime).expect("live rat");
    kill_creature(&mut runtime, dead);
    // Observed at this drive: the one occurrence is due one full delay later.
    assert_eq!(
        runtime.drive_spawn_respawns(1_000, reference_view),
        Ok(SpawnDriveReport::default())
    );
    let point = runtime.spawn_point_creatures()[0].0;
    assert_eq!(
        runtime.pending_respawn(point).map(|p| p.due_micros),
        Some(1_000 + SPAWN_DELAY)
    );
    // At most one pending occurrence per point: a later drive schedules nothing more.
    runtime
        .drive_spawn_respawns(2_000, reference_view)
        .expect("drive");
    assert_eq!(
        runtime.pending_respawn(point).map(|p| p.due_micros),
        Some(1_000 + SPAWN_DELAY)
    );
    assert_eq!(
        runtime
            .drive_spawn_respawns(SPAWN_DELAY, reference_view)
            .expect("drive"),
        SpawnDriveReport::default()
    );
    let warned = runtime
        .drive_spawn_respawns(1_000 + SPAWN_DELAY, reference_view)
        .expect("drive");
    assert_eq!(warned.warnings, vec![DEN]);
    assert!(warned.admitted.is_empty());
    let admit_at = 1_000 + SPAWN_DELAY + spawn::SPAWN_WARNING_MICROS;
    assert!(
        runtime
            .drive_spawn_respawns(admit_at - 1, reference_view)
            .expect("drive")
            .admitted
            .is_empty()
    );
    let admitted = runtime
        .drive_spawn_respawns(admit_at, reference_view)
        .expect("drive");
    assert_eq!(admitted.admitted.len(), 1);
    let fresh = den_creature(&runtime).expect("respawned rat");
    assert_ne!(fresh, dead);
    assert_eq!(fresh, admitted.admitted[0]);
    assert_eq!(runtime.pending_respawn(point), None);
}

#[test]
fn an_occupied_den_retries_three_times_every_retry_interval_then_skips() {
    let mut runtime = spawned_runtime(den_definition());
    let dead = den_creature(&runtime).expect("live rat");
    kill_creature(&mut runtime, dead);
    runtime
        .drive_spawn_respawns(0, reference_view)
        .expect("observe");
    place_creature(&mut runtime, DEN);
    let mut now = SPAWN_DELAY;
    assert_eq!(
        runtime
            .drive_spawn_respawns(now, reference_view)
            .expect("warn")
            .warnings,
        vec![DEN]
    );
    now += spawn::SPAWN_WARNING_MICROS;
    for _ in 0..spawn::OCCUPANCY_RETRIES_MAX {
        assert_eq!(
            runtime
                .drive_spawn_respawns(now, reference_view)
                .expect("occupied")
                .retried,
            1
        );
        assert!(
            runtime
                .drive_spawn_respawns(now + SPAWN_RETRY - 1, reference_view)
                .expect("wait")
                .warnings
                .is_empty()
        );
        now += SPAWN_RETRY;
        assert_eq!(
            runtime
                .drive_spawn_respawns(now, reference_view)
                .expect("retry")
                .warnings,
            vec![DEN]
        );
        now += spawn::SPAWN_WARNING_MICROS;
    }
    let skipped = runtime
        .drive_spawn_respawns(now, reference_view)
        .expect("skip");
    assert_eq!((skipped.skipped, skipped.admitted.len()), (1, 0));
    let point = runtime.spawn_point_creatures()[0].0;
    let pending = runtime.pending_respawn(point).expect("successor");
    assert_eq!(
        (pending.due_micros, pending.successor),
        (now + SPAWN_DELAY, 1)
    );
    assert_eq!(den_creature(&runtime), None);
}

#[test]
fn player_interest_blocks_a_blockable_den_and_the_successor_is_a_full_delay_later() {
    let mut runtime = spawned_runtime(den_definition().with_blockable(true));
    let dead = den_creature(&runtime).expect("live rat");
    kill_creature(&mut runtime, dead);
    place_player(
        &mut runtime,
        40,
        MovementLocalPosition {
            x: 0,
            y: 0,
            floor: 7,
        },
    );
    runtime
        .drive_spawn_respawns(0, reference_view)
        .expect("observe");
    let blocked = runtime
        .drive_spawn_respawns(SPAWN_DELAY, reference_view)
        .expect("blocked");
    assert_eq!((blocked.blocked, blocked.warnings.len()), (1, 0));
    let point = runtime.spawn_point_creatures()[0].0;
    assert_eq!(
        runtime.pending_respawn(point).map(|p| p.due_micros),
        Some(2 * SPAWN_DELAY)
    );
}

#[test]
fn player_interest_never_blocks_the_non_blockable_fixture_point() {
    let mut runtime = spawned_runtime(den_definition());
    let dead = den_creature(&runtime).expect("live rat");
    kill_creature(&mut runtime, dead);
    place_player(
        &mut runtime,
        41,
        MovementLocalPosition {
            x: 0,
            y: 0,
            floor: 7,
        },
    );
    runtime
        .drive_spawn_respawns(0, reference_view)
        .expect("observe");
    let warned = runtime
        .drive_spawn_respawns(SPAWN_DELAY, reference_view)
        .expect("warned");
    assert_eq!((warned.blocked, warned.warnings), (0, vec![DEN]));
}

#[test]
fn a_player_on_another_floor_does_not_block_a_blockable_den() {
    let mut runtime = spawned_runtime(den_definition().with_blockable(true));
    let dead = den_creature(&runtime).expect("live rat");
    kill_creature(&mut runtime, dead);
    place_player(
        &mut runtime,
        42,
        MovementLocalPosition {
            x: 2,
            y: 0,
            floor: 6,
        },
    );
    runtime
        .drive_spawn_respawns(0, reference_view)
        .expect("observe");
    let warned = runtime
        .drive_spawn_respawns(SPAWN_DELAY, reference_view)
        .expect("warned");
    assert_eq!((warned.blocked, warned.warnings), (0, vec![DEN]));
}

#[test]
fn realize_spawn_admits_population_creatures_at_declared_cells() {
    let (continuity, mut carrier) = carrier(4);
    let context = spawn_position_context(&continuity);
    carrier
        .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
        .expect("D116 realizes");

    let state_0 = carrier
        .spawn_cell_state(SpawnSourceId(1), 0)
        .expect("cell 0 tracked");
    let state_1 = carrier
        .spawn_cell_state(SpawnSourceId(1), 1)
        .expect("cell 1 tracked");
    let (actor_0, actor_1) = (
        state_0.live.expect("cell 0 has a live rat"),
        state_1.live.expect("cell 1 has a live rat"),
    );
    assert_ne!(actor_0, actor_1);
    assert!(
        carrier
            .current_owner_exact_lookup(&continuity)
            .contains(actor_0)
    );
    assert!(
        carrier
            .current_owner_exact_lookup(&continuity)
            .contains(actor_1)
    );
    assert_eq!(
        carrier
            .read_position(&continuity, actor_0.0)
            .map(|snapshot| snapshot.version.position),
        Ok(LocalPosition {
            x: 1,
            y: 0,
            floor: 7
        })
    );
}

#[test]
fn realize_spawn_rejects_duplicate_source_without_mutation() {
    let (continuity, mut carrier) = carrier(4);
    let context = spawn_position_context(&continuity);
    carrier
        .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
        .expect("first realization");
    let before = carrier.slots.clone();
    assert_eq!(
        carrier.realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context),
        Err(CarrierError::DuplicateSpawnSource)
    );
    assert_eq!(carrier.slots, before);
}

#[test]
fn realize_spawn_with_an_invalid_context_mutates_nothing() {
    // P1 (Codex review, PR #1193): the context is now validated before any admission, so a
    // rejected realization never leaves an untracked, unpositioned creature behind; a
    // retry with a valid context starts from byte-identical state.
    let (continuity, mut carrier) = carrier(4);
    let mut invalid_context = spawn_position_context(&continuity);
    invalid_context.coordinate_frame_marker = 0;
    let slots_before = carrier.slots.clone();
    let free_head_before = carrier.free_head;
    assert_eq!(
        carrier.realize_spawn(
            &continuity,
            SpawnSourceId(1),
            d116_definition(),
            invalid_context
        ),
        Err(CarrierError::InvalidPreProductionPositionContext)
    );
    assert_eq!(carrier.slots, slots_before);
    assert_eq!(carrier.free_head, free_head_before);
    assert!(carrier.spawns.is_empty());

    let context = spawn_position_context(&continuity);
    carrier
        .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
        .expect("a valid context still realizes the spawn afterward");
}

/// A one-source definition with `points` cells on one row, starting at x = `first_x`.
fn row_definition(first_x: i32, points: usize) -> SpawnDefinition {
    let cells = (0..points)
        .map(|offset| (first_x + i32::try_from(offset).expect("small"), 0, 7))
        .collect::<Vec<_>>();
    SpawnDefinition::new(
        cells,
        points,
        60_000_000,
        5_000_000,
        "oteryn:creature/rat".to_string(),
        20,
    )
    .expect("within CREATUREAI0-RL-03")
}

/// `CREATUREAI0-RL-03`: 64 points (one cell and one creature each) per source pass and
/// 65 refuse, whether counted by cells or by population.
#[test]
fn spawn_points_per_source_max_accepted_max_plus_one_rejected() {
    assert_eq!(CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX, 64);
    let cells = |count: usize| {
        (0..count)
            .map(|x| (i32::try_from(x).expect("small"), 0, 0))
            .collect::<Vec<(i32, i32, i16)>>()
    };
    let definition = |cells, population| {
        SpawnDefinition::new(
            cells,
            population,
            60_000_000,
            5_000_000,
            "oteryn:creature/rat".to_string(),
            20,
        )
    };
    let max = CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX;
    assert!(definition(cells(max), max).is_ok());
    assert_eq!(
        definition(cells(max + 1), max + 1),
        Err(CarrierError::InvalidSpawnDefinition)
    );
    assert_eq!(
        definition(cells(max), max + 1),
        Err(CarrierError::InvalidSpawnDefinition)
    );
    assert_eq!(
        definition(cells(max + 1), max),
        Err(CarrierError::InvalidSpawnDefinition)
    );
    // The former first-slice bound (4) no longer limits a source.
    let mut runtime = runtime(2 * max);
    assert_eq!(
        runtime.realize_spawn_definitions(vec![row_definition(0, max)]),
        Ok(max)
    );
    assert_eq!(runtime.spawn_point_creatures().len(), max);
}

/// `CREATUREAI0-RL-01`: the channel ceiling on spawn points is checked as a sum, not as
/// the product of the per-source and source-count rows. 131,072 points pass; one more
/// refuses; so does more than `CREATUREAI0-RL-02` sources, whatever their points.
#[test]
fn spawn_points_per_channel_ceiling_is_exact() {
    assert_eq!(CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX, 131_072);
    let max = CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX;
    let per_source = CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX;
    let mut definitions = (0..max / per_source)
        .map(|_| row_definition(0, per_source))
        .collect::<Vec<_>>();
    assert_eq!(spawn_points_within_ceilings(&definitions), Ok(max));
    definitions.push(row_definition(0, 1));
    assert_eq!(
        spawn_points_within_ceilings(&definitions),
        Err(CarrierError::CapacityExceeded)
    );
    let one_point_sources = vec![row_definition(0, 1); CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX];
    assert_eq!(
        spawn_points_within_ceilings(&one_point_sources),
        Ok(CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX)
    );
    let mut over_sources = one_point_sources;
    over_sources.push(row_definition(0, 1));
    assert_eq!(
        spawn_points_within_ceilings(&over_sources),
        Err(CarrierError::CapacityExceeded)
    );

    // At the runtime, one point over refuses before any admission or reservation, even
    // with free slots to spare.
    let mut runtime = runtime(8);
    let slots_before = runtime.carrier.slots.clone();
    assert_eq!(
        runtime.realize_spawn_definitions(definitions),
        Err(CarrierError::CapacityExceeded)
    );
    assert_eq!(runtime.carrier.slots, slots_before);
    assert!(runtime.carrier.spawns.is_empty());
    assert!(runtime.carrier.occupied.is_empty());
}

/// `CREATUREAI0-RL-02` at the runtime: 65,536 one-point sources realize in canonical
/// order (the former 16-source ceiling no longer refuses readiness); 65,537 refuse with
/// nothing admitted.
#[test]
fn spawn_sources_per_channel_max_accepted_max_plus_one_rejected() {
    assert_eq!(CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX, 65_536);
    let max = CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX;
    let sources = |count: usize| {
        (0..count)
            .map(|index| row_definition(i32::try_from(index % 256).expect("small"), 1))
            .collect::<Vec<_>>()
    };

    let mut over = runtime(max + 1);
    let slots_before = over.carrier.slots.clone();
    assert_eq!(
        over.realize_spawn_definitions(sources(max + 1)),
        Err(CarrierError::CapacityExceeded)
    );
    assert_eq!(over.carrier.slots, slots_before);
    assert!(over.carrier.spawns.is_empty());
    assert!(over.carrier.occupied.is_empty());
    drop(over);

    let mut runtime = runtime(max + SPAWN_ACTIVATION_PLAYER_HEADROOM);
    // More than one owner window: one turn refuses, owner windows realize it.
    assert_eq!(
        runtime.realize_spawn_definitions(sources(max)),
        Err(CarrierError::CapacityExceeded)
    );
    assert!(runtime.carrier.spawns.is_empty());
    assert_eq!(
        realize_in_owner_windows(&mut runtime, sources(max)),
        (max, max / spawn::REALIZATIONS_PER_WINDOW_MAX)
    );
    assert_eq!(runtime.carrier.spawns.len(), max);
    assert!(
        runtime
            .carrier
            .spawns
            .windows(2)
            .all(|pair| pair[0].source.0 < pair[1].source.0)
    );
    let last = SpawnSourceId(u16::MAX);
    assert!(runtime.carrier.spawn_cell_state(last, 0).is_ok());
    // Every source ordinal is taken: a further single source is a duplicate, and nothing
    // changes.
    let context = runtime.pinned_position_context();
    assert_eq!(
        runtime
            .carrier
            .realize_spawn(&runtime.continuity, last, row_definition(0, 1), context),
        Err(CarrierError::DuplicateSpawnSource)
    );
    assert_eq!(runtime.carrier.spawns.len(), max);
}

/// The creature key and health of live creature `actor`.
fn spawned_creature(runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> Option<(Vec<u8>, i64)> {
    let index = runtime
        .carrier
        .validate_ref(&runtime.continuity, actor.0)
        .ok()?;
    match &runtime.carrier.slots[index] {
        Slot::CreatureOccupied {
            target_identity,
            health,
            ..
        } => Some((target_identity.to_vec(), *health)),
        _ => None,
    }
}

/// SPAWN-1b (Codex P1 on #1981): one authored source whose points mix creatures, respawn
/// delays and initial health stays one runtime source. Its point ordinals run 0..3 in
/// authored order without a reset, each point realizes its own creature with its own
/// health, and each respawns after its own delay as its own creature.
#[test]
fn spawn1b_mixed_point_source_is_one_runtime_source_with_per_point_facts() {
    const RAT: &str = "oteryn:creature/rat";
    const BAT: &str = "oteryn:creature/bat";
    let rat = || Some((RAT.as_bytes().to_vec(), 20));
    let bat = || Some((BAT.as_bytes().to_vec(), 35));
    let mut runtime = runtime(8);
    let source: ActivationSpawnSourceFacts = (
        vec![
            ((2, 0, 7), 60_000, RAT.to_string(), 20, false),
            ((3, 0, 7), 30_000, BAT.to_string(), 35, false),
            ((4, 0, 7), 30_000, RAT.to_string(), 20, false),
        ],
        5_000,
    );
    assert_eq!(runtime.begin_activation_spawn_sources(vec![source]), Ok(3));
    let progress = runtime
        .realize_activation_spawn_window(0)
        .expect("one window");
    assert_eq!((progress.realized, progress.remaining), (3, 0));
    assert_eq!(runtime.carrier.spawns.len(), 1);
    let points = runtime.spawn_point_creatures();
    assert_eq!(
        points
            .iter()
            .map(|(point, cell, _)| (point.source, point.ordinal, cell.x))
            .collect::<Vec<_>>(),
        vec![(0, 0, 2), (0, 1, 3), (0, 2, 4)]
    );
    let live = points
        .iter()
        .map(|(_, _, live)| live.expect("live creature"))
        .collect::<Vec<_>>();
    assert_eq!(
        live.iter()
            .map(|actor| spawned_creature(&runtime, *actor))
            .collect::<Vec<_>>(),
        vec![rat(), bat(), rat()]
    );

    for actor in &live {
        kill_creature(&mut runtime, *actor);
    }
    runtime
        .drive_spawn_respawns(1_000, reference_view)
        .expect("observe");
    assert_eq!(
        points
            .iter()
            .map(|(point, _, _)| runtime.pending_respawn(*point).map(|p| p.due_micros))
            .collect::<Vec<_>>(),
        vec![
            Some(1_000 + 60_000_000),
            Some(1_000 + 30_000_000),
            Some(1_000 + 30_000_000)
        ]
    );
    let warned = runtime
        .drive_spawn_respawns(1_000 + 30_000_000, reference_view)
        .expect("warn");
    assert_eq!(warned.warnings.len(), 2);
    let admitted = runtime
        .drive_spawn_respawns(
            1_000 + 30_000_000 + spawn::SPAWN_WARNING_MICROS,
            reference_view,
        )
        .expect("admit");
    assert_eq!(
        admitted
            .admitted
            .iter()
            .map(|actor| spawned_creature(&runtime, *actor))
            .collect::<Vec<_>>(),
        vec![bat(), rat()]
    );
    assert_eq!(
        runtime
            .pending_respawn(points[0].0)
            .map(|pending| pending.due_micros),
        Some(1_000 + 60_000_000)
    );
}

/// SPAWN-1b (Codex P2 on #2002): blockability is per point (CREATURE-AI-0 §6.3, the point's
/// creature profile). In one source mixing a blockable rat and a non-blockable bat, a
/// player whose interest area holds both cells blocks the rat's respawn (`BLOCKED`, a
/// successor a full delay later) while the bat's warns and admits as usual.
#[test]
fn spawn1b_mixed_source_blocks_only_its_blockable_points() {
    const RAT: &str = "oteryn:creature/rat";
    const BAT: &str = "oteryn:creature/bat";
    const BAT_CELL: MovementLocalPosition = MovementLocalPosition {
        x: 3,
        y: 0,
        floor: 7,
    };
    let source: ActivationSpawnSourceFacts = (
        vec![
            ((2, 0, 7), 60_000, RAT.to_string(), 20, true),
            ((3, 0, 7), 60_000, BAT.to_string(), 35, false),
        ],
        5_000,
    );
    let definition = SpawnDefinition::from_activation_source(source.clone()).expect("valid source");
    assert_eq!(
        [0, 1].map(|ordinal| definition.timing(ordinal).map(|timing| timing.blockable)),
        [Ok(true), Ok(false)]
    );

    let mut runtime = runtime(8);
    assert_eq!(runtime.begin_activation_spawn_sources(vec![source]), Ok(2));
    let progress = runtime
        .realize_activation_spawn_window(0)
        .expect("one window");
    assert_eq!(progress.remaining, 0);
    let points = runtime.spawn_point_creatures();
    for (_, _, live) in &points {
        kill_creature(&mut runtime, live.expect("live creature"));
    }
    place_player(
        &mut runtime,
        43,
        MovementLocalPosition {
            x: 0,
            y: 0,
            floor: 7,
        },
    );
    runtime
        .drive_spawn_respawns(0, reference_view)
        .expect("observe");
    let due = runtime
        .drive_spawn_respawns(SPAWN_DELAY, reference_view)
        .expect("due");
    assert_eq!((due.blocked, due.warnings), (1, vec![BAT_CELL]));
    assert_eq!(
        runtime.pending_respawn(points[0].0).map(|p| p.due_micros),
        Some(2 * SPAWN_DELAY)
    );
    let admitted = runtime
        .drive_spawn_respawns(SPAWN_DELAY + spawn::SPAWN_WARNING_MICROS, reference_view)
        .expect("admit");
    assert_eq!(
        admitted
            .admitted
            .iter()
            .map(|actor| spawned_creature(&runtime, *actor))
            .collect::<Vec<_>>(),
        vec![Some((BAT.as_bytes().to_vec(), 35))]
    );
    assert_eq!(
        runtime.pending_respawn(points[0].0).map(|p| p.due_micros),
        Some(2 * SPAWN_DELAY)
    );
}

/// SPAWN-1b: a mixed source keeps every per-source check: `CREATUREAI0-RL-03` (64 points
/// pass, 65 refuse), a positive health on every point and no empty source. A refused
/// source refuses the whole activation with nothing begun.
#[test]
fn spawn1b_mixed_point_source_keeps_the_per_source_checks() {
    let points = |count: usize, health: u64| {
        (0..count)
            .map(|x| {
                let key = if x % 2 == 0 {
                    "oteryn:creature/rat"
                } else {
                    "oteryn:creature/bat"
                };
                (
                    (i32::try_from(x).expect("small"), 0, 7),
                    60_000,
                    key.to_string(),
                    health,
                    false,
                )
            })
            .collect::<Vec<_>>()
    };
    let max = CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX;
    assert!(SpawnDefinition::from_activation_source((points(max, 20), 5_000)).is_ok());
    for refused in [
        (points(max + 1, 20), 5_000),
        (points(0, 20), 5_000),
        (points(2, 0), 5_000),
        (
            vec![(
                (0, 0, 7),
                u64::MAX,
                "oteryn:creature/rat".to_string(),
                20,
                false,
            )],
            5_000,
        ),
        (points(1, u64::MAX), 5_000),
    ] {
        assert_eq!(
            SpawnDefinition::from_activation_source(refused.clone()),
            Err(CarrierError::InvalidSpawnDefinition)
        );
        let mut runtime = runtime(256);
        assert_eq!(
            runtime.begin_activation_spawn_sources(vec![(points(1, 20), 5_000), refused]),
            Err(CarrierError::InvalidSpawnDefinition)
        );
        assert!(runtime.spawn_activation.is_none());
        assert!(runtime.carrier.spawns.is_empty());
    }
}

/// SPAWN-1b (Codex P1 on #1981): the pinned preprod bundle's scale, 51,078 authored sources
/// with 82,673 active points of mixed creatures and delays, is 51,078 runtime sources: it
/// passes `CREATUREAI0-RL-02` and `CREATUREAI0-RL-01` and realizes window by window. Split
/// into its 70,206 creature/delay runs it would refuse `CapacityExceeded`, and so does
/// 65,537 sources through the per-point entry.
#[test]
fn spawn1b_bundle_scale_sources_stay_under_the_source_ceiling() {
    const SOURCES: usize = 51_078;
    const POINTS: usize = 82_673;
    const SPLIT_RUNS: usize = 70_206;
    // 31,595 sources of a rat and a bat with different delays, the rest one rat each.
    let mixed = POINTS - SOURCES;
    let source = |index: usize, two: bool| -> ActivationSpawnSourceFacts {
        let x = i32::try_from(index % 256).expect("small");
        let y = i32::try_from(index / 256).expect("small");
        let mut points = vec![(
            (x, y, 7),
            60_000,
            "oteryn:creature/rat".to_string(),
            20,
            false,
        )];
        if two {
            points.push((
                (x, y + 512, 7),
                30_000,
                "oteryn:creature/bat".to_string(),
                35,
                false,
            ));
        }
        (points, 5_000)
    };
    let sources = (0..SOURCES)
        .map(|index| source(index, index < mixed))
        .collect::<Vec<_>>();
    let definitions = sources
        .iter()
        .cloned()
        .map(SpawnDefinition::from_activation_source)
        .collect::<Result<Vec<_>, _>>()
        .expect("valid sources");
    assert_eq!(definitions.len(), SOURCES);
    assert_eq!(spawn_points_within_ceilings(&definitions), Ok(POINTS));
    let split_runs = vec![row_definition(0, 1); SPLIT_RUNS];
    assert_eq!(
        spawn_points_within_ceilings(&split_runs),
        Err(CarrierError::CapacityExceeded)
    );
    drop(split_runs);

    let mut over = runtime(8);
    assert_eq!(
        over.begin_activation_spawn_sources(
            (0..=CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX)
                .map(|index| source(index, false))
                .collect()
        ),
        Err(CarrierError::CapacityExceeded)
    );
    assert!(over.spawn_activation.is_none());
    drop(over);

    let mut runtime = runtime(POINTS + SPAWN_ACTIVATION_PLAYER_HEADROOM);
    assert_eq!(runtime.begin_activation_spawn_sources(sources), Ok(POINTS));
    let mut now = 0;
    loop {
        let progress = runtime
            .realize_activation_spawn_window(now)
            .expect("one window");
        if progress.remaining == 0 {
            assert_eq!(progress.realized_total, POINTS);
            break;
        }
        now += spawn::OWNER_WINDOW_MICROS;
    }
    assert_eq!(runtime.carrier.spawns.len(), SOURCES);
    let points = runtime.spawn_point_creatures();
    assert_eq!(points.len(), POINTS);
    let first = points
        .iter()
        .take(2)
        .map(|(point, _, live)| {
            (
                (point.source, point.ordinal),
                spawned_creature(&runtime, live.expect("live")),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        first,
        vec![
            ((0, 0), Some((b"oteryn:creature/rat".to_vec(), 20))),
            ((0, 1), Some((b"oteryn:creature/bat".to_vec(), 35))),
        ]
    );
}

/// Runs a begun activation one window per owner window from owner time 0, checking that no
/// owner window realizes more than `CREATUREAI0-RL-14` points and that a second call in an
/// owner window realizes nothing. Returns the creatures realized and the windows used.
fn realize_in_owner_windows(
    runtime: &mut ChannelRuntimeV1,
    definitions: Vec<SpawnDefinition>,
) -> (usize, usize) {
    let total = runtime.begin_spawn_definitions(definitions).expect("begin");
    let mut now = 0;
    let mut windows = 0;
    loop {
        let progress = runtime
            .realize_activation_spawn_window(now)
            .expect("one window");
        assert!(progress.realized <= spawn::REALIZATIONS_PER_WINDOW_MAX);
        windows += 1;
        if progress.remaining == 0 {
            assert_eq!(progress.realized_total, total);
            assert!(runtime.spawn_activation.is_none());
            return (total, windows);
        }
        let again = runtime
            .realize_activation_spawn_window(now + spawn::OWNER_WINDOW_MICROS - 1)
            .expect("same owner window");
        assert_eq!(again.realized, 0);
        assert_eq!(again.remaining, progress.remaining);
        now += spawn::OWNER_WINDOW_MICROS;
    }
}

/// SPAWN-CEILINGS-1 (Codex 4242094682): `CREATUREAI0-RL-14` holds above one window. One
/// owner turn refuses an activation of more than 4,096 points with nothing realized; begun
/// instead, it realizes exactly the `spawn::realization_windows` plan, one window per owner
/// window, in canonical order, a source spanning two windows included, and the respawn
/// drive does nothing until it completes.
#[test]
fn activation_realizes_at_most_rl_14_points_per_owner_window() {
    // 131 sources of 63 points: 8,253 points, windows of 4,096, 4,096 and 61; source 65
    // has 1 point in the first window and 62 in the second.
    let definitions = || (0..131).map(|_| row_definition(0, 63)).collect::<Vec<_>>();
    let mut runtime = runtime(8_253 + SPAWN_ACTIVATION_PLAYER_HEADROOM);
    let slots_before = runtime.carrier.slots.clone();
    assert_eq!(
        runtime.realize_spawn_definitions(definitions()),
        Err(CarrierError::CapacityExceeded)
    );
    assert_eq!(runtime.carrier.slots, slots_before);
    assert!(runtime.carrier.spawns.is_empty() && runtime.spawn_activation.is_none());

    assert_eq!(runtime.begin_spawn_definitions(definitions()), Ok(8_253));
    assert_eq!(runtime.carrier.slots, slots_before, "begin admits nothing");
    assert_eq!(
        runtime.begin_spawn_definitions(definitions()),
        Err(CarrierError::DuplicateSpawnSource)
    );
    let plan = spawn::realization_windows(
        &(0..131_u16)
            .flat_map(|source| (0..63_u8).map(move |ordinal| spawn::SpawnPoint { source, ordinal }))
            .collect::<Vec<_>>(),
    )
    .iter()
    .map(Vec::len)
    .collect::<Vec<_>>();
    assert_eq!(plan, vec![4_096, 4_096, 61]);

    let first = runtime
        .realize_activation_spawn_window(1_000)
        .expect("first window");
    assert_eq!((first.realized, first.remaining), (4_096, 4_157));
    assert_eq!(runtime.spawn_point_creatures().len(), 4_096);
    assert_eq!(runtime.carrier.spawns[65].cells.len(), 1);
    // Nothing respawns or realizes outside the plan while the activation runs.
    assert_eq!(
        runtime.drive_spawn_respawns(1_000, reference_view),
        Ok(SpawnDriveReport::default())
    );
    let same = runtime
        .realize_activation_spawn_window(spawn::OWNER_WINDOW_MICROS - 1)
        .expect("same owner window");
    assert_eq!((same.realized, same.remaining), (0, 4_157));
    assert_eq!(runtime.spawn_point_creatures().len(), 4_096);

    let second = runtime
        .realize_activation_spawn_window(spawn::OWNER_WINDOW_MICROS)
        .expect("second window");
    assert_eq!((second.realized, second.remaining), (4_096, 61));
    assert_eq!(runtime.carrier.spawns[65].cells.len(), 63);
    let third = runtime
        .realize_activation_spawn_window(2 * spawn::OWNER_WINDOW_MICROS)
        .expect("last window");
    assert_eq!(
        (third.realized, third.realized_total, third.remaining),
        (61, 8_253, 0)
    );
    assert!(runtime.spawn_activation.is_none());
    let points = runtime.spawn_point_creatures();
    assert_eq!(points.len(), 8_253);
    assert!(points.iter().all(|(_, _, live)| live.is_some()));
    assert!(points.windows(2).all(|pair| pair[0].0 < pair[1].0));
    assert_eq!(
        runtime.realize_activation_spawn_window(3 * spawn::OWNER_WINDOW_MICROS),
        Err(CarrierError::PlanConflict)
    );
}

/// A refusal in a later activation window realizes nothing: the earlier windows'
/// creatures are removed, other actors admitted between windows stay, and a new
/// activation may begin.
#[test]
fn a_refused_activation_window_realizes_nothing() {
    // 65 sources of 64 points: windows of 4,096 and 64.
    let capacity = 4_160 + SPAWN_ACTIVATION_PLAYER_HEADROOM;
    let mut runtime = runtime(capacity);
    assert_eq!(
        runtime.begin_spawn_definitions((0..65).map(|_| row_definition(0, 64)).collect()),
        Ok(4_160)
    );
    let first = runtime.realize_activation_spawn_window(0).expect("first");
    assert_eq!(first.realized, 4_096);
    let early = runtime.spawn_point_creatures()[0].2.expect("realized");
    // Other owner work takes slots between windows: the second window admits 63 of its 64
    // points and then refuses.
    let others = (0..runtime.carrier.free_slot_count() - 63)
        .map(|_| {
            runtime
                .carrier
                .admit_creature(&runtime.continuity, ActorState(0), "test:other", 20)
                .expect("other creature")
        })
        .collect::<Vec<_>>();
    assert_eq!(others.len(), SPAWN_ACTIVATION_PLAYER_HEADROOM + 1);
    assert_eq!(
        runtime.realize_activation_spawn_window(spawn::OWNER_WINDOW_MICROS),
        Err(CarrierError::CapacityExceeded)
    );
    assert!(runtime.carrier.spawns.is_empty() && runtime.spawn_activation.is_none());
    assert!(!runtime.carrier.creature_alive(&runtime.continuity, early));
    for other in &others {
        assert!(
            runtime
                .carrier
                .creature_alive(&runtime.continuity, ExactActorRef(*other))
        );
    }
    assert_eq!(runtime.carrier.occupied.len(), others.len());
    assert_eq!(runtime.carrier.free_slot_count(), capacity - others.len());
    assert_eq!(
        runtime.realize_spawn_definitions(vec![row_definition(0, 64)]),
        Ok(64)
    );
}

/// SPAWN-CEILINGS-1 (Codex 4242245064): an activation leaves
/// [`SPAWN_ACTIVATION_PLAYER_HEADROOM`] slots of the pre-production carrier free, so players
/// are admitted after the largest accepted activation; filling the carrier with
/// `CREATUREAI0-RL-01` points refuses with nothing kept.
#[test]
fn activation_keeps_player_headroom_in_the_pre_production_carrier() {
    // The pre-production carrier capacity (`PREPRODUCTION_FIRST_SLICE_ACTOR_CAPACITY`,
    // 131,072) equals `CREATUREAI0-RL-01`; this file is also built into crates without
    // the node configuration, so the test names the equal ceiling.
    let capacity = CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX;
    assert_eq!(
        spawn_activation_player_headroom(capacity),
        SPAWN_ACTIVATION_PLAYER_HEADROOM
    );
    let points = |total: usize| {
        let mut definitions = (0..total / 64)
            .map(|index| row_definition(i32::try_from(index % 4).expect("small") * 64, 64))
            .collect::<Vec<_>>();
        if !total.is_multiple_of(64) {
            definitions.push(row_definition(0, total % 64));
        }
        definitions
    };
    let mut runtime = runtime(capacity);
    let largest = capacity - SPAWN_ACTIVATION_PLAYER_HEADROOM;
    for refused in [capacity, largest + 1] {
        assert_eq!(
            runtime.begin_spawn_definitions(points(refused)),
            Err(CarrierError::CapacityExceeded)
        );
        assert!(runtime.spawn_activation.is_none() && runtime.carrier.spawns.is_empty());
    }
    assert_eq!(
        realize_in_owner_windows(&mut runtime, points(largest)).0,
        largest
    );
    for raw in 0..u64::try_from(SPAWN_ACTIVATION_PLAYER_HEADROOM).expect("small") {
        let reservation = runtime
            .reserve_fresh_session(session(1_000 + raw))
            .expect("player headroom");
        runtime
            .commit_fresh_session(reservation)
            .expect("player commit");
    }
    assert_eq!(runtime.carrier.free_slot_count(), 0);
}

/// SPAWN-CEILINGS-1 (Codex 4242381779): `CREATUREAI0-RL-04` is the creature admission
/// ceiling. The pre-production carrier is smaller, so it never binds there; a carrier with
/// one slot more than the ceiling admits 262,144 creatures, refuses the next one with
/// nothing written and still admits a player into the free slot.
#[test]
fn creature_admission_refuses_past_the_creature_actor_ceiling() {
    const {
        assert!(
            CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX < CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX
        );
    }
    let mut runtime = runtime(CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX + 1);
    for _ in 0..CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX {
        runtime
            .carrier
            .admit_creature(&runtime.continuity, ActorState(0), "test:creature", 20)
            .expect("below the ceiling");
    }
    assert_eq!(
        runtime
            .carrier
            .admit_creature(&runtime.continuity, ActorState(0), "test:creature", 20),
        Err(CarrierError::CapacityExceeded)
    );
    assert_eq!(runtime.carrier.free_slot_count(), 1);
    let reservation = runtime
        .reserve_fresh_session(session(1))
        .expect("the free slot admits a player");
    runtime
        .commit_fresh_session(reservation)
        .expect("player commit");
    assert_eq!(runtime.carrier.free_slot_count(), 0);
}

/// SPAWN-CEILINGS-1 (Codex 4242094686): the full spawn scan runs once per Channel owner
/// window, not once per drive. The owner drives once per connected session; a creature
/// that dies after a window's first drive is observed only at the next window's first
/// drive, however many sessions drive meanwhile, and an occurrence due inside a window
/// still runs at its exact deadline.
#[test]
fn the_spawn_scan_runs_once_per_owner_window_however_many_sessions_drive() {
    let mut runtime = spawned_runtime(den_definition());
    let point = runtime.spawn_point_creatures()[0].0;
    assert_eq!(
        runtime.drive_spawn_respawns(0, reference_view),
        Ok(SpawnDriveReport::default())
    );
    let dead = den_creature(&runtime).expect("live rat");
    kill_creature(&mut runtime, dead);
    for session_drive in 1..=256 {
        assert_eq!(
            runtime.drive_spawn_respawns(session_drive, reference_view),
            Ok(SpawnDriveReport::default())
        );
    }
    assert_eq!(
        runtime.pending_respawn(point),
        None,
        "no rescan in the window"
    );
    let observed_at = spawn::OWNER_WINDOW_MICROS + 123;
    runtime
        .drive_spawn_respawns(observed_at, reference_view)
        .expect("next window");
    let due = observed_at + SPAWN_DELAY;
    assert_eq!(
        runtime.pending_respawn(point).map(|p| p.due_micros),
        Some(due)
    );
    // The window holding the deadline: its drives before the deadline run nothing, the
    // drive at the deadline warns.
    let window_start = due - 123;
    for now in window_start..due {
        assert_eq!(
            runtime.drive_spawn_respawns(now, reference_view),
            Ok(SpawnDriveReport::default())
        );
    }
    let warned = runtime
        .drive_spawn_respawns(due, reference_view)
        .expect("deadline");
    assert_eq!(warned.warnings, vec![DEN]);
    assert_eq!(
        runtime.drive_spawn_respawns(due + 1, reference_view),
        Ok(SpawnDriveReport::default())
    );
}

/// SPAWN-CEILINGS-1 (Codex 4242270322): when many respawns fall due at once, each owner
/// window runs at most [`SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX`] steps however many drives it
/// gets; the rest stay pending, run first in the next windows, and every point is admitted
/// again in the end.
#[test]
fn simultaneous_respawns_run_a_bounded_number_of_steps_per_owner_window() {
    let sources = 4;
    let total = sources * CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX;
    let mut runtime = runtime(2 * total);
    let definitions = (0..sources)
        .map(|source| {
            row_definition(
                i32::try_from(source * CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX).expect("small"),
                CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX,
            )
        })
        .collect();
    assert_eq!(runtime.realize_spawn_definitions(definitions), Ok(total));
    let points = runtime.spawn_point_creatures();
    assert_eq!(points.len(), total);
    for (_, _, live) in &points {
        kill_creature(&mut runtime, live.expect("realized creature"));
    }
    // One drive observes every death: all occurrences fall due at the same deadline.
    assert_eq!(
        runtime.drive_spawn_respawns(0, reference_view),
        Ok(SpawnDriveReport::default())
    );
    assert!(points.iter().all(|(point, _, _)| {
        runtime.pending_respawn(*point).map(|p| p.due_micros) == Some(SPAWN_DELAY)
    }));
    let steps = |report: &SpawnDriveReport| {
        report.warnings.len()
            + report.admitted.len()
            + report.blocked
            + report.retried
            + report.skipped
    };
    let first = runtime
        .drive_spawn_respawns(SPAWN_DELAY, reference_view)
        .expect("deadline");
    assert_eq!(first.warnings.len(), SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX);
    // Later drives of the same window (one per session) run nothing more.
    for session_drive in 1..=64 {
        assert_eq!(
            runtime.drive_spawn_respawns(SPAWN_DELAY + session_drive, reference_view),
            Ok(SpawnDriveReport::default())
        );
    }
    let mut admitted = 0;
    let mut warned = first.warnings.len();
    let mut window_start = SPAWN_DELAY - SPAWN_DELAY % spawn::OWNER_WINDOW_MICROS;
    let mut windows = 0;
    while admitted < total {
        window_start += spawn::OWNER_WINDOW_MICROS;
        windows += 1;
        assert!(windows < 10_000, "every respawn completes");
        let mut window_steps = 0;
        for offset in [0, 1, spawn::OWNER_WINDOW_MICROS - 1] {
            let report = runtime
                .drive_spawn_respawns(window_start + offset, reference_view)
                .expect("drive");
            assert_eq!(report.blocked + report.retried + report.skipped, 0);
            window_steps += steps(&report);
            warned += report.warnings.len();
            admitted += report.admitted.len();
        }
        assert!(window_steps <= SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX);
    }
    assert_eq!((warned, admitted), (total, total));
    for (point, cell, live) in runtime.spawn_point_creatures() {
        assert_eq!(runtime.pending_respawn(point), None);
        let fresh = live.expect("respawned creature");
        assert!(runtime.carrier.creature_alive(&runtime.continuity, fresh));
        assert!(!points.iter().any(|(_, _, dead)| *dead == Some(fresh)));
        assert_eq!(
            points
                .iter()
                .find(|(dead_point, _, _)| *dead_point == point)
                .map(|(_, dead_cell, _)| *dead_cell),
            Some(cell)
        );
    }
    // The dead creatures are gone: the occupied index holds exactly the respawned ones.
    assert_eq!(runtime.carrier.occupied.len(), total);
}

/// SPAWN-CEILINGS-1 (Codex 4243397472): with all 131,072 creatures of a Channel dead at once,
/// each owner window reads at most [`SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX`] points and
/// [`SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX`] deadline entries, however many drives it gets:
/// the gone scan observes the deaths 4,096 per window from its cursor, and once the whole
/// backlog is due each window runs its 32 steps by deadline then point without collecting
/// or sorting the backlog. The deadline index equals the schedule after every window.
#[test]
fn a_131_072_death_backlog_reads_bounded_spawn_records_per_owner_window() {
    let total = CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX;
    let per_source = CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX;
    let mut runtime = runtime(total + SPAWN_ACTIVATION_PLAYER_HEADROOM);
    let definitions = (0..total / per_source)
        .map(|_| row_definition(0, per_source))
        .collect();
    let (realized, activation_windows) = realize_in_owner_windows(&mut runtime, definitions);
    assert_eq!(realized, total);
    let points = runtime.spawn_point_creatures();
    for (_, _, live) in &points {
        kill_creature(&mut runtime, live.expect("realized creature"));
    }
    let window_reads = SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX + SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX;
    let window_micros = spawn::OWNER_WINDOW_MICROS;
    let start = u64::try_from(activation_windows).expect("small") * window_micros;
    let drive_window = |runtime: &mut ChannelRuntimeV1, now: u64| {
        let report = runtime
            .drive_spawn_respawns(now, reference_view)
            .expect("drive");
        // Another session's drive in the same window runs nothing more.
        assert_eq!(
            runtime.drive_spawn_respawns(now + 1, reference_view),
            Ok(SpawnDriveReport::default())
        );
        let (visits, consistent) = runtime.take_spawn_drive_visits();
        assert!(consistent, "the deadline index equals the schedule");
        assert!(
            visits <= window_reads,
            "{visits} records read in one window"
        );
        report
    };
    // The gone scan: 4,096 deaths observed per window, 32 windows for the whole Channel.
    let scan_windows = total / SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX;
    assert_eq!(scan_windows, 32);
    let _ = runtime.take_spawn_drive_visits();
    for window in 0..scan_windows {
        let now = start + u64::try_from(window).expect("small") * window_micros;
        assert_eq!(drive_window(&mut runtime, now), SpawnDriveReport::default());
        assert_eq!(
            runtime.spawn_schedule.len(),
            (window + 1) * SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX
        );
    }
    for (index, (point, _, _)) in points.iter().enumerate() {
        let window = u64::try_from(index / SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX).expect("small");
        assert_eq!(
            runtime
                .pending_respawn(*point)
                .map(|pending| pending.due_micros),
            Some(start + window * window_micros + SPAWN_DELAY)
        );
    }
    // The whole backlog is due: each window warns 32 points, oldest deadline first.
    let mut now = start + u64::try_from(scan_windows).expect("small") * window_micros + SPAWN_DELAY;
    for window in 0..64 {
        let report = drive_window(&mut runtime, now);
        assert_eq!(report.warnings.len(), SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX);
        assert!(report.admitted.is_empty());
        now += window_micros;
        let warned = (window + 1) * SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX;
        assert_eq!(
            runtime
                .pending_respawn(points[warned - 1].0)
                .map(|pending| pending.step),
            Some(spawn::RespawnStep::Admit)
        );
        assert_eq!(
            runtime
                .pending_respawn(points[warned].0)
                .map(|pending| pending.step),
            Some(spawn::RespawnStep::Due)
        );
    }
    assert_eq!(runtime.spawn_schedule.len(), total);
}

/// The single-source path enforces `CREATUREAI0-RL-01` against the points already
/// realized: up to the ceiling passes, one past it refuses with no admission.
#[test]
fn single_spawn_source_respects_the_channel_point_ceiling() {
    let (continuity, mut carrier) = carrier(4);
    let context = spawn_position_context(&continuity);
    let per_source = CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX;
    // Bookkeeping-only sources stand for already realized points; their cells name no
    // creature, so no slot is spent on them.
    let filler = |source: u16, points: usize| SpawnRealization {
        source: SpawnSourceId(source),
        definition: row_definition(0, points),
        cells: vec![
            SpawnCellState {
                live: None,
                attempt: 0,
                successor: 0,
            };
            points
        ],
    };
    let full_sources = CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX / per_source;
    carrier.spawns = (0..full_sources - 1)
        .map(|source| filler(u16::try_from(source).expect("small"), per_source))
        .collect();
    carrier.spawns.push(filler(
        u16::try_from(full_sources - 1).expect("small"),
        per_source - 1,
    ));
    let next = SpawnSourceId(u16::try_from(full_sources).expect("small"));
    carrier
        .realize_spawn(&continuity, next, row_definition(0, 1), context)
        .expect("exactly CREATUREAI0-RL-01 points");
    let slots_before = carrier.slots.clone();
    let spawns_before = carrier.spawns.len();
    assert_eq!(
        carrier.realize_spawn(
            &continuity,
            SpawnSourceId(next.0 + 1),
            row_definition(1, 1),
            context
        ),
        Err(CarrierError::CapacityExceeded)
    );
    assert_eq!(carrier.slots, slots_before);
    assert_eq!(carrier.spawns.len(), spawns_before);
}

/// Sources realized out of order still land in ascending order, and every lookup finds
/// them by binary search.
#[test]
fn spawn_sources_stay_ordered_for_lookup() {
    let (continuity, mut carrier) = carrier(8);
    let context = spawn_position_context(&continuity);
    for (source, x) in [(7_u16, 0), (2, 1), (40_000, 2), (3, 3)] {
        carrier
            .realize_spawn(
                &continuity,
                SpawnSourceId(source),
                row_definition(x, 1),
                context,
            )
            .expect("realizes");
    }
    let order = carrier
        .spawns
        .iter()
        .map(|spawn| spawn.source.0)
        .collect::<Vec<_>>();
    assert_eq!(order, [2, 3, 7, 40_000]);
    for source in [2_u16, 3, 7, 40_000] {
        assert!(carrier.spawn_cell_state(SpawnSourceId(source), 0).is_ok());
    }
    assert!(matches!(
        carrier.spawn_cell_state(SpawnSourceId(4), 0),
        Err(CarrierError::UnknownSpawnSource)
    ));
    assert_eq!(
        carrier.realize_spawn(&continuity, SpawnSourceId(7), row_definition(4, 1), context),
        Err(CarrierError::DuplicateSpawnSource)
    );
}

/// Removal no longer scans the spawn cells, so a cell can keep naming a removed creature.
/// That stale reference never reads as the slot's later occupant, and the respawn's
/// best-effort removal of it never removes that occupant.
#[test]
fn a_removed_spawned_creature_reads_as_gone_after_its_slot_is_reused() {
    let (continuity, mut carrier) = carrier(1);
    let context = spawn_position_context(&continuity);
    carrier
        .realize_spawn(&continuity, SpawnSourceId(0), row_definition(0, 1), context)
        .expect("realizes");
    let stale = carrier
        .spawn_cell_state(SpawnSourceId(0), 0)
        .expect("cell tracked")
        .live
        .expect("live rat");
    carrier.remove(&continuity, stale.0).expect("removed");
    let occupant = ExactActorRef(
        carrier
            .admit_creature(&continuity, ActorState(0), "oteryn:creature/rat", 20)
            .expect("the slot is reused"),
    );
    assert_eq!(occupant.0.actor_local_id, stale.0.actor_local_id);
    assert_eq!(
        carrier
            .spawn_cell_state(SpawnSourceId(0), 0)
            .expect("cell tracked")
            .live,
        Some(stale)
    );
    assert!(!carrier.creature_alive(&continuity, stale));
    assert!(carrier.creature_alive(&continuity, occupant));
    assert!(matches!(
        carrier.remove(&continuity, stale.0),
        Err(CarrierError::StaleActorGeneration)
    ));
    assert!(carrier.creature_alive(&continuity, occupant));
}

#[test]
fn spawn_population_and_placement_cells_mismatch_rejected_both_directions() {
    // P2 (Codex review, PR #1193): §4.3 "one declared cell per creature" -- fewer cells
    // than the population must not clamp down, and extra unused cells must not be
    // tolerated either, since `realize_spawn`/`resolve_respawn_timer` would otherwise be
    // able to respawn more creatures than the declared population.
    assert_eq!(
        SpawnDefinition::new(
            vec![(0, 0, 0)],
            2,
            60_000_000,
            5_000_000,
            "oteryn:creature/rat".to_string(),
            20,
        ),
        Err(CarrierError::InvalidSpawnDefinition),
        "population above the declared cell count is rejected"
    );
    assert_eq!(
        SpawnDefinition::new(
            vec![(0, 0, 0), (1, 0, 0)],
            1,
            60_000_000,
            5_000_000,
            "oteryn:creature/rat".to_string(),
            20,
        ),
        Err(CarrierError::InvalidSpawnDefinition),
        "extra, unused cells beyond the population are rejected"
    );
    assert!(
        SpawnDefinition::new(
            vec![(0, 0, 0), (1, 0, 0)],
            2,
            60_000_000,
            5_000_000,
            "oteryn:creature/rat".to_string(),
            20,
        )
        .is_ok(),
        "an exact population/cell-count match is accepted"
    );
}

/// Kills the live creature at `cell_index` of `source` (a direct lethal commit through the
/// exact-actor commit path, mirroring `CombatDeathFixture::strike`), so its cell's respawn
/// can be exercised.
fn kill_spawn_cell(
    continuity: &NamespaceContinuityGuard,
    carrier: &mut ChannelActorCarrier,
    source: SpawnSourceId,
    cell_index: usize,
) -> ExactActorRef {
    let dead = carrier
        .spawn_cell_state(source, cell_index)
        .expect("cell tracked")
        .live
        .expect("cell has a live rat to kill");
    carrier
        .current_owner_exact_commit(continuity)
        .commit_damage(
            dead,
            OwnerDamageCommand {
                target: b"oteryn:creature/rat",
                occurrence: b"kill",
                binding: b"kill\0respawn-test.v1",
                damage: 20,
            },
        )
        .expect("lethal strike");
    dead
}

/// VIS-3 (Codex 4178196302): the occupied index names exactly the occupied slots after
/// admissions, removals, slot reuse, a spawn and a respawn, and holds no more than them.
#[test]
fn the_occupied_index_is_exactly_the_occupied_slots() {
    let by_scan = |carrier: &ChannelActorCarrier| -> Vec<u32> {
        (0_u32..)
            .zip(carrier.slots.iter())
            .filter(|(_, slot)| {
                matches!(
                    slot,
                    Slot::Occupied { .. }
                        | Slot::CreatureOccupied { .. }
                        | Slot::CreatureReserved { .. }
                )
            })
            .map(|(index, _)| index)
            .collect()
    };
    let indexed = |carrier: &ChannelActorCarrier| -> Vec<u32> {
        let mut occupied = carrier.occupied.clone();
        occupied.sort_unstable();
        occupied
    };
    let (continuity, mut carrier) = carrier(16);
    assert!(carrier.occupied.is_empty());
    assert!(carrier.occupied.capacity() >= 16);
    let first = carrier.admit(&continuity, ActorState(1)).expect("admit");
    let second = carrier.admit(&continuity, ActorState(2)).expect("admit");
    carrier.admit(&continuity, ActorState(3)).expect("admit");
    assert_eq!(indexed(&carrier), [0, 1, 2]);
    carrier.remove(&continuity, second).expect("remove");
    assert_eq!(indexed(&carrier), by_scan(&carrier));
    assert_eq!(indexed(&carrier), [0, 2]);
    // The freed slot is reused, and listed once.
    carrier.admit(&continuity, ActorState(4)).expect("admit");
    assert_eq!(indexed(&carrier), [0, 1, 2]);
    carrier.remove(&continuity, first).expect("remove");
    let context = spawn_position_context(&continuity);
    carrier
        .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
        .expect("D116 realizes");
    assert_eq!(indexed(&carrier), by_scan(&carrier));
    kill_spawn_cell(&continuity, &mut carrier, SpawnSourceId(1), 0);
    carrier
        .resolve_respawn_timer(&continuity, SpawnSourceId(1), 0, context)
        .expect("resolves");
    assert_eq!(indexed(&carrier), by_scan(&carrier));
    assert!(carrier.occupied.len() < carrier.slots.len());
}

#[test]
fn respawn_admits_a_fresh_generation_when_the_cell_is_free() {
    let (continuity, mut carrier) = carrier(4);
    let context = spawn_position_context(&continuity);
    carrier
        .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
        .expect("D116 realizes");
    let dead = kill_spawn_cell(&continuity, &mut carrier, SpawnSourceId(1), 0);

    let resolution = carrier
        .resolve_respawn_timer(&continuity, SpawnSourceId(1), 0, context)
        .expect("resolves");
    assert!(
        matches!(resolution, RespawnResolution::Admitted(_)),
        "expected an admission when the cell is free: {resolution:?}"
    );
    let RespawnResolution::Admitted(new_actor) = resolution else {
        unreachable!("checked above");
    };
    assert_ne!(
        new_actor.0.actor_local_generation, dead.0.actor_local_generation,
        "D52: the dead actor's generation is never reused"
    );
    assert!(
        carrier
            .current_owner_exact_lookup(&continuity)
            .contains(new_actor)
    );
    assert_eq!(
        carrier
            .spawn_cell_state(SpawnSourceId(1), 0)
            .expect("cell tracked")
            .live,
        Some(new_actor)
    );
}

#[test]
fn respawn_with_an_invalid_context_mutates_nothing_and_returns_err() {
    // P1 (Codex review, PR #1193): the context is validated before the dead actor is
    // removed or a replacement admitted, so a rejected respawn never leaves the cell
    // pointing at a removed actor while an unpositioned replacement stays admitted.
    let (continuity, mut carrier) = carrier(4);
    let context = spawn_position_context(&continuity);
    carrier
        .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
        .expect("D116 realizes");
    let dead = kill_spawn_cell(&continuity, &mut carrier, SpawnSourceId(1), 0);

    let mut invalid_context = context;
    invalid_context.coordinate_frame_marker = 0;
    let slots_before = carrier.slots.clone();
    let free_head_before = carrier.free_head;
    let spawns_before = carrier.spawns.clone();
    assert_eq!(
        carrier.resolve_respawn_timer(&continuity, SpawnSourceId(1), 0, invalid_context),
        Err(CarrierError::InvalidPreProductionPositionContext)
    );
    assert_eq!(carrier.slots, slots_before);
    assert_eq!(carrier.free_head, free_head_before);
    assert_eq!(carrier.spawns, spawns_before);
    assert_eq!(
        carrier
            .spawn_cell_state(SpawnSourceId(1), 0)
            .expect("cell tracked")
            .live,
        Some(dead),
        "the cell still names the original dead actor"
    );

    let resolution = carrier
        .resolve_respawn_timer(&continuity, SpawnSourceId(1), 0, context)
        .expect("a valid context still resolves afterward");
    assert!(matches!(resolution, RespawnResolution::Admitted(_)));
}

#[test]
fn respawn_postpones_three_times_then_terminates_skipped_and_schedules_a_successor() {
    let (continuity, mut carrier) = carrier(6);
    let context = spawn_position_context(&continuity);
    carrier
        .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
        .expect("D116 realizes");
    kill_spawn_cell(&continuity, &mut carrier, SpawnSourceId(1), 0);
    // An unrelated live creature occupies cell 0's exact position, blocking respawn.
    let blocker = carrier
        .admit_creature(&continuity, ActorState(9), "oteryn:creature/blocker", 20)
        .expect("blocker admits");
    carrier
        .initialize_position(
            &continuity,
            blocker,
            context,
            LocalPosition {
                x: 1,
                y: 0,
                floor: 7,
            },
        )
        .expect("blocker occupies cell 0");

    for expected_attempt in 1..=AI01_SPAWN_OCCUPANCY_RETRIES_MAX {
        assert_eq!(
            carrier.resolve_respawn_timer(&continuity, SpawnSourceId(1), 0, context),
            Ok(RespawnResolution::Postponed {
                next_attempt: expected_attempt
            })
        );
    }
    assert_eq!(
        carrier.resolve_respawn_timer(&continuity, SpawnSourceId(1), 0, context),
        Ok(RespawnResolution::Skipped { next_successor: 1 }),
        "the third retry still finds the cell occupied: terminal SKIPPED"
    );
    let state = carrier
        .spawn_cell_state(SpawnSourceId(1), 0)
        .expect("cell tracked");
    assert_eq!(
        state.attempt, 0,
        "a fresh successor chain starts at attempt 0"
    );
    assert_eq!(state.successor, 1);

    // The blocker leaves; the successor chain's own respawn now succeeds.
    carrier
        .remove(&continuity, blocker)
        .expect("blocker leaves");
    let resolution = carrier
        .resolve_respawn_timer(&continuity, SpawnSourceId(1), 0, context)
        .expect("resolves once the cell is free");
    assert!(matches!(resolution, RespawnResolution::Admitted(_)));
}

#[test]
fn respawn_timer_lane_binding_items_and_deadline_state_never_collapses_distinct_cells() {
    // AI-1 binding items applied to respawn occurrences (carried over per the coordinator's
    // instruction): (a) bounded replay evidence via `SpawnCellState.attempt`/`.successor`,
    // which `resolve_respawn_timer` alone advances, so a `RespawnOccurrence` tuple can never
    // repeat; (b) `schedule` itself only accepts a fence-issued `RuntimeWorkStamp` (AI-1's
    // own enforcement, exercised here exactly as its own tests do).
    let (continuity, mut carrier) = carrier(6);
    let context = spawn_position_context(&continuity);
    carrier
        .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
        .expect("D116 realizes");
    let dead_0 = kill_spawn_cell(&continuity, &mut carrier, SpawnSourceId(1), 0);
    let dead_1 = kill_spawn_cell(&continuity, &mut carrier, SpawnSourceId(1), 1);

    use oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1;
    use oteryn_gameplay_foundation::foundation::owner_timer::{
        CatchUpPolicy, FamilyPolicy, OwnerTimerError, OwnerTimerLane, SemanticTimeMicros,
        VirtualOwnerClock,
    };

    let scope = RuntimeScopeRefV1::channel(continuity.world_id, continuity.channel_id);
    let generation = continuity.current_generation;
    let mut lane: OwnerTimerLane<RespawnFamily, RespawnOccurrence> =
        OwnerTimerLane::for_generation(
            scope,
            generation,
            [(
                RespawnFamily::Respawn,
                FamilyPolicy {
                    max_pending: AI01_PENDING_RESPAWN_TIMERS_PER_DEAD_ACTOR,
                    catch_up: CatchUpPolicy::DeadlineState,
                },
            )],
        )
        .expect("cap within its registered maximum");
    let mut owner_fence =
        oteryn_gameplay_foundation::foundation::scoped_fence_fixture(scope, generation);
    let due = SemanticTimeMicros::from_micros(60_000_000);

    for (dead, index) in [(dead_0, 0_u8), (dead_1, 1_u8)] {
        let ordinal = owner_fence
            .accept_input(generation)
            .expect("issue a live ordinal");
        let stamp = owner_fence.stamp(ordinal);
        lane.schedule(
            &owner_fence,
            stamp,
            RespawnFamily::Respawn,
            RespawnOccurrence {
                source: SpawnSourceId(1),
                cell_index: index,
                dead_actor: dead,
                successor: 0,
                attempt: 1,
            },
            Some(dead),
            due,
        )
        .expect("schedule");
    }
    // Binding item (a): the exact same occurrence can never be scheduled twice.
    let ordinal = owner_fence
        .accept_input(generation)
        .expect("issue a live ordinal");
    let stamp = owner_fence.stamp(ordinal);
    assert_eq!(
        lane.schedule(
            &owner_fence,
            stamp,
            RespawnFamily::Respawn,
            RespawnOccurrence {
                source: SpawnSourceId(1),
                cell_index: 0,
                dead_actor: dead_0,
                successor: 0,
                attempt: 1,
            },
            Some(dead_0),
            due,
        ),
        Err(OwnerTimerError::DuplicateOccurrence)
    );

    let clock = VirtualOwnerClock::new(due);
    let fired = lane.drain_due(&clock, &owner_fence, |_| true);
    // DeadlineState (distinct from AI-1's SkipToLatest): both distinct cells' occurrences
    // fire, never collapsed into one.
    assert_eq!(fired.len(), 2);
    let occurrences: std::collections::HashSet<_> = fired
        .iter()
        .map(|timer| timer.occurrence.cell_index)
        .collect();
    assert_eq!(occurrences, std::collections::HashSet::from([0, 1]));

    for fired_timer in &fired {
        let resolution = carrier
            .resolve_respawn_timer(
                &continuity,
                fired_timer.occurrence.source,
                fired_timer.occurrence.cell_index as usize,
                context,
            )
            .expect("resolves");
        assert!(matches!(resolution, RespawnResolution::Admitted(_)));
    }
}

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
