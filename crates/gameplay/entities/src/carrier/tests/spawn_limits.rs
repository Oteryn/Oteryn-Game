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
