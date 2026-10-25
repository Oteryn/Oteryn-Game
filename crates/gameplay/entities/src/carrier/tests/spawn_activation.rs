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
