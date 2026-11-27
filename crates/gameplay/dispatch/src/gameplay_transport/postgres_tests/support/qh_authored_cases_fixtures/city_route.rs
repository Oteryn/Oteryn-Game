use super::{
    AdmittedSession, ComposedFreshAdmission, IRIANA, JIMMY, Position, TestResult, debug, target,
    use_target,
};
use crate::content::{CollisionClass, LogicalCell};
use crate::map::floor::{BundleDoors, Cell, CellBlocker, Ground, resolve_landing, table_of};
use crate::map::overlay::TilePos;
use crate::movement::CardinalStep;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::{
    StepDirection, StepDisposition, decode_step_result_profile, encode_step_intent,
};
use std::collections::{BTreeMap, VecDeque};
use tokio::time::{Duration, Instant, timeout_at};

const LADDER_UP: TilePos = TilePos {
    x: 32833,
    y: 31331,
    floor: -6,
};
const JIMMY_DOOR: TilePos = super::tile(oteryn_gameplay_quests::quest::top_of_the_city::JIMMY_DOOR);
const DOOR_APPROACH: Cell = (32858, 31373, 6);
pub(super) const JIMMY_STOP: Position = Position {
    x: 32858,
    y: 31382,
    floor: JIMMY.floor,
};

// North and south lanes of the first westbound bridge.
const BRIDGE_WEST_NORTH: Cell = (32834, 31314, 6);
const BRIDGE_WEST_SOUTH: Cell = (32834, 31315, 6);
const BRIDGE_SOUTH_EAST: Cell = (32827, 31324, 6);
const UPPER_BRIDGES: [Cell; 6] = [
    (32820, 31346, 5),
    (32823, 31351, 5),
    (32827, 31363, 5),
    (32835, 31360, 5),
    (32837, 31355, 5),
    (32843, 31359, 5),
];
const LAST_BRIDGE: Cell = (32849, 31363, 5);

pub(super) async fn lose_trail(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    deadline: Instant,
) -> TestResult {
    let (x, y, z) = BRIDGE_WEST_SOUTH;
    super::relocate_city(
        owner,
        admitted,
        Position {
            x: u16::try_from(x)?,
            y: u16::try_from(y)?,
            floor: -i8::try_from(z)?,
        },
    )
    .await?;
    step_to(
        owner,
        admitted,
        StepDirection::North,
        BRIDGE_WEST_NORTH,
        deadline,
    )
    .await?;
    // The failed pursuit exercises the gate's USE predicate from its approach tile.
    super::relocate_city(
        owner,
        admitted,
        Position {
            y: JIMMY_DOOR.y - 1,
            ..oteryn_gameplay_quests::quest::top_of_the_city::JIMMY_DOOR
        },
    )
    .await
}

pub(super) async fn roof_to_door(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    deadline: Instant,
) -> TestResult {
    assert_eq!(
        actor_cell(owner, admitted).await?,
        cell(super::city_start())
    );

    // Walk onto Iriana's north-facing stair (Crystal items.xml:14781-14783).
    walk_to(owner, admitted, (32845, 31300, 7), deadline).await?;
    step_to(
        owner,
        admitted,
        StepDirection::East,
        (32846, 31299, 6),
        deadline,
    )
    .await?;

    walk_to(owner, admitted, (32837, 31315, 6), deadline).await?;
    walk_to(owner, admitted, BRIDGE_WEST_SOUTH, deadline).await?;
    walk_to(owner, admitted, BRIDGE_SOUTH_EAST, deadline).await?;
    // The west ladder is disconnected on the pinned map. Use its reachable east ladder.
    walk_to(owner, admitted, (32834, 31331, 6), deadline).await?;
    assert_eq!(
        use_placement(owner, admitted, LADDER_UP, deadline).await?,
        UseDisposition::Committed
    );
    assert_eq!(actor_cell(owner, admitted).await?, (32833, 31332, 5));
    for bridge in UPPER_BRIDGES.into_iter().chain([LAST_BRIDGE]) {
        walk_to(owner, admitted, bridge, deadline).await?;
    }
    walk_to(owner, admitted, (32856, 31362, 5), deadline).await?;
    step_to(
        owner,
        admitted,
        StepDirection::South,
        (32856, 31363, 6),
        deadline,
    )
    .await?;
    walk_to(owner, admitted, DOOR_APPROACH, deadline).await
}

pub(super) async fn use_jimmy_door(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    deadline: Instant,
) -> TestResult<UseDisposition> {
    use_placement(owner, admitted, JIMMY_DOOR, deadline).await
}

pub(super) async fn assert_door_watchdog(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let mut stalled = owner.clone();
    let _held = owner.runtime.lock().await;
    let command = admitted.continuity.next_command_id;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        use_jimmy_door(&mut stalled, admitted, Instant::now()),
    )
    .await
    .map_err(|_| "roof watchdog did not terminate the stalled framed USE")?;
    let Err(error) = result else {
        return Err("a stalled framed USE must hit the roof deadline".into());
    };
    assert_eq!(
        error.to_string(),
        format!("USE_INTENT {command} at {JIMMY_DOOR:?} timed out")
    );
    assert_eq!(admitted.continuity.next_command_id, command);
    Ok(())
}

