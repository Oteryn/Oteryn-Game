#[test]
fn activity_runtime_uses_private_scope_with_shared_native_owner() {
    let mut channel = runtime(4);
    let binding = channel.binding();
    let local = *binding.channel_id().expect("Channel fixture").as_bytes();
    let scope = RuntimeScopeRefV1::instance(binding.world_id(), local).expect("UUIDv7 instance");
    let mut instance = ScopeRuntimeV1::from_committed_scope_assignment(
        scope,
        binding.node_id(),
        binding.node_registration_revision(),
        binding.scope_generation().get(),
        binding.source_revision(),
        &format!("runtime-scope-assignment:{}", binding.source_revision()),
        4,
        channel.content_pin().clone(),
    )
    .expect("independently assigned Instance runtime");
    assert_eq!(instance.binding().runtime_scope(), scope);
    assert_eq!(instance.binding().channel_id(), None);
    assert!(instance.owner_fence().is_ok());
    let channel_reservation = channel
        .reserve_fresh_session(session(80))
        .expect("channel slot");
    let channel_actor = channel
        .commit_fresh_session(channel_reservation)
        .expect("channel actor");
    channel
        .initialize_first_entry_position(channel_actor)
        .expect("channel position");
    let instance_reservation = instance
        .reserve_fresh_session(session(81))
        .expect("instance slot");
    let instance_actor = instance
        .commit_fresh_session(instance_reservation)
        .expect("instance actor");
    instance
        .initialize_first_entry_position(instance_actor)
        .expect("instance position");
    assert_eq!(instance_actor.runtime_scope(), scope);
    assert_eq!(
        channel_actor.actor_local_id(),
        instance_actor.actor_local_id()
    );
    assert_eq!(
        channel_actor.actor_local_generation(),
        instance_actor.actor_local_generation()
    );
    assert!(channel.read_actor_position(instance_actor).is_err());
    assert!(instance.read_actor_position(channel_actor).is_err());
    assert_eq!(
        instance
            .positioned_actor_census()
            .expect("instance census")
            .len(),
        1
    );
    assert_eq!(
        channel
            .positioned_actor_census()
            .expect("channel census")
            .len(),
        1
    );
    instance.retire_owner_cycle();
    assert!(instance.owner_fence().is_err());
    assert!(instance.reserve_fresh_session(session(82)).is_err());
    assert!(channel.read_actor_position(channel_actor).is_ok());
}

#[test]
fn activity_runtime_rejects_malformed_scope_and_assignment_before_bootstrap() {
    let channel = runtime(1);
    let binding = channel.binding();
    let invalid = RuntimeScopeRefV1::Instance {
        world_id: binding.world_id(),
        instance_id: [0; 16],
    };
    assert!(
        ScopeRuntimeV1::from_committed_scope_assignment(
            invalid,
            binding.node_id(),
            binding.node_registration_revision(),
            binding.scope_generation().get(),
            binding.source_revision(),
            &format!("runtime-scope-assignment:{}", binding.source_revision()),
            1,
            channel.content_pin().clone(),
        )
        .is_err()
    );
    let scope = RuntimeScopeRefV1::instance(
        binding.world_id(),
        *binding.channel_id().expect("Channel fixture").as_bytes(),
    )
    .expect("UUIDv7 instance");
    assert!(
        ScopeRuntimeV1::from_committed_scope_assignment(
            scope,
            binding.node_id(),
            binding.node_registration_revision(),
            binding.scope_generation().get(),
            binding.source_revision(),
            "runtime-scope-assignment:0",
            1,
            channel.content_pin().clone(),
        )
        .is_err()
    );
}

