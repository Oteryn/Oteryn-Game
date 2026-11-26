//! M02 client capture: a far door use makes the server autowalk and retry the use.

use super::*;
use crate::map::door::AuthoredClosedDoor;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::world_map::{MapDefinition, MapItem, MapOrigin, MapTile};
use oteryn_protocol_oteryn::world_object::UseDisposition;

const GATE_POSITION: ActorPosition = ActorPosition {
    x: 32402,
    y: 32235,
    floor: 7,
};
#[path = "client_row_m02_replay.rs"]
mod replay;

const CLOSED_GATE: u16 = 2177;
const OPEN_GATE: u16 = 2178;

const GATE: TilePos = TilePos {
    x: 32402,
    y: 32235,
    floor: -7,
};

const EXPECTED_ARRIVAL: ActorPosition = ActorPosition {
    x: 32401,
    y: 32235,
    floor: 7,
};

const APPROACH_POSITION: ActorPosition = ActorPosition {
    x: 32399,
    ..EXPECTED_ARRIVAL
};
const INTERMEDIATE: ActorPosition = ActorPosition {
    x: 32400,
    ..EXPECTED_ARRIVAL
};
const FAR_SIDE: ActorPosition = ActorPosition {
    x: 32403,
    ..EXPECTED_ARRIVAL
};
const RETREAT: [ActorPosition; 2] = [INTERMEDIATE, APPROACH_POSITION];
const PASSAGE: [ActorPosition; 6] = [
    GATE_POSITION,
    FAR_SIDE,
    GATE_POSITION,
    EXPECTED_ARRIVAL,
    GATE_POSITION,
    FAR_SIDE,
];

fn distance(a: TilePos, b: TilePos) -> u16 {
    if a.floor == b.floor {
        a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
    } else {
        u16::MAX
    }
}

fn plain_gate(world: &BundleWorld) -> TestResult<u64> {
    let doors = world.facts().doors();
    let mut keys = doors.keys().filter(|&key| {
        doors.authored_closed(key) == Some(AuthoredClosedDoor::Plain)
            && crate::map::view::placement_of(key).is_some_and(|(at, _)| at == GATE)
    });
    let key = keys.next().ok_or("M02 authored gate is missing")?;
    if keys.next().is_some() {
        return Err("M02 authored gate is ambiguous".into());
    }
    Ok(key)
}

async fn walk_to_approach(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
) -> TestResult<Option<MapTile>> {
    let snapshot = client
        .session_mut()
        .gated_snapshot(oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("M02 initial map snapshot is missing")?;
    let mut gate = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
        .map_err(|error| format!("M02 map snapshot: {error:?}"))?
        .tiles
        .into_iter()
        .find(|tile| tile.position == GATE_POSITION);
    let from = tile(client.world_spatial().actor_position)?;
    let path = route_live(client, world, |at| {
        tile(EXPECTED_ARRIVAL).is_ok_and(|adjacent| at == adjacent)
    })
    .await?
    .ok_or_else(|| format!("M02 authored gate approach is unreachable from {from:?}"))?;
    for direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, before, direction).ok_or("M02 missing planned landing")?;
        let outcome = client.step(direction).await?;
        let after = tile(client.world_spatial().actor_position)?;
        if client.world_spatial().actor_position == FAR_SIDE
            || outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || after != expected
        {
            return Err(format!(
                "M02 approach step {direction:?} from={before:?} expected={expected:?} actual={after:?}: {outcome:?}"
            ).into());
        }
        update_gate(&mut gate, &client.take_events());
        verdict::Harness::global().progress("walk-to-gate");
    }
    Ok(gate)
}

fn is_expected_walk(steps: &[ActorPosition]) -> bool {
    steps
        == [
            ActorPosition {
                x: 32400,
                ..EXPECTED_ARRIVAL
            },
            EXPECTED_ARRIVAL,
        ]
}

fn verify_open_order(arrival: Option<u64>, opening: Option<u64>) -> TestResult {
    if !matches!((arrival, opening), (Some(arrival), Some(opening)) if arrival < opening) {
        return Err(format!(
            "M02 gate must open strictly after arrival: arrival={arrival:?} opening={opening:?}"
        )
        .into());
    }
    Ok(())
}

