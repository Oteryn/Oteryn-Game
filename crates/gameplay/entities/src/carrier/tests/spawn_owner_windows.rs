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
