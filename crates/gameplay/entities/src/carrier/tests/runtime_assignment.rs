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
        blocked_by_nearby_players: false,
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