fn passage_step_matches(
    disposition: StepDisposition,
    delta_position: Option<ActorPosition>,
    actual: ActorPosition,
    expected: ActorPosition,
) -> bool {
    disposition == StepDisposition::Moved && delta_position == Some(expected) && actual == expected
}

fn observe_gate(
    events: &[SessionEvent],
    gate: &mut Option<MapTile>,
    ordinal: u8,
    opening: &mut Option<u64>,
    allow_open: bool,
) -> TestResult {
    for event in events {
        if let SessionEvent::WorldMap(delta) = event {
            update_gate(gate, std::slice::from_ref(event));
            let closed = gate
                .as_ref()
                .is_some_and(|tile| gate_matches(tile, ordinal, CLOSED_GATE, 0));
            let open = gate
                .as_ref()
                .is_some_and(|tile| gate_matches(tile, ordinal, OPEN_GATE, 1));
            match (*opening, closed, open) {
                (None, true, false) => {}
                (None, false, true) if allow_open => *opening = Some(delta.server_sequence),
                (Some(_), false, true) => {}
                _ => {
                    return Err(format!(
                        "M02 invalid gate state at sequence {}: {gate:?}",
                        delta.server_sequence
                    )
                    .into());
                }
            }
        }
    }
    Ok(())
}

fn step_matches(
    disposition: StepDisposition,
    delta_position: Option<ActorPosition>,
    actual: ActorPosition,
    expected: ActorPosition,
    blocked: bool,
) -> bool {
    if blocked {
        disposition == StepDisposition::Blocked && delta_position.is_none() && actual == expected
    } else {
        passage_step_matches(disposition, delta_position, actual, expected)
    }
}

fn gate_matches(tile: &MapTile, ordinal: u8, appearance: u16, revision: u64) -> bool {
    let mut entries = tile.items.iter().filter_map(|item| match item.origin {
        MapOrigin::BaseOrdinal {
            ordinal: at,
            object_revision,
        } if at == ordinal => Some((item.appearance_id, object_revision)),
        _ => None,
    });
    tile.position == GATE_POSITION
        && !tile.more
        && entries.next() == Some((appearance, revision))
        && entries.next().is_none()
}

// Pinned gate cell's two authored, walkable background entries in stack order.
fn gate_background(item_ref: u32, appearance_id: u16, ordinal: u8) -> MapItem {
    MapItem {
        definition: MapDefinition::Item(
            std::num::NonZeroU32::new(item_ref).unwrap_or(std::num::NonZeroU32::MIN),
        ),
        count: 1,
        sub_type: 0,
        origin: MapOrigin::BaseOrdinal {
            ordinal,
            object_revision: 0,
        },
        appearance_id,
    }
}

fn verify_closed_probe_tile(
    gate: Option<&MapTile>,
    ordinal: u8,
    occupants: impl IntoIterator<Item = ActorPosition>,
) -> TestResult {
    let gate = gate.ok_or("M02 closed collision tile is missing")?;
    if ordinal != 2
        || !gate_matches(gate, ordinal, CLOSED_GATE, 0)
        || gate.ground_speed != 150
        || !matches!(gate.items.as_slice(), [ground, decoration, door]
            if *ground == gate_background(328, 103, 0)
                && *decoration == gate_background(25930, 4532, 1)
                && matches!(door.origin, MapOrigin::BaseOrdinal { ordinal: 2, .. }))
    {
        return Err(format!(
            "M02 closed probe requires the complete authored gate stack: {gate:?}"
        )
        .into());
    }
    if occupants
        .into_iter()
        .any(|position| position == GATE_POSITION)
    {
        return Err("M02 another entity occupied the closed collision tile".into());
    }
    Ok(())
}

fn verify_relog_position(position: ActorPosition) -> TestResult {
    if position != FAR_SIDE {
        return Err(
            format!("M02 relog restored {position:?}, expected post-passage {FAR_SIDE:?}").into(),
        );
    }
    Ok(())
}

