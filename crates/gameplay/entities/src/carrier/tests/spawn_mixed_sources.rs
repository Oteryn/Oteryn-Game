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
