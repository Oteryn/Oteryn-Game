//! W24 full-row player scenario, derived from the matrix/audit and the native
//! ambient owner in `gameplay_transport/ambient_sounds.rs`:
//! 1. Admit the provisioned character and require the private ambient snapshot
//!    delta (`AmbientSound { cue_id: 0 }` outside a supported city). If an earlier
//!    row left the character in a city, walk out and freshly admit there first.
//! 2. Send ordinary STEP commands along the compiled map from the current tile
//!    into a supported city subarea; require every Moved reply and require the
//!    city's non-zero day/night ambient replacement delta.
//! 3. Send a STEP into compiled collision from a reachable boundary tile;
//!    require the Blocked reply, no position delta, and no ambient replacement.
//! 4. Send ordinary STEP commands out of the city; require every Moved reply and
//!    the stop replacement delta (`cue_id: 0`).
//! 5. Walk back into the city, require its replacement again, logout normally,
//!    freshly admit the same character, and re-check the persisted position and
//!    the server-recomputed non-zero ambient replacement before final logout.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::presentation::PresentationEventKind;

const ACTIONS: &[&str] = &[
    "outside-snapshot",
    "city-entry",
    "blocked-refusal",
    "city-exit",
    "city-reentry",
    "fresh-admission",
    "final-logout",
];

struct Row {
    passed: std::collections::BTreeSet<&'static str>,
    started: std::time::Instant,
}

impl Row {
    fn pass(&mut self, action: &'static str) {
        if ACTIONS.contains(&action) && self.passed.insert(action) {
            evidence(&format!("W24_CHECK {action} PASS server-frames-validated"));
        }
    }

    fn finish(self, result: TestResult) {
        if let Err(error) = &result {
            evidence(&format!("W24_GAP step-failed observed={error}"));
        }
        let complete = ACTIONS.iter().all(|action| self.passed.contains(action));
        evidence(&format!(
            "e2e_all scenario=W24.full status={} seconds={:.3}",
            if result.is_ok() && complete {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row {
        passed: Default::default(),
        started: std::time::Instant::now(),
    };
    let result = verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(&mut row, clients))).await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err("step=journey observed=timeout-240s need=bounded-server-replies".into()),
    });
}

fn clock_snapshot(client: &mut DevClientSession) -> TestResult<u16> {
    use oteryn_protocol_oteryn::world_clock::{decode_world_clock, STATE_DOMAIN_WORLD_CLOCK, SNAPSHOT_TYPE_WORLD_CLOCK_V1};
    let snapshot = client.session_mut().gated_snapshot(STATE_DOMAIN_WORLD_CLOCK)
        .ok_or("step=world-clock-admission observed=missing-snapshot need=tibian-minute")?;
    if snapshot.snapshot_type != SNAPSHOT_TYPE_WORLD_CLOCK_V1 {
        return Err("step=world-clock-admission observed=wrong-snapshot-type".into());
    }
    decode_world_clock(&snapshot.payload).map(|clock| clock.tibian_minute)
        .map_err(|error| format!("step=world-clock-admission observed={error:?}").into())
}

type TimedCue = (u32, u16);

fn ambient(client: &mut DevClientSession, minute: &mut u16) -> Vec<TimedCue> {
    let mut cues = Vec::new();
    for event in client.take_events() {
        if let SessionEvent::WorldClock(ref delta) = event {
            *minute = delta.value.tibian_minute;
        }
        if let SessionEvent::Presentation(delta) = event {
            cues.extend(delta.value.into_iter().filter_map(|event| match event.kind {
                PresentationEventKind::AmbientSound { cue_id } => Some((cue_id, *minute)),
                _ => None,
            }));
        }
    }
    cues
}

fn zone(world: &BundleWorld, at: TilePos) -> Option<&str> {
    let table = &world.base().subareas;
    let (_, id) = table.resolve(at.x, at.y, at.floor)?;
    let key = table.areas.iter().find(|area| area.id == id)?.key.as_str();
    let key = key.strip_prefix("oteryn:content.area.")?;
    let city = key
        .strip_prefix("city.")
        .or_else(|| key.strip_prefix("subregion.")?.strip_suffix("_city"))?;
    matches!(
        city,
        "venore" | "thais" | "carlin" | "edron" | "kazordoon" | "port_hope"
    )
    .then_some(city)
}

fn city_replacement(city: &str, cues: &[TimedCue]) -> bool {
    let (day, night) = match city {
        "venore" => (23, 28),
        "thais" | "carlin" => (25, 28),
        "edron" => (3, 36),
        "kazordoon" => (15, 41),
        "port_hope" => (11, 8),
        _ => return false,
    };
    cues.last().is_some_and(|&(cue, minute)| {
        let daylight = (crate::world_clock::SUNRISE_START..crate::world_clock::SUNSET_START)
            .contains(&u32::from(minute));
        cue == if daylight { day } else { night }
    })
}

