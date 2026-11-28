//! W24 records the original inbound session frames for client replay.
use super::*;
use oteryn_dev_client::{
    DevClientSession, SessionEvent,
    native_session::{take_qualification_frames, with_qualification_frames},
};
use oteryn_protocol_oteryn::presentation::{PresentationEvent, PresentationEventKind};

use super::full_row_w24::{city_replacement, clock_snapshot, zone};

fn ambient_batch(
    client: &mut DevClientSession,
    minute: &mut u16,
) -> TestResult<Option<(PresentationEvent, u16, u64)>> {
    let mut batch = None;
    for event in client.take_events() {
        match event {
            SessionEvent::WorldClock(delta) => *minute = delta.value.tibian_minute,
            SessionEvent::Presentation(delta) => {
                for event in delta.value {
                    if matches!(event.kind, PresentationEventKind::AmbientSound { .. })
                        && batch
                            .replace((event, *minute, delta.server_sequence))
                            .is_some()
                    {
                        return Err("step=ambient unexpected multiple cues".into());
                    }
                }
            }
            _ => {}
        }
    }
    Ok(batch)
}

async fn wait_for_ambient(
    client: &mut DevClientSession,
    minute: &mut u16,
) -> TestResult<Option<(PresentationEvent, u16, u64)>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    loop {
        if let Some(batch) = ambient_batch(client, minute)? {
            return Ok(Some(batch));
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(25)))
            .await?;
    }
}

fn walking_landing(
    world: &BundleWorld,
    from: TilePos,
    direction: StepDirection,
) -> Option<TilePos> {
    let (dx, dy) = direction.delta();
    let at = TilePos {
        x: u16::try_from(i32::from(from.x) + dx).ok()?,
        y: u16::try_from(i32::from(from.y) + dy).ok()?,
        floor: from.floor,
    };
    (world.floors().teleport(at).is_none()
        && destination_with_teleports(world, from, direction, false) == Some(at))
    .then_some(at)
}

async fn walk_boundary(
    client: &mut DevClientSession,
    world: &BundleWorld,
    minute: &mut u16,
    entering: bool,
) -> TestResult<(String, (PresentationEvent, u16, u64))> {
    let from = tile(client.world_spatial().actor_position)?;
    let path = route_with_landings(
        from,
        |at| zone(world, at).is_some() == entering,
        |_| true,
        |at, direction| walking_landing(world, at, direction),
    )
    .ok_or("step=boundary no non-teleport walking route")?;
    let mut route = String::new();
    let mut crossing_sequence = None;
    for direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected = walking_landing(world, before, direction)
            .ok_or("step=walk missing ordinary landing")?;
        let reply = client.step(direction).await?;
        let delta = reply
            .world_spatial_delta
            .ok_or("step=walk missing spatial delta")?;
        if reply.disposition != StepDisposition::Moved
            || delta.server_sequence <= reply.result_server_sequence
            || tile(delta.value.actor_position)? != expected
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(
                format!("step=walk invalid first reply from={before:?} to={expected:?}").into(),
            );
        }
        route.push(match direction {
            StepDirection::North => 'N',
            StepDirection::East => 'E',
            StepDirection::South => 'S',
            StepDirection::West => 'W',
            _ => return Err("step=walk non-cardinal route".into()),
        });
        if zone(world, expected).is_some() == entering {
            crossing_sequence = Some(delta.server_sequence);
        } else if ambient_batch(client, minute)?.is_some() {
            return Err("step=boundary unexpected ambient before crossing".into());
        }
    }
    let crossing_sequence = crossing_sequence.ok_or("step=boundary missing crossing")?;
    let cue = wait_for_ambient(client, minute)
        .await?
        .ok_or("step=boundary missing replacement cue")?;
    if cue.2 <= crossing_sequence {
        return Err("step=boundary replacement preceded the crossing spatial delta".into());
    }
    Ok((route, cue))
}

async fn capture_logout(
    client: &mut DevClientSession,
    minute: &mut u16,
    root: &Path,
    name: &str,
    initial: (u32, u64),
    crossing: Option<(&str, u32, u64)>,
) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("step={name} logout refused").into());
    }
    if ambient_batch(client, minute)?.is_some() {
        return Err(format!("step={name} unexpected ambient during logout").into());
    }
    let session = client.join_snapshot().game_session_id;
    let (cue, sequence) = initial;
    let crossing = crossing.map_or_else(
        || "-".to_owned(),
        |(path, cue, seq)| format!("{path} {cue} {seq}"),
    );
    std::fs::write(
        root.join(format!("{name}.bin")),
        take_qualification_frames(),
    )?;
    std::fs::write(
        root.join(format!("{name}.txt")),
        format!("{session:?}\n{cue} {sequence}\n{crossing}"),
    )?;
    Ok(())
}