fn update_gate(gate: &mut Option<MapTile>, events: &[SessionEvent]) {
    for event in events {
        if let SessionEvent::WorldMap(delta) = event {
            if delta.value.cleared.contains(&GATE_POSITION) {
                *gate = None;
            }
            if let Some(tile) = delta
                .value
                .tiles
                .iter()
                .find(|tile| tile.position == GATE_POSITION)
            {
                *gate = Some(tile.clone());
            }
        }
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let capture_dir =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("M02 requires a fresh capture directory")?;
    let (result, wire) =
        oteryn_dev_client::native_session::capture_qualification_wire(journey_capture(clients))
            .await;
    let (binding, frames) = result?;
    replay::write_capture(Path::new(&capture_dir), &binding, frames, wire)?;
    evidence(&format!(
        "e2e_capture scenario=M02.client status=CAPTURED binding={}",
        binding.identity()
    ));
    Ok(())
}

async fn journey_capture(
    clients: &SeamClients<'_>,
) -> TestResult<(replay::Capture, replay::SequencedFrames)> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let key = plain_gate(world)?;
    let mut client = connect_world(clients, 0xa2).await?;
    if matches!(
        client.world_spatial().actor_position,
        EXPECTED_ARRIVAL | FAR_SIDE
    ) {
        return Err("M02 first admission already starts at the expected relog position".into());
    }
    let (_, ordinal) = crate::map::view::placement_of(key).ok_or("M02 invalid gate placement")?;
    let mut gate = walk_to_approach(&mut client, world).await?;
    if !gate
        .as_ref()
        .is_some_and(|tile| gate_matches(tile, ordinal, CLOSED_GATE, 0))
    {
        return Err(
            "M02 approach did not observe the authored closed gate at revision zero".into(),
        );
    }
    if client.world_spatial().actor_position != EXPECTED_ARRIVAL {
        return Err("M02 route missed its closed collision position".into());
    }
    let mut opening = None;
    observe_gate(
        &client.take_events(),
        &mut gate,
        ordinal,
        &mut opening,
        false,
    )?;
    verify_closed_probe_tile(
        gate.as_ref(),
        ordinal,
        client
            .world_entities()
            .ok_or("M02 requires a visible entity baseline")?
            .others()
            .map(|entity| entity.position),
    )?;
    let map = client
        .session_mut()
        .world_map_snapshot()
        .ok_or("M02 missing map binding")?
        .header;
    let session = client.join_snapshot().game_session_id;
    let first_sequence = client
        .last_server_sequence()
        .checked_add(1)
        .ok_or("M02 sequence overflow")?;
    let capabilities = client.selected_capabilities().to_vec();
    client.session_mut().begin_qualification_capture();
    let mut setup = Vec::new();
    verdict::Harness::global().progress("closed-collision");
    for (direction, expected, blocked) in [
        (StepDirection::East, EXPECTED_ARRIVAL, true),
        (StepDirection::West, RETREAT[0], false),
        (StepDirection::West, RETREAT[1], false),
    ] {
        let step = client.step(direction).await?;
        require_mechanic_acceptance(step.status, "M02 closed probe or retreat")?;
        let events = client.take_events();
        if blocked {
            for event in &events {
                observe_gate(
                    std::slice::from_ref(event),
                    &mut gate,
                    ordinal,
                    &mut opening,
                    false,
                )?;
                let occupants = match event {
                    SessionEvent::WorldSpatialEntities(delta) => delta
                        .value
                        .enter
                        .iter()
                        .chain(&delta.value.update)
                        .map(|entity| entity.position)
                        .collect::<Vec<_>>(),
                    _ => Vec::new(),
                };
                verify_closed_probe_tile(gate.as_ref(), ordinal, occupants)?;
            }
            if client.session_mut().last_refusal_detail()
                != Some(oteryn_protocol_oteryn::refusal_detail::RefusalDetail::NotPossible)
            {
                return Err("M02 blocked probe missed the negotiated movement refusal".into());
            }
        } else {
            observe_gate(&events, &mut gate, ordinal, &mut opening, false)?;
        }
        if !step_matches(
            step.disposition,
            step.world_spatial_delta
                .as_ref()
                .map(|delta| delta.value.actor_position),
            client.world_spatial().actor_position,
            expected,
            blocked,
        ) || events.iter().any(|event| {
            matches!(event,
            SessionEvent::WorldSpatialEntities(delta) if delta.value.actor_position != expected)
        }) {
            return Err(format!(
                "M02 closed probe or retreat {direction:?} at {expected:?}: {step:?}"
            )
            .into());
        }
        setup.push((step.command_id, step.result_server_sequence));
    }
    let setup_end = client.last_server_sequence();
    verdict::Harness::global().progress("far-use");
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let result = client.use_object(&placement, 0).await?;
    require_mechanic_acceptance(result.status, "M02 far use")?;
    if result.disposition != UseDisposition::Committed {
        return Err(format!("M02 far use did not commit: {result:?}").into());
    }
    let events = client.take_events();
    let mut previous = APPROACH_POSITION;
    let mut steps = Vec::new();
    let mut arrival = None;
    for event in &events {
        if let SessionEvent::WorldSpatialEntities(delta) = event
            && delta.value.actor_position != previous
        {
            if delta.server_sequence >= result.result_server_sequence {
                return Err("M02 actor moved after the far-use result".into());
            }
            previous = delta.value.actor_position;
            steps.push(previous);
            if previous == EXPECTED_ARRIVAL {
                arrival = Some(delta.server_sequence);
            }
        }
    }
    if !is_expected_walk(&steps) {
        return Err(
            format!("M02 expected two consecutive eastward autowalk steps, got {steps:?}").into(),
        );
    }
    observe_gate(&events, &mut gate, ordinal, &mut opening, true)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !gate
        .as_ref()
        .is_some_and(|tile| gate_matches(tile, ordinal, OPEN_GATE, 1))
    {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("M02 far use did not publish the open gate at revision one".into());
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(100)))
            .await?;
        let events = client.take_events();
        if events.iter().any(|event| {
            matches!(event, SessionEvent::WorldSpatialEntities(delta)
                if delta.value.actor_position != EXPECTED_ARRIVAL)
        }) {
            return Err("M02 actor moved while waiting for the gate update".into());
        }
        observe_gate(&events, &mut gate, ordinal, &mut opening, true)?;
    }
    verify_open_order(arrival, opening)?;
    let arrived = client.world_spatial().actor_position;
    if arrived != EXPECTED_ARRIVAL || distance(tile(arrived)?, GATE) > 1 {
        return Err(format!(
            "M02 far use did not reach {EXPECTED_ARRIVAL:?} adjacent to the gate: {arrived:?}"
        )
        .into());
    }
    let opening_end = client.last_server_sequence();
    let mut passage = Vec::new();
    for (direction, expected) in [
        (StepDirection::East, PASSAGE[0]),
        (StepDirection::East, PASSAGE[1]),
        (StepDirection::West, PASSAGE[2]),
        (StepDirection::West, PASSAGE[3]),
        (StepDirection::East, PASSAGE[4]),
        (StepDirection::East, PASSAGE[5]),
    ] {
        let step = client.step(direction).await?;
        passage.push((step.command_id, step.result_server_sequence));
        let events = client.take_events();
        observe_gate(&events, &mut gate, ordinal, &mut opening, true)?;
        if events.iter().any(|event| {
            matches!(event,
            SessionEvent::WorldSpatialEntities(delta) if delta.value.actor_position != expected)
        }) {
            return Err("M02 unexpected movement during gate passage".into());
        }
        require_mechanic_acceptance(step.status, "M02 gate passage")?;
        if !passage_step_matches(
            step.disposition,
            step.world_spatial_delta
                .as_ref()
                .map(|delta| delta.value.actor_position),
            client.world_spatial().actor_position,
            expected,
        ) {
            return Err(format!(
                "M02 gate passage step {direction:?} did not reach {expected:?}: {step:?}"
            )
            .into());
        }
    }
    // Give the final paced step a quiet interval before the single logout attempt.
    verdict::Harness::global().progress("logout-and-relog");
    client.service_liveness(Duration::from_secs(1)).await?;
    let events = client.take_events();
    observe_gate(&events, &mut gate, ordinal, &mut opening, true)?;
    if events.iter().any(|event| {
        matches!(event, SessionEvent::WorldSpatialEntities(delta)
        if delta.value.actor_position != FAR_SIDE)
    }) {
        return Err("M02 position changed before logout".into());
    }
    verify_relog_position(client.world_spatial().actor_position)?;
    let logout = client.session_mut().logout().await?;
    require_mechanic_acceptance(logout.status, "M02 first logout")?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("M02 first logout refused: {logout:?}").into());
    }
    let last_sequence = client.last_server_sequence();
    let frames = client.session_mut().take_qualification_capture();
    drop(client);
    let relog = connect_world(clients, 0xa3).await?;
    if relog.join_snapshot().game_session_id == session {
        return Err("M02 relog reused the closed session".into());
    }
    verify_relog_position(relog.world_spatial().actor_position)?;
    evidence(
        "M02_GAMEPLAY_VERIFIED closed_collision=blocked steps=2 gate=open revision=1 passage=east,east,west,west,east,east relog_position=32403,32235,7",
    );
    let binding = replay::Capture {
        game_session_id: uuid_text(session.as_bytes()),
        character: clients.characters[1],
        command_id: result.command_id,
        first_sequence,
        result_sequence: result.result_server_sequence,
        last_sequence,
        setup: setup.try_into().map_err(|_| "M02 setup command count")?,
        setup_end,
        opening_end,
        logout: (logout.command_id, logout.result_server_sequence),
        passage: passage
            .try_into()
            .map_err(|_| "M02 passage command count")?,
        capabilities,
        content_generation: map.content_generation,
        bundle_digest: map.bundle_digest,
        reset_epoch: map.reset_epoch,
        first_visible_floor: map.first_visible_floor,
        ordinal,
    };
    Ok((binding, frames))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = verdict::observe(journey(clients)).await {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=M02.client status=FAIL evidence={error}"
        ));
    }
}