fn stopped(cues: &[TimedCue]) -> bool {
    cues.last().is_some_and(|&(cue, _)| cue == 0)
}

async fn wait_for_cue(
    client: &mut DevClientSession,
    minute: &mut u16,
    mut cues: Vec<TimedCue>,
    matches: impl Fn(&[TimedCue]) -> bool,
) -> TestResult<Vec<TimedCue>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    cues.extend(ambient(client, minute));
    while !matches(&cues) && tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(25)).await?;
        cues.extend(ambient(client, minute));
    }
    Ok(cues)
}

async fn step(client: &mut DevClientSession, minute: &mut u16, direction: StepDirection) -> TestResult<Vec<TimedCue>> {
    let before = client.world_spatial().actor_position;
    let reply = client.step(direction).await?;
    if reply.disposition != StepDisposition::Moved || reply.world_spatial_delta.is_none() {
        return Err(format!(
            "step=walking-command observed=disposition:{:?},delta:{} need=Moved+world-spatial-delta",
            reply.disposition,
            reply.world_spatial_delta.is_some()
        )
        .into());
    }
    if client.world_spatial().actor_position == before {
        return Err("step=walking-command observed=unchanged-position need=position-change".into());
    }
    Ok(ambient(client, minute))
}

async fn follow(client: &mut DevClientSession, minute: &mut u16, path: &[StepDirection]) -> TestResult<Vec<TimedCue>> {
    let mut cues = Vec::new();
    for &direction in path {
        cues.extend(step(client, minute, direction).await?);
    }
    Ok(cues)
}

