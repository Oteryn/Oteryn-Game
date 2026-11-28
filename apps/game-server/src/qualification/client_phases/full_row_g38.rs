//! G38: Windows touch joystick and action hotbar (#3074, O035 and O056).
//! Require live client library touch assertions and served World STEP and USE receipts.
use super::*;
use crate::map::door::AuthoredClosedDoor;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_map::{MapOrigin, MapTile};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::{
    CAPABILITY_DIAGONAL_MOVEMENT_V1, CAPABILITY_PACED_MOVEMENT_V1,
};

const TOUCH_REPORT: &str = "client_tests:verified-touch";

const TOUCH_TESTS: [&str; 5] = [
    "touch_overlay_is_hidden_until_touch_and_highlights_the_selected_mode",
    "touch_joystick_uses_paced_keyboard_steps_and_release_stops",
    "touch_hotbar_world_taps_equal_mouse_context_and_follow_keys",
    "touch_default_tap_dead_zone_and_contact_ownership",
    "negotiated_touch_diagonals_submit_one_diagonal_step_per_receipt",
];

fn verify_client_touch() -> TestResult {
    let executable = required("NODE_BOOT_G38_CLIENT_TEST")?;
    if executable.is_empty() {
        return Err("step=client-touch;observed=missing-test-executable".into());
    }
    let filter = "play::tests::touch_controls::";
    let output = std::process::Command::new("timeout")
        .args(["30", &executable, filter, "--test-threads=1"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../client"))
        .output()?;
    let stdout = std::str::from_utf8(&output.stdout)?;
    if !output.status.success() {
        return Err(format!("step=client-touch;observed={}\n{stdout}", output.status).into());
    }
    for case in TOUCH_TESTS {
        let expected = format!("test {filter}{case} ... ok");
        if stdout.lines().filter(|line| *line == expected).count() != 1 {
            return Err(format!("step=client-touch;missing={filter}{case}").into());
        }
        evidence(&format!("G38_CHECK client_test={filter}{case} result=ok"));
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    // The 73-step approach and return each wait for the server's pacing.
    // At 180 seconds a live run was still walking back. The progress
    // watchdog applies throughout both legs.
    let result =
        verdict::indexed_timeout(Duration::from_secs(300), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            format!(
                "paced-cardinal-and-diagonal-step-receipts;hotbar-use-open-close;{TOUCH_REPORT}"
            ),
        ),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G38.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn step_receipt(
    client: &mut DevClientSession,
    direction: StepDirection,
    expected: TilePos,
) -> TestResult {
    let outcome = client.step(direction).await?;
    if outcome.status != CommandStatus::Accepted
        || outcome.disposition != StepDisposition::Moved
        || outcome.step_wait_ms == 0
        || outcome.world_spatial_delta.as_ref().is_none_or(|delta| {
            delta.server_sequence <= outcome.result_server_sequence
                || tile(delta.value.actor_position).ok() != Some(expected)
        })
        || tile(client.world_spatial().actor_position)? != expected
    {
        return Err(format!("step={direction:?};needed={expected:?};observed={outcome:?}").into());
    }
    verdict::Harness::global().progress("G38 step receipt");
    tokio::time::sleep(Duration::from_millis(u64::from(outcome.step_wait_ms))).await;
    Ok(())
}

async fn hotbar_use(
    client: &mut DevClientSession,
    placement: &[u8],
    position: ActorPosition,
    ordinal: u8,
    revision: u64,
    appearance: u16,
) -> TestResult<u64> {
    client.take_events();
    let outcome = client.use_object(placement, revision).await?;
    if outcome.status != CommandStatus::Accepted || outcome.disposition != UseDisposition::Committed
    {
        return Err(format!("step=hotbar-use;observed={outcome:?}").into());
    }
    let next_revision = revision.checked_add(1).ok_or("door revision overflow")?;
    let expected = MapOrigin::BaseOrdinal {
        ordinal,
        object_revision: next_revision,
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::WorldMap(delta) = event
                && delta.server_sequence > outcome.result_server_sequence
                && let Some(cell) = delta
                    .value
                    .tiles
                    .iter()
                    .find(|cell| cell.position == position)
                && let Some(item) = cell.items.iter().find(|item| item.origin == expected)
            {
                if item.appearance_id == appearance {
                    return Ok(next_revision);
                }
                return Err(format!(
                    "step=hotbar-map;observed={item:?};needed=appearance-{appearance}"
                )
                .into());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=hotbar-map;observed=missing-door-update".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

fn observe_door_frame(
    event: SessionEvent,
    after: u64,
    position: ActorPosition,
    observed: &mut Option<([u8; 32], MapTile)>,
) {
    match event {
        SessionEvent::WorldMapSnapshot(frame) if frame.server_sequence > after => {
            *observed = frame
                .value
                .tiles
                .into_iter()
                .find(|tile| tile.position == position)
                .map(|tile| (frame.value.header.bundle_digest, tile));
        }
        SessionEvent::WorldMap(frame) if frame.server_sequence > after => {
            if let Some(tile) = frame
                .value
                .tiles
                .into_iter()
                .find(|tile| tile.position == position)
            {
                *observed = Some((frame.value.header.bundle_digest, tile));
            } else if frame.value.cleared.contains(&position) {
                *observed = None;
            }
        }
        _ => {}
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    use StepDirection::*;

    verify_client_touch()?;

    let world = load_world(
        std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?,
        &serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xe9).await?;
    for capability in [
        CAPABILITY_PACED_MOVEMENT_V1,
        CAPABILITY_DIAGONAL_MOVEMENT_V1,
    ] {
        if !client.selected_capabilities().contains(&capability) {
            return Err(format!("step=admit;needed=capability-{capability}").into());
        }
    }
    let start = tile(client.world_spatial().actor_position)?;
    for (directions, diagonal) in [
        (
            [(North, South), (East, West), (South, North), (West, East)],
            false,
        ),
        (
            [
                (NorthEast, SouthWest),
                (SouthEast, NorthWest),
                (SouthWest, NorthEast),
                (NorthWest, SouthEast),
            ],
            true,
        ),
    ] {
        let (out, back, end) = directions
            .into_iter()
            .find_map(|(out, back)| {
                let end = destination_with_teleports(world, start, out, false)?;
                let dx = end.x.abs_diff(start.x);
                let dy = end.y.abs_diff(start.y);
                (end.floor == start.floor
                    && dx + dy == if diagonal { 2 } else { 1 }
                    && dx <= 1
                    && dy <= 1
                    && destination_with_teleports(world, end, back, false) == Some(start))
                .then_some((out, back, end))
            })
            .ok_or_else(|| format!("step=route;needed=reversible-step;diagonal={diagonal}"))?;
        step_receipt(&mut client, out, end).await?;
        step_receipt(&mut client, back, start).await?;
    }

    let doors = world.facts().doors();
    let mut approaches = BTreeMap::new();
    for key in doors
        .keys()
        .filter(|&key| doors.authored_closed(key) == Some(AuthoredClosedDoor::Plain))
    {
        let Some((door, ordinal)) = crate::map::view::placement_of(key) else {
            continue;
        };
        if door.floor != start.floor {
            continue;
        }
        let Some(closed) = world
            .base()
            .tile(door.x, door.y, door.floor)
            .and_then(|cell| cell.appearance(ordinal))
        else {
            continue;
        };
        let Some(open) =
            crate::map::door::open_of(u32::from(closed)).and_then(|id| u16::try_from(id).ok())
        else {
            continue;
        };
        for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            if let Some((x, y)) = door
                .x
                .checked_add_signed(dx)
                .zip(door.y.checked_add_signed(dy))
            {
                approaches
                    .entry(TilePos {
                        x,
                        y,
                        floor: door.floor,
                    })
                    .or_insert((door, key, ordinal, closed, open));
            }
        }
    }
    let path = route(world, start, |at| approaches.contains_key(&at))
        .ok_or("step=hotbar-route;needed=reachable-plain-door")?;
    evidence(&format!(
        "G38_CHECK step=hotbar-approach planned_steps={}",
        path.len()
    ));
    let approach_sequence = client.last_server_sequence();
    let mut at = start;
    for direction in path {
        at = destination(world, at, direction)
            .ok_or("step=hotbar-route;observed=missing-landing")?;
        step_receipt(&mut client, direction, at).await?;
    }
    let &(door, key, ordinal, closed, open) = approaches
        .get(&at)
        .ok_or("step=hotbar-route;observed=not-adjacent")?;
    let position = ActorPosition {
        x: door.x.into(),
        y: door.y.into(),
        floor: -i16::from(door.floor),
    };
    let mut observed = None;
    client.service_liveness(Duration::from_millis(50)).await?;
    for event in client.take_events() {
        observe_door_frame(event, approach_sequence, position, &mut observed);
    }
    let (bundle_digest, observed) =
        observed.ok_or("step=hotbar-route;needed=fresh-served-door-tile")?;
    let revision = observed
        .items
        .into_iter()
        .find_map(|item| match item.origin {
            MapOrigin::BaseOrdinal {
                ordinal: seen,
                object_revision,
            } if seen == ordinal && item.appearance_id == closed => Some(object_revision),
            _ => None,
        })
        .ok_or("step=hotbar-route;needed=served-closed-door")?;
    let mut placement = bundle_digest.to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let open_revision =
        hotbar_use(&mut client, &placement, position, ordinal, revision, open).await?;
    hotbar_use(
        &mut client,
        &placement,
        position,
        ordinal,
        open_revision,
        closed,
    )
    .await?;

    evidence("G38_CHECK step=hotbar-use open-and-close=PASS");
    let at = tile(client.world_spatial().actor_position)?;
    let path =
        route(world, at, |at| at == start).ok_or("step=restore-position;needed=route-to-start")?;
    let mut at = at;
    for direction in path {
        at = destination(world, at, direction)
            .ok_or("step=restore-position;observed=missing-landing")?;
        step_receipt(&mut client, direction, at).await?;
    }
    if tile(client.world_spatial().actor_position)? != start {
        return Err("step=restore-position;observed=not-at-start".into());
    }
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout;observed={logout:?}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn door_prestate_requires_a_fresh_frame_and_discards_a_cleared_tile() {
        use oteryn_dev_client::AppliedDelta;
        use oteryn_protocol_oteryn::world_map::{MapViewHeader, WorldMapViewDelta};

        let position = ActorPosition {
            x: 10,
            y: 20,
            floor: -7,
        };
        let tile = MapTile {
            position,
            items: Vec::new(),
            more: false,
            ground_speed: 100,
        };
        let frame = |server_sequence, tiles, cleared| {
            SessionEvent::WorldMap(AppliedDelta {
                server_sequence,
                base_revision: 1,
                new_revision: 2,
                value: WorldMapViewDelta {
                    header: MapViewHeader {
                        content_generation: [0; 32],
                        bundle_digest: [1; 32],
                        reset_epoch: 0,
                        origin: position,
                        first_visible_floor: -7,
                    },
                    tiles,
                    cleared,
                },
            })
        };
        let mut seen = None;
        observe_door_frame(
            frame(40, vec![tile.clone()], Vec::new()),
            40,
            position,
            &mut seen,
        );
        assert_eq!(seen, None);
        observe_door_frame(
            frame(41, vec![tile.clone()], Vec::new()),
            40,
            position,
            &mut seen,
        );
        assert_eq!(seen, Some(([1; 32], tile)));
        observe_door_frame(
            frame(42, Vec::new(), vec![position]),
            40,
            position,
            &mut seen,
        );
        assert_eq!(seen, None);
    }

    #[test]
    fn source_token_has_no_spaces() {
        assert!(TOUCH_REPORT.starts_with("client_tests:") && !TOUCH_REPORT.contains(' '));
        assert_eq!(
            token(
                "a
b"
            ),
            "a-b"
        );
    }
}