#[test]
fn m02_requires_intermediate_step_in_order() {
    let intermediate = ActorPosition {
        x: 32400,
        ..EXPECTED_ARRIVAL
    };
    assert!(is_expected_walk(&[intermediate, EXPECTED_ARRIVAL]));
    assert!(!is_expected_walk(&[EXPECTED_ARRIVAL]));
    assert!(!is_expected_walk(&[EXPECTED_ARRIVAL, intermediate]));
}

#[test]
fn m02_requires_open_appearance_at_next_revision() -> TestResult {
    let mut gate = test_gate(CLOSED_GATE, 0);
    assert!(gate_matches(&gate, 1, CLOSED_GATE, 0));
    gate.items[0].appearance_id = OPEN_GATE;
    assert!(!gate_matches(&gate, 1, OPEN_GATE, 1));
    assert!(gate_matches(&test_gate(OPEN_GATE, 1), 1, OPEN_GATE, 1));
    Ok(())
}

#[cfg(test)]
fn test_gate(appearance_id: u16, object_revision: u64) -> MapTile {
    use oteryn_protocol_oteryn::world_map::{MapDefinition, MapItem};
    MapTile {
        position: GATE_POSITION,
        items: vec![MapItem {
            definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
            count: 1,
            sub_type: 0,
            origin: MapOrigin::BaseOrdinal {
                ordinal: 1,
                object_revision,
            },
            appearance_id,
        }],
        more: false,
        ground_speed: 150,
    }
}