pub(super) async fn assert_door_does_not_hold_runtime(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("door command window")?;
    let actor = admitted.runtime_actor.ok_or("city actor")?;
    let map = owner.bundle_map().ok_or("city map")?;
    let mut stale = target(owner, JIMMY_DOOR)?;
    // The previous, closed revision cannot toggle the open door.
    stale.expected_revision = stale
        .expected_revision
        .checked_sub(1)
        .ok_or("open door revision")?;
    let reply = super::while_spell_state_is_busy(
        owner,
        owner.use_bundle_door(actor, admitted.game_session_id, map, &stale),
    )
    .await?;
    assert_eq!(reply.disposition, UseDisposition::StaleState);
    Ok(())
}

pub(super) async fn enter_jimmy(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    deadline: Instant,
) -> TestResult<Position> {
    assert_eq!(actor_cell(owner, admitted).await?, DOOR_APPROACH);
    step_to(
        owner,
        admitted,
        StepDirection::South,
        cell(Position {
            x: JIMMY_DOOR.x,
            y: JIMMY_DOOR.y,
            floor: JIMMY_DOOR.floor,
        }),
        deadline,
    )
    .await?;
    walk_to(owner, admitted, cell(JIMMY_STOP), deadline).await?;
    Ok(JIMMY_STOP)
}

pub(super) async fn assert_guest_blocked(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &super::Harness,
    deadline: Instant,
) -> TestResult {
    use crate::foundation::{CharacterId, CharacterLease};
    use crate::gameplay_transport::connection::SessionContinuity;

    // The authorized player's StepOut closed the door. Reopen it from
    // Jimmy's side so the guest faces an open and unoccupied doorway.
    let south = (
        i32::from(JIMMY_DOOR.x),
        i32::from(JIMMY_DOOR.y) + 1,
        -i32::from(JIMMY_DOOR.floor),
    );
    walk_to(owner, admitted, south, deadline).await?;
    assert_eq!(
        use_jimmy_door(owner, admitted, deadline).await?,
        UseDisposition::Committed
    );

    let base = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "city fence")?
        .ok_or("city fence")?
        .item_fence();
    let fence = super::super::private_channel_peers::active_peer(
        h,
        owner,
        base,
        617,
        *admitted.world_id.as_bytes(),
        "City Guest",
    )
    .await?;
    let progress = h
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(progress.tracks().get(super::CITY_STAGE), None);
    let mut guest = AdmittedSession {
        game_session_id: fence.game_session_id,
        item_fence: Some(fence),
        controller: None,
        continuity: SessionContinuity::FRESH,
        ..*admitted
    };
    {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(guest.game_session_id)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                guest.game_session_id,
                CharacterLease::new(
                    CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let facts = owner
            .states
            .spell_states
            .lock()
            .await
            .get(
                &runtime,
                admitted.runtime_actor.ok_or("city actor")?,
                admitted.game_session_id,
            )
            .ok_or("city spell facts")?
            .character_facts();
        assert!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .initialize(
                    &runtime,
                    actor,
                    guest.game_session_id,
                    facts,
                    (0, 0),
                    owner.owner_now()
                )
                .is_some()
        );
        runtime
            .initialize_pinned_test_position(
                actor,
                super::quest_world_callbacks_fixtures::local(Position {
                    x: JIMMY_DOOR.x,
                    y: JIMMY_DOOR.y - 2,
                    floor: JIMMY_DOOR.floor,
                }),
            )
            .map_err(debug)?;
        guest.runtime_actor = Some(actor);
    }
    owner.admit_quest_session(&guest).await;
    step_to(
        owner,
        &mut guest,
        StepDirection::South,
        DOOR_APPROACH,
        deadline,
    )
    .await?;
    let door = target(owner, JIMMY_DOOR)?;
    assert!(door.expected_revision > 0, "Jimmy opened the shared door");
    assert!(!owner.bundle_map().ok_or("city map")?.facts.doors().blocks(JIMMY_DOOR));
    let actor = guest.runtime_actor.ok_or("guest actor")?;
    let entities = owner.observe_channel_entities_projected(actor, true).await.ok_or("guest entities")?;
    let motion = entities.entities.iter()
        .find(|entity| entity.identity == entities.observer)
        .and_then(|entity| entity.motion.as_ref())
        .ok_or("guest movement forecast")?;
    assert_eq!(motion.prewalk_ms[StepDirection::South as usize - 1], 0);
    let command = guest.continuity.next_command_id;
    let reply = timeout_at(
        command_deadline(deadline),
        super::quest_world_callbacks_fixtures::wire(
            owner,
            &mut guest,
            command,
            1,
            encode_step_intent(StepDirection::South),
        ),
    )
    .await
    .map_err(|_| format!("STEP_INTENT {command} at {DOOR_APPROACH:?} timed out"))??;
    let step = decode_step_result_profile(
        &reply,
        guest.continuity.selected_capabilities.contains(13),
        false,
    )
    .map_err(debug)?;
    assert_eq!(
        step.disposition,
        StepDisposition::Blocked,
        "a second session cannot follow through the open quest door"
    );
    assert_eq!(actor_cell(owner, &guest).await?, DOOR_APPROACH);
    walk_to(owner, admitted, cell(JIMMY_STOP), deadline).await
}