fn city_cue(city: &str, event: PresentationEvent, minute: u16) -> TestResult<u32> {
    if let PresentationEventKind::AmbientSound { cue_id } = event.kind
        && city_replacement(city, &[(cue_id, minute)])
    {
        return Ok(cue_id);
    }
    Err(format!(
        "step=city-cue city={city} minute={minute} observed={:?}",
        event.kind
    )
    .into())
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    let root = std::env::var_os("E2E_ALL_W24_CAPTURE_DIR")
        .or_else(|| std::env::var_os("E2E_ALL_CAPTURE_DIR"))
        .ok_or("step=replay capture directory missing")?;
    let root = Path::new(&root);
    let mut client = connect_character(clients, 0x9d, 1).await?;
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::presentation::CAPABILITY_AMBIENT_SOUND_V1)
    {
        return Err("step=admission ambient capability 288 not selected".into());
    }
    let mut minute = clock_snapshot(&mut client)?;
    let from = tile(client.world_spatial().actor_position)?;
    let exited = if let Some(city) = zone(world, from) {
        let (event, observed_minute, initial_sequence) = wait_for_ambient(&mut client, &mut minute)
            .await?
            .ok_or("step=starting-city missing admission ambience")?;
        let initial_cue = city_cue(city, event, observed_minute)?;
        let (route, (event, _, exit_sequence)) =
            walk_boundary(&mut client, world, &mut minute, false).await?;
        require_silence(event)?;
        let outside = client.world_spatial().actor_position;
        let session = client.join_snapshot().game_session_id;
        capture_logout(
            &mut client,
            &mut minute,
            root,
            "w24-exit",
            (initial_cue, initial_sequence),
            Some((&route, 0, exit_sequence)),
        )
        .await?;
        drop(client);
        client = connect_character(clients, 0x9d, 1).await?;
        minute = clock_snapshot(&mut client)?;
        if client.join_snapshot().game_session_id == session
            || client.world_spatial().actor_position != outside
        {
            return Err("step=outside fresh admission did not restore the saved position".into());
        }
        true
    } else {
        false
    };
    let (outside, _, outside_sequence) = wait_for_ambient(&mut client, &mut minute)
        .await?
        .ok_or("step=outside missing admission ambience")?;
    require_silence(outside)?;
    let (route, (entry, observed_minute, city_sequence)) =
        walk_boundary(&mut client, world, &mut minute, true).await?;
    let expected_position = client.world_spatial().actor_position;
    let city = zone(world, tile(expected_position)?).ok_or("step=city-entry still outside")?;
    let replacement = city_cue(city, entry, observed_minute)?;
    let session = client.join_snapshot().game_session_id;
    capture_logout(
        &mut client,
        &mut minute,
        root,
        "w24-entry",
        (0, outside_sequence),
        Some((&route, replacement, city_sequence)),
    )
    .await?;
    drop(client);

    let mut fresh = connect_character(clients, 0x9d, 1).await?;
    if fresh.join_snapshot().game_session_id == session
        || fresh.world_spatial().actor_position != expected_position
    {
        return Err("step=city-relog saved position differs from city entry".into());
    }
    let mut minute = clock_snapshot(&mut fresh)?;
    let (event, observed_minute, sequence) = wait_for_ambient(&mut fresh, &mut minute)
        .await?
        .ok_or("step=city-relog missing admission ambience")?;
    let relog_cue = city_cue(city, event, observed_minute)?;
    capture_logout(
        &mut fresh,
        &mut minute,
        root,
        "w24-relog",
        (relog_cue, sequence),
        None,
    )
    .await?;
    std::fs::write(
        root.join("w24-expected.txt"),
        if exited { "exit" } else { "outside" },
    )?;
    Ok(format!(
        "outside=0 city={replacement} relog={relog_cue} exited={exited}"
    ))
}

fn require_silence(event: PresentationEvent) -> TestResult {
    if !matches!(
        event.kind,
        PresentationEventKind::AmbientSound { cue_id: 0 }
    ) {
        return Err(format!("step=outside incorrect cue {:?}", event.kind).into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    match verdict::indexed_timeout(
        Duration::from_secs(240),
        verdict::observe(with_qualification_frames(journey(clients, world))),
    )
    .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W24.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W24.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W24.client status=FAIL evidence=timeout"),
    }
}