#[test]
fn m02_rejects_open_at_wrong_revision_and_reclose_during_passage() -> TestResult {
    use oteryn_protocol_oteryn::world_map::{MapViewHeader, WorldMapViewDelta};
    let update = |server_sequence, appearance, revision| {
        SessionEvent::WorldMap(oteryn_dev_client::native_session::AppliedDelta {
            server_sequence,
            base_revision: 1,
            new_revision: 2,
            value: WorldMapViewDelta {
                header: MapViewHeader {
                    content_generation: [0; 32],
                    bundle_digest: [0; 32],
                    reset_epoch: 0,
                    origin: EXPECTED_ARRIVAL,
                    first_visible_floor: 0,
                },
                tiles: vec![test_gate(appearance, revision)],
                cleared: vec![],
            },
        })
    };
    let mut opening = None;
    let mut gate = Some(test_gate(CLOSED_GATE, 0));
    assert!(
        observe_gate(
            &[update(10, OPEN_GATE, 2), update(11, OPEN_GATE, 1)],
            &mut gate,
            1,
            &mut opening,
            true,
        )
        .is_err()
    );
    assert_eq!(opening, None);
    observe_gate(
        &[update(12, OPEN_GATE, 1)],
        &mut gate,
        1,
        &mut opening,
        true,
    )?;
    assert_eq!(opening, Some(12));
    assert!(
        observe_gate(
            &[update(13, CLOSED_GATE, 2), update(14, OPEN_GATE, 1)],
            &mut gate,
            1,
            &mut opening,
            true,
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn m02_rejects_closed_gate_that_allows_movement_and_open_gate_that_blocks_exit() {
    assert!(step_matches(
        StepDisposition::Blocked,
        None,
        EXPECTED_ARRIVAL,
        EXPECTED_ARRIVAL,
        true
    ));
    assert!(!step_matches(
        StepDisposition::Moved,
        Some(GATE_POSITION),
        GATE_POSITION,
        EXPECTED_ARRIVAL,
        true
    ));
    assert!(!step_matches(
        StepDisposition::Blocked,
        None,
        GATE_POSITION,
        FAR_SIDE,
        false
    ));
}

#[test]
fn m02_opening_requires_both_steps_before_gate_transition() -> TestResult {
    verify_open_order(Some(12), Some(14))?;
    assert!(verify_open_order(Some(12), Some(11)).is_err());
    assert!(verify_open_order(Some(12), Some(12)).is_err());
    assert!(verify_open_order(None, Some(14)).is_err());
    assert!(verify_open_order(Some(12), None).is_err());
    Ok(())
}

#[test]
fn m02_open_gate_must_admit_first_passage_step() {
    assert!(passage_step_matches(
        StepDisposition::Moved,
        Some(GATE_POSITION),
        GATE_POSITION,
        GATE_POSITION
    ));
    assert!(!passage_step_matches(
        StepDisposition::Blocked,
        None,
        EXPECTED_ARRIVAL,
        GATE_POSITION
    ));
    assert!(!passage_step_matches(
        StepDisposition::Moved,
        Some(EXPECTED_ARRIVAL),
        EXPECTED_ARRIVAL,
        GATE_POSITION
    ));
}

#[test]
fn m02_relog_rejects_saved_setup_and_autowalk_positions() -> TestResult {
    for stale in [
        APPROACH_POSITION,
        INTERMEDIATE,
        EXPECTED_ARRIVAL,
        GATE_POSITION,
    ] {
        assert!(verify_relog_position(stale).is_err());
    }
    verify_relog_position(FAR_SIDE)
}

#[test]
fn m02_closed_collision_requires_an_isolated_complete_gate_tile() -> TestResult {
    let mut gate = test_gate(CLOSED_GATE, 0);
    gate.items[0].origin = MapOrigin::BaseOrdinal {
        ordinal: 2,
        object_revision: 0,
    };
    gate.items.insert(0, gate_background(25930, 4532, 1));
    gate.items.insert(0, gate_background(328, 103, 0));
    verify_closed_probe_tile(Some(&gate), 2, [EXPECTED_ARRIVAL])?;
    assert!(verify_closed_probe_tile(Some(&gate), 2, [GATE_POSITION]).is_err());
    gate.more = true;
    assert!(verify_closed_probe_tile(Some(&gate), 2, []).is_err());
    gate.more = false;
    gate.items[0].appearance_id += 1;
    assert!(verify_closed_probe_tile(Some(&gate), 2, []).is_err());
    gate.items[0].appearance_id -= 1;
    let mut extra = gate.items[0];
    extra.origin = MapOrigin::DisplayOnly;
    gate.items.push(extra);
    assert!(verify_closed_probe_tile(Some(&gate), 2, []).is_err());
    Ok(())
}

#[test]
fn m02_conflicting_ordinal_rejects_even_the_correct_open_entry() {
    let mut gate = test_gate(OPEN_GATE, 1);
    gate.items.extend(test_gate(CLOSED_GATE, 2).items);
    assert!(!gate_matches(&gate, 1, OPEN_GATE, 1));
    gate.items.reverse();
    assert!(!gate_matches(&gate, 1, OPEN_GATE, 1));
}
