//! W24 full-row player scenario, derived from the matrix/audit and the native
//! ambient owner in `gameplay_transport/ambient_sounds.rs`:
//! 1. Admit the provisioned character and require the private ambient snapshot
//!    delta (`AmbientSound { cue_id: 0 }` outside a supported city).
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
    let result = row_complete_with_timeout(Duration::from_secs(240), journey(&mut row, clients)).await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err("step=journey observed=timeout-240s need=bounded-server-replies".into()),
    });
}

fn ambient(client: &mut DevClientSession) -> Vec<u32> {
    client
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            SessionEvent::Presentation(delta) => delta.value.into_iter().find_map(|event| {
                if let PresentationEventKind::AmbientSound { cue_id } = event.kind {
                    Some(cue_id)
                } else {
                    None
                }
            }),
            _ => None,
        })
        .collect()
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

async fn step(client: &mut DevClientSession, direction: StepDirection) -> TestResult<Vec<u32>> {
    let before = client.world_spatial().actor_position;
    let reply = client.step_retrying(direction).await?;
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
    Ok(ambient(client))
}

async fn follow(client: &mut DevClientSession, path: &[StepDirection]) -> TestResult<Vec<u32>> {
    let mut cues = Vec::new();
    for &direction in path {
        cues.extend(step(client, direction).await?);
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
    let mut client = row_connect_character(clients, 0x24, 1).await?;
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::presentation::CAPABILITY_AMBIENT_SOUND_V1)
    {
        return Err(
            "step=admission observed=ambient-capability-not-selected need=capability-288".into(),
        );
    }
    let initial = ambient(&mut client);
    if !initial.contains(&0) {
        return Err(
            format!("step=outside-snapshot observed=ambient-cues:{initial:?} need=cue-0").into(),
        );
    }
    row.pass("outside-snapshot");

    let from = tile(client.world_spatial().actor_position)?;
    let into = route(world, from, |at| zone(world, at).is_some())
        .ok_or("step=city-entry observed=no-compiled-player-route need=reachable-supported-city")?;
    let city_cues = follow(&mut client, &into).await?;
    if !city_cues.iter().any(|cue| *cue != 0) {
        return Err(format!(
            "step=city-entry observed=ambient-cues:{city_cues:?} need=nonzero-city-replacement"
        )
        .into());
    }
    row.pass("city-entry");

    let at = tile(client.world_spatial().actor_position)?;
    let to_edge = route(world, at, |candidate| {
        zone(world, candidate).is_some() && blocked_edge(world, candidate).is_some()
    })
    .ok_or("step=blocked-refusal observed=no-reachable-collision-edge need=city-boundary")?;
    follow(&mut client, &to_edge).await?;
    let direction = blocked_edge(world, tile(client.world_spatial().actor_position)?)
        .ok_or("step=blocked-refusal observed=edge-vanished need=blocked-direction")?;
    client.take_events();
    let before = client.world_spatial().actor_position;
    let refusal = client.step_retrying(direction).await?;
    let refusal_cues = ambient(&mut client);
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
    let exit_cues = follow(&mut client, &out).await?;
    if !exit_cues.contains(&0) {
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
    let reentry_cues = follow(&mut client, &back).await?;
    let expected_position = client.world_spatial().actor_position;
    if !reentry_cues.iter().any(|cue| *cue != 0) {
        return Err(format!(
            "step=city-reentry observed=ambient-cues:{reentry_cues:?} need=nonzero-city-replacement"
        )
        .into());
    }
    row.pass("city-reentry");
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused need=Accepted".into());
    }

    let mut fresh = row_connect_character(clients, 0x25, 1).await?;
    let fresh_cues = ambient(&mut fresh);
    if fresh.world_spatial().actor_position != expected_position
        || !fresh_cues.iter().any(|cue| *cue != 0)
    {
        return Err(format!("step=fresh-admission observed=position:{:?},ambient:{fresh_cues:?} need=position:{expected_position:?}+nonzero-replacement", fresh.world_spatial().actor_position).into());
    }
    row.pass("fresh-admission");
    if fresh.row_logout().await?.result
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
    fn row_requires_the_complete_lifecycle() {
        assert_eq!(ACTIONS.len(), 7);
        assert!(ACTIONS.contains(&"blocked-refusal"));
        assert!(ACTIONS.contains(&"fresh-admission"));
    }
}
