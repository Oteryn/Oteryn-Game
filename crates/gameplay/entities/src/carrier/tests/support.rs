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
