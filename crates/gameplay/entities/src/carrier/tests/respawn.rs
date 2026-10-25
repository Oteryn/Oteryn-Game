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