#[test]
fn composed_runtime_binds_assignment_and_reserves_before_commit() {
    let mut runtime = runtime(1);
    let binding = runtime.binding();
    assert!(binding.matches_committed_assignment(
        binding.world_id(),
        binding.channel_id().expect("Channel fixture"),
        node(22),
        7,
        3,
        11,
        "runtime-scope-assignment:11",
    ));
    assert!(!binding.matches_committed_assignment(
        binding.world_id(),
        binding.channel_id().expect("Channel fixture"),
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
        forge_difficulty: None,
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
        // The derived last-translation revision uses the remaining u64 within that
        // unchanged 184-byte budget; it introduces no independent counter or allocation.
        184,
        "spell candidate fixed-slot footprint must stay inside the D314 budget"
    );
}

fn activity_instance_for(channel: &ScopeRuntimeV1, capacity: usize) -> ScopeRuntimeV1 {
    let binding = channel.binding();
    let scope =
        RuntimeScopeRefV1::instance(binding.world_id(), *session(91).as_bytes()).expect("instance");
    ScopeRuntimeV1::from_committed_scope_assignment(
        scope,
        binding.node_id(),
        binding.node_registration_revision(),
        binding.scope_generation().get(),
        binding.source_revision(),
        &format!("runtime-scope-assignment:{}", binding.source_revision()),
        capacity,
        channel.content_pin().clone(),
    )
    .expect("native instance")
}

fn activity_member(
    channel: &mut ScopeRuntimeV1,
    id: u8,
    x: i32,
) -> activity_transfer::ActivityTransferMember {
    let source_session = session(u64::from(id));
    let reservation = channel
        .reserve_fresh_session(source_session)
        .expect("source reserve");
    let actor = channel
        .commit_fresh_session(reservation)
        .expect("source commit");
    channel
        .initialize_pinned_test_position(actor, MovementLocalPosition { x, y: 0, floor: 0 })
        .expect("source position");
    let character = CharacterId::decode(session(u64::from(id) + 30).as_bytes()).expect("character");
    let source_lease = CharacterLease::new(character, 7).expect("source lease");
    channel
        .bind_attacker_lease(actor, source_session, source_lease)
        .expect("bind source");
    activity_transfer::ActivityTransferMember {
        source_actor: actor,
        source_session,
        source_lease,
        destination_session: session(u64::from(id) + 60),
        destination_lease: CharacterLease::new(character, 8).expect("destination lease"),
        destination_cell: MovementLocalPosition { x, y: 5, floor: 0 },
    }
}

#[test]
fn activity_transfer_freezes_entire_group_and_abort_restores_only_its_reservations() {
    let mut channel = runtime(4);
    let first = activity_member(&mut channel, 10, 1);
    let second = activity_member(&mut channel, 11, 2);
    let unrelated = activity_member(&mut channel, 12, 3);
    let mut instance = activity_instance_for(&channel, 2);
    let mut reservation = channel
        .prepare_activity_transfer(&mut instance, &[first, second])
        .expect("freeze and reserve");
    assert!(channel.read_actor_position(first.source_actor).is_err());
    assert!(
        channel
            .player_control_facts(second.source_actor, second.source_session)
            .is_err()
    );
    assert!(channel.read_actor_position(unrelated.source_actor).is_ok());
    assert_eq!(instance.player_slot_counts(), (0, 2));
    channel
        .abort_activity_transfer(&mut instance, &mut reservation)
        .expect("definite noncommit abort");
    assert_eq!(instance.player_slot_counts(), (0, 0));
    assert!(channel.read_actor_position(first.source_actor).is_ok());
    assert!(channel.read_actor_position(second.source_actor).is_ok());
    assert!(
        channel
            .abort_activity_transfer(&mut instance, &mut reservation)
            .is_err()
    );
}

#[test]
fn activity_transfer_moves_physical_payload_and_retires_old_writer_without_cloning() {
    let mut channel = runtime(3);
    let first = activity_member(&mut channel, 13, 1);
    let unrelated = activity_member(&mut channel, 14, 2);
    let source_index = channel
        .carrier
        .player_slot_index(
            &channel.continuity,
            first.source_actor.0,
            first.source_session,
        )
        .expect("source slot");
    if let Slot::Occupied {
        actor,
        spell_combat,
        ..
    } = &mut channel.carrier.slots[source_index]
    {
        *actor = ActorState(777);
        spell_combat.paralysed = true;
    }
    let mut instance = activity_instance_for(&channel, 1);
    let mut reservation = channel
        .prepare_activity_transfer(&mut instance, &[first])
        .expect("reserve");
    let moved = channel
        .commit_activity_transfer(&mut instance, &mut reservation)
        .expect("committed group install");
    assert_eq!(moved.len(), 1);
    let target = moved[0].destination_actor;
    assert_eq!(target.runtime_scope(), instance.binding().runtime_scope());
    assert!(channel.read_actor_position(first.source_actor).is_err());
    assert!(
        instance
            .player_control_facts(target, first.source_session)
            .is_err()
    );
    assert_eq!(
        instance.bound_attacker_lease(target, first.destination_session),
        Ok(first.destination_lease)
    );
    let target_index = instance
        .carrier
        .player_slot_index(&instance.continuity, target.0, first.destination_session)
        .expect("target slot");
    assert!(
        matches!(&instance.carrier.slots[target_index], Slot::Occupied { actor: ActorState(777), spell_combat, .. } if spell_combat.paralysed)
    );
    assert!(channel.read_actor_position(unrelated.source_actor).is_ok());
    assert_eq!(channel.player_slot_counts(), (1, 0));
    assert_eq!(instance.player_slot_counts(), (1, 0));
    assert!(
        channel
            .commit_activity_transfer(&mut instance, &mut reservation)
            .is_err()
    );
    assert_eq!(instance.player_slot_counts(), (1, 0));
}

#[test]
fn activity_transfer_capacity_or_lease_failure_never_freezes_a_subset() {
    let mut channel = runtime(3);
    let first = activity_member(&mut channel, 15, 1);
    let second = activity_member(&mut channel, 16, 2);
    let mut instance = activity_instance_for(&channel, 1);
    assert!(
        channel
            .prepare_activity_transfer(&mut instance, &[first, second])
            .is_err()
    );
    assert_eq!(instance.player_slot_counts(), (0, 0));
    assert!(channel.read_actor_position(first.source_actor).is_ok());
    assert!(channel.read_actor_position(second.source_actor).is_ok());
    let mut invalid = first;
    invalid.destination_lease = invalid.source_lease;
    assert!(
        channel
            .prepare_activity_transfer(&mut instance, &[invalid])
            .is_err()
    );
    assert_eq!(instance.player_slot_counts(), (0, 0));
}

#[test]
fn activity_transfer_rechecks_destination_owner_after_durable_wait() {
    let mut channel = runtime(1);
    let first = activity_member(&mut channel, 17, 1);
    let mut instance = activity_instance_for(&channel, 1);
    let mut reservation = channel
        .prepare_activity_transfer(&mut instance, &[first])
        .expect("reserve");
    instance.retire_owner_cycle();
    assert!(
        channel
            .commit_activity_transfer(&mut instance, &mut reservation)
            .is_err()
    );
    assert_eq!(channel.player_slot_counts(), (1, 0));
    assert_eq!(instance.player_slot_counts(), (0, 1));
    assert!(
        channel.read_actor_position(first.source_actor).is_err(),
        "uncertain/stale commit remains frozen for recovery"
    );
}

#[test]
fn activity_authored_boss_entry_stacks_one_group_without_guessing_other_cells() {
    let mut channel = runtime(3);
    let first = activity_member(&mut channel, 18, 1);
    let mut second = activity_member(&mut channel, 19, 2);
    second.destination_cell = first.destination_cell;
    let mut instance = activity_instance_for(&channel, 2);
    assert!(
        channel
            .prepare_activity_transfer(&mut instance, &[first, second])
            .is_err()
    );
    assert_eq!(instance.player_slot_counts(), (0, 0));
    let mut reservation = channel
        .prepare_activity_group_entry(&mut instance, &[first, second])
        .expect("authored shared teleport");
    let moved = channel
        .commit_activity_transfer(&mut instance, &mut reservation)
        .expect("one atomic group");
    assert_eq!(instance.player_slot_counts(), (2, 0));
    for member in moved {
        assert_eq!(
            instance
                .read_actor_position(member.destination_actor)
                .expect("position")
                .position(),
            MovementLocalPosition {
                x: first.destination_cell.x,
                y: first.destination_cell.y,
                floor: first.destination_cell.floor
            }
        );
    }
    let third = activity_member(&mut channel, 20, 3);
    assert!(
        channel
            .prepare_activity_group_entry(&mut instance, &[third])
            .is_err(),
        "populated instance is never reused"
    );
    assert!(channel.read_actor_position(third.source_actor).is_ok());
}

#[test]
fn activity_definite_abort_releases_exact_reservations_after_destination_retirement() {
    let mut channel = runtime(1);
    let member = activity_member(&mut channel, 21, 1);
    let mut instance = activity_instance_for(&channel, 1);
    let mut reservation = channel
        .prepare_activity_transfer(&mut instance, &[member])
        .expect("reservation");
    instance.retire_owner_cycle();
    assert!(
        channel
            .commit_activity_transfer(&mut instance, &mut reservation)
            .is_err()
    );
    channel
        .abort_activity_transfer(&mut instance, &mut reservation)
        .expect("known non-commit cleanup");
    assert!(channel.read_actor_position(member.source_actor).is_ok());
    assert_eq!(instance.player_slot_counts(), (0, 0));
    assert!(
        channel
            .abort_activity_transfer(&mut instance, &mut reservation)
            .is_err()
    );
}

#[test]
fn activity_boss_exit_uses_authored_occupied_cell_without_widening_normal_movement() {
    let mut channel = runtime(3);
    let member = activity_member(&mut channel, 22, 1);
    let spectator = activity_member(&mut channel, 23, 2);
    let mut instance = activity_instance_for(&channel, 1);
    let mut entry = channel
        .prepare_activity_group_entry(&mut instance, &[member])
        .expect("entry");
    let moved = channel
        .commit_activity_transfer(&mut instance, &mut entry)
        .expect("entered")[0];
    let returning = activity_transfer::ActivityTransferMember {
        source_actor: moved.destination_actor,
        source_session: moved.destination_session,
        source_lease: moved.destination_lease,
        destination_session: session(110),
        destination_lease: CharacterLease::new(moved.character_id, 9).expect("return lease"),
        destination_cell: channel
            .read_actor_position(spectator.source_actor)
            .expect("occupied exit")
            .position(),
    };
    assert!(
        instance
            .prepare_activity_transfer(&mut channel, &[returning])
            .is_err()
    );
    let mut exit = instance
        .prepare_activity_boss_return(&mut channel, &[returning])
        .expect("authored forced exit");
    let back = instance
        .commit_activity_transfer(&mut channel, &mut exit)
        .expect("returned")[0];
    assert_eq!(
        channel
            .read_actor_position(back.destination_actor)
            .expect("returned position")
            .position(),
        returning.destination_cell
    );
    assert!(channel.read_actor_position(spectator.source_actor).is_ok());
}

#[test]
fn activity_retirement_census_retains_control_lost_players_and_pending_reservations() {
    let mut channel = runtime(3);
    let member = activity_member(&mut channel, 24, 1);
    let mut instance = activity_instance_for(&channel, 2);
    assert_eq!(instance.activity_retirement_empty(), Ok(true));
    let mut held = channel
        .prepare_activity_transfer(&mut instance, &[member])
        .expect("retained reservation");
    assert_eq!(instance.activity_retirement_empty(), Ok(false));
    assert!(
        instance
            .activity_participant_sessions()
            .expect("census")
            .is_empty()
    );
    channel
        .abort_activity_transfer(&mut instance, &mut held)
        .expect("known abort");
    assert_eq!(instance.activity_retirement_empty(), Ok(true));
    let mut held = channel
        .prepare_activity_transfer(&mut instance, &[member])
        .expect("fresh reservation");
    let moved = channel
        .commit_activity_transfer(&mut instance, &mut held)
        .expect("commit")[0];
    instance
        .record_control_loss(
            moved.destination_actor,
            moved.destination_session,
            ControlLossMark {
                epoch: 1,
                grace_deadline: 100,
            },
        )
        .expect("real native loss");
    assert_eq!(
        instance
            .activity_participant_sessions()
            .expect("lost participant"),
        vec![moved.destination_session]
    );
    assert_eq!(instance.activity_retirement_empty(), Ok(false));
    assert!(channel.activity_retirement_empty().is_err());
}