fn command_deadline(route_deadline: Instant) -> Instant {
    route_deadline.min(Instant::now() + Duration::from_secs(15))
}

fn cell(position: Position) -> Cell {
    (
        i32::from(position.x),
        i32::from(position.y),
        -i32::from(position.floor),
    )
}

async fn actor_cell(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<Cell> {
    let actor = admitted.runtime_actor.ok_or("city actor")?;
    let origin = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    Ok((origin.x, origin.y, i32::from(origin.floor)))
}

async fn use_placement(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: TilePos,
    deadline: Instant,
) -> TestResult<UseDisposition> {
    let target = target(owner, at)?;
    let command = admitted.continuity.next_command_id;
    timeout_at(
        command_deadline(deadline),
        use_target(owner, admitted, command, &target),
    )
    .await
    .map_err(|_| format!("USE_INTENT {command} at {at:?} timed out"))?
}

async fn step_to(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: StepDirection,
    expected: Cell,
    deadline: Instant,
) -> TestResult {
    let command = admitted.continuity.next_command_id;
    timeout_at(command_deadline(deadline), async {
        super::super::desert_earned_route_fixtures::step(owner, admitted, direction).await?;
        assert_eq!(
            actor_cell(owner, admitted).await?,
            expected,
            "STEP {direction:?}"
        );
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await
    .map_err(|_| format!("STEP_INTENT {command} {direction:?} to {expected:?} timed out"))??;
    Ok(())
}

async fn walk_to(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    stop: Cell,
    deadline: Instant,
) -> TestResult {
    let start = timeout_at(command_deadline(deadline), actor_cell(owner, admitted))
        .await
        .map_err(|_| format!("STEP_INTENT approach to {stop:?} timed out"))??;
    let occupied = [cell(IRIANA), cell(JIMMY)];
    let stairs = table_of(&owner.movement_cells).ok_or("city floor changes")?;
    let doors = BundleDoors(owner.bundle_map().ok_or("city map")?.facts.doors());
    let wrong_lanes = owner
        .callback_content()
        .iter()
        .filter(|callback| {
            callback
                .key
                .starts_with("oteryn:quest-callback/authored-top-of-the-city/")
                && callback.key.ends_with("-lost")
        })
        .flat_map(|callback| {
            callback
                .selector
                .position
                .into_iter()
                .chain(callback.selector.position_aliases.iter().copied())
        })
        .map(cell)
        .collect::<std::collections::BTreeSet<_>>();
    let ground = |(x, y, z)| {
        let cell = LogicalCell { x, y, z };
        if doors.blocks(cell)
            || occupied.contains(&(x, y, z))
            || (x, y, z) != stop && wrong_lanes.contains(&(x, y, z))
        {
            return Ground::Blocked;
        }
        match owner
            .movement_cells
            .index()
            .lookup(owner.movement_cells.scope(), cell)
        {
            Ok(CollisionClass::Walkable) => Ground::Walkable,
            Ok(CollisionClass::Blocked) => Ground::Blocked,
            Err(_) => Ground::Absent,
        }
    };

    // Keep the search within the Yalahar roofs and their approaches.
    let mut queue = VecDeque::from([start]);
    let mut previous = BTreeMap::<Cell, Option<(Cell, StepDirection)>>::from([(start, None)]);
    loop {
        let at = queue
            .pop_front()
            .ok_or_else(|| format!("no source STEP route from {start:?} to {stop:?}"))?;
        if at == stop {
            break;
        }
        for (direction, wire) in [
            (CardinalStep::North, StepDirection::North),
            (CardinalStep::East, StepDirection::East),
            (CardinalStep::South, StepDirection::South),
            (CardinalStep::West, StepDirection::West),
        ] {
            let Ok(next) = resolve_landing(stairs, ground, |_| true, at, direction) else {
                continue;
            };
            if !(32795..=32900).contains(&next.0)
                || !(31235..=31405).contains(&next.1)
                || !(4..=10).contains(&next.2)
            {
                continue;
            }
            if let std::collections::btree_map::Entry::Vacant(entry) = previous.entry(next) {
                entry.insert(Some((at, wire)));
                queue.push_back(next);
            }
        }
    }
    let mut path = Vec::new();
    let mut cursor = stop;
    while let Some(&(parent, direction)) = previous.get(&cursor).and_then(Option::as_ref) {
        path.push((cursor, direction));
        cursor = parent;
    }
    for (expected, direction) in path.into_iter().rev() {
        step_to(owner, admitted, direction, expected, deadline).await?;
    }
    Ok(())
}
