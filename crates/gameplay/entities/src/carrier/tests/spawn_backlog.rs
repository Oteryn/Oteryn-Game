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