fn blocked_edge(world: &BundleWorld, at: TilePos) -> Option<StepDirection> {
    [
        StepDirection::North,
        StepDirection::East,
        StepDirection::South,
        StepDirection::West,
    ]
    .into_iter()
    .find(|direction| destination(world, at, *direction).is_none())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let world = qualification_world(clients)?;
    let mut client = connect_character(clients, 0x24, 1).await?;
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::presentation::CAPABILITY_AMBIENT_SOUND_V1)
    {
        return Err(
            "step=admission observed=ambient-capability-not-selected need=capability-288".into(),
        );
    }
    let mut minute = clock_snapshot(&mut client)?;
    let from = tile(client.world_spatial().actor_position)?;
    let start_city = zone(world, from);
    let mut initial = wait_for_cue(&mut client, &mut minute, Vec::new(), |cues| {
        start_city.map_or_else(|| stopped(cues), |city| city_replacement(city, cues))
    }).await?;
    if let Some(city) = start_city {
        if !city_replacement(city, &initial) {
            return Err(format!(
                "step=starting-city-snapshot observed=ambient-cues:{initial:?} need={city}-cue-matching-observed-tibian-minute"
            ).into());
        }
        let path = route(world, from, |at| zone(world, at).is_none())
            .ok_or("step=outside-snapshot-route observed=no-city-exit need=reachable-outside-tile")?;
        let cues = follow(&mut client, &mut minute, &path).await?;
        let cues = wait_for_cue(&mut client, &mut minute, cues, stopped).await?;
        if !stopped(&cues) {
            return Err(format!("step=outside-snapshot-route observed=ambient-cues:{cues:?} need=cue-0").into());
        }
        let outside = client.world_spatial().actor_position;
        if client.session_mut().logout().await?.result
            != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        {
            return Err("step=outside-snapshot-logout observed=refused need=Accepted".into());
        }
        drop(client);
        client = connect_character(clients, 0x26, 1).await?;
        minute = clock_snapshot(&mut client)?;
        if client.world_spatial().actor_position != outside {
            return Err(format!(
                "step=outside-snapshot-admission observed={:?} need={outside:?}",
                client.world_spatial().actor_position
            ).into());
        }
        initial = wait_for_cue(&mut client, &mut minute, Vec::new(), stopped).await?;
    }
    if !stopped(&initial) {
        return Err(
            format!("step=outside-snapshot observed=ambient-cues:{initial:?} need=cue-0").into(),
        );
    }
    row.pass("outside-snapshot");

    let from = tile(client.world_spatial().actor_position)?;
    let into = route(world, from, |at| zone(world, at).is_some())
        .ok_or("step=city-entry observed=no-compiled-player-route need=reachable-supported-city")?;
    let city_cues = follow(&mut client, &mut minute, &into).await?;
    let city = zone(world, tile(client.world_spatial().actor_position)?)
        .ok_or("step=city-entry observed=outside-city-after-walking")?;
    let city_cues = wait_for_cue(&mut client, &mut minute, city_cues, |cues| city_replacement(city, cues)).await?;
    if !city_replacement(city, &city_cues) {
        return Err(format!(
            "step=city-entry observed=ambient-cues:{city_cues:?} need={city}-cue-matching-observed-tibian-minute"
        )
        .into());
    }
    row.pass("city-entry");

    let at = tile(client.world_spatial().actor_position)?;
    let to_edge = route(world, at, |candidate| {
        zone(world, candidate).is_some() && blocked_edge(world, candidate).is_some()
    })
    .ok_or("step=blocked-refusal observed=no-reachable-collision-edge need=city-boundary")?;
    follow(&mut client, &mut minute, &to_edge).await?;
    let direction = blocked_edge(world, tile(client.world_spatial().actor_position)?)
        .ok_or("step=blocked-refusal observed=edge-vanished need=blocked-direction")?;
    ambient(&mut client, &mut minute);
    let before = client.world_spatial().actor_position;
    let refusal = client.step(direction).await?;
    let refusal_cues = ambient(&mut client, &mut minute);
    if refusal.disposition != StepDisposition::Blocked
        || refusal.world_spatial_delta.is_some()
        || client.world_spatial().actor_position != before
        || !refusal_cues.is_empty()
    {
        return Err(format!("step=blocked-refusal observed=reply:{refusal:?},ambient:{refusal_cues:?} need=Blocked+no-delta+no-ambient").into());
    }
    row.pass("blocked-refusal");

    let at = tile(client.world_spatial().actor_position)?;
    let out = route(world, at, |candidate| zone(world, candidate).is_none())
        .ok_or("step=city-exit observed=no-compiled-player-route need=outside-city")?;
    let exit_cues = follow(&mut client, &mut minute, &out).await?;
    let exit_cues = wait_for_cue(&mut client, &mut minute, exit_cues, stopped).await?;
    if !stopped(&exit_cues) {
        return Err(
            format!("step=city-exit observed=ambient-cues:{exit_cues:?} need=cue-0").into(),
        );
    }
    row.pass("city-exit");

    let outside = tile(client.world_spatial().actor_position)?;
    let back = route(world, outside, |candidate| {
        zone(world, candidate).is_some()
    })
    .ok_or("step=city-reentry observed=no-compiled-player-route need=supported-city")?;
    let reentry_cues = follow(&mut client, &mut minute, &back).await?;
    let expected_position = client.world_spatial().actor_position;
    let city = zone(world, tile(expected_position)?)
        .ok_or("step=city-reentry observed=outside-city-after-walking")?;
    let reentry_cues = wait_for_cue(&mut client, &mut minute, reentry_cues, |cues| city_replacement(city, cues)).await?;
    if !city_replacement(city, &reentry_cues) {
        return Err(format!(
            "step=city-reentry observed=ambient-cues:{reentry_cues:?} need={city}-cue-matching-observed-tibian-minute"
        )
        .into());
    }
    row.pass("city-reentry");
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused need=Accepted".into());
    }

    let mut fresh = connect_character(clients, 0x25, 1).await?;
    let mut minute = clock_snapshot(&mut fresh)?;
    let fresh_cues = wait_for_cue(&mut fresh, &mut minute, Vec::new(), |cues| city_replacement(city, cues)).await?;
    if fresh.world_spatial().actor_position != expected_position
        || !city_replacement(city, &fresh_cues)
    {
        return Err(format!("step=fresh-admission observed=position:{:?},ambient:{fresh_cues:?} need=position:{expected_position:?}+{city}-cue-matching-observed-tibian-minute", fresh.world_spatial().actor_position).into());
    }
    row.pass("fresh-admission");
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=final-logout observed=refused need=Accepted".into());
    }
    row.pass("final-logout");
    Ok(())
}

fn qualification_world(clients: &SeamClients<'_>) -> TestResult<&'static BundleWorld> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journey_regression_requires_the_destination_city_cue() {
        assert!(city_replacement("edron", &[(3, 360)]));
        assert!(city_replacement("edron", &[(36, 1050)]));
        assert!(!city_replacement("edron", &[(25, 360)]));
        assert!(!city_replacement("edron", &[(36, 360)]));
        assert!(!city_replacement("edron", &[(3, 1050)]));
        assert!(!city_replacement("edron", &[(3, 360), (0, 360)]));
    }

    #[test]
    fn row_requires_the_complete_lifecycle() {
        assert_eq!(ACTIONS.len(), 7);
        assert!(ACTIONS.contains(&"blocked-refusal"));
        assert!(ACTIONS.contains(&"fresh-admission"));
    }
}
