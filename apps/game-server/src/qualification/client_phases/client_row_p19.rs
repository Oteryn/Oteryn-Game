//! Capture a live forge marker for the session replay test.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::appearance::CAPABILITY_FORGE_CREATURE_V1;

async fn logout(client: &mut DevClientSession) -> TestResult {
    let (result, detail, _, _) = logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P19 capture logout refused: {detail}").into());
    }
    Ok(())
}

fn marked_creatures(client: &mut DevClientSession) -> Vec<oteryn_dev_client::EntityRef> {
    client.session_mut().visible_appearances().into_iter()
        .filter(|entry| entry.entity.kind == EntityKind::Creature && entry.forge_stacks.is_some())
        .map(|entry| entry.entity.entity)
        .collect()
}

fn capture_marker(client: &mut DevClientSession, target: oteryn_dev_client::EntityRef) -> Option<[(&'static str, Vec<u8>); 3]> {
    let session = client.session_mut();
    let limit = oteryn_dev_client::native_session::REPLAY_DELTA_LIMIT;
    if session.raw_world_entity_deltas().len() >= limit || session.raw_appearance_deltas().len() >= limit {
        return None;
    }
    session.p19_capture_files(target)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let directory = required("E2E_ALL_CAPTURE_DIR")?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let mut client = connect_character(clients, 0x53, 0).await?;
    if !client.selected_capabilities().contains(&CAPABILITY_FORGE_CREATURE_V1) {
        return Err("forge creature capability was not selected".into());
    }
    let start = tile(client.world_spatial().actor_position)?;
    let mut spawns: std::collections::BTreeSet<_> = metadata.spawns.sources.iter()
        .flat_map(|source| source.points.iter().map(move |point| TilePos {
            x: point.x, y: point.y, floor: source.floor,
        }))
        .filter(|point| point.floor.abs_diff(start.floor) <= 2 &&
            point.x.abs_diff(start.x).max(point.y.abs_diff(start.y)) <= 160)
        .collect();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(330);
    let mut visits = 0;
    let files = loop {
        let marked = marked_creatures(&mut client);
        if let Some(files) = marked.iter().find_map(|target| capture_marker(&mut client, *target)) {
            break files;
        }
        if !marked.is_empty() {
            // Scout movement can evict the original chain. A fresh admission at
            // this position supplies a new wire snapshot with the live marker.
            let prior = client.join_snapshot().game_session_id;
            let position = client.world_spatial().actor_position;
            logout(&mut client).await?;
            drop(client);
            client = connect_character(clients, 0x54, 0).await?;
            if client.join_snapshot().game_session_id == prior || client.world_spatial().actor_position != position {
                return Err("P19 fresh capture admission did not preserve the scouting position".into());
            }
            let files = marked.into_iter().find_map(|target| capture_marker(&mut client, target))
                .ok_or("P19 marked creature did not retain a complete fresh replay chain")?;
            break files;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("P19 did not observe a forge marker after {visits} authored spawn visits").into());
        }
        let from = tile(client.world_spatial().actor_position)?;
        let near = |at: TilePos, point: TilePos| {
            at.floor == point.floor && at.x.abs_diff(point.x).max(at.y.abs_diff(point.y)) <= 3
        };
        // Exhaust this floor before following a drop that may have no return route.
        let goal = |at: TilePos| spawns.iter().any(|point| near(at, *point));
        let path = route_avoiding(world, from, goal, |at| at.floor == from.floor)
            .or_else(|| route(world, from, goal))
            .ok_or_else(|| format!("P19 no reachable unvisited forge marker site from {from:?} after {visits} visits, {} candidates left", spawns.len()))?;
        walk(&mut client, world, &path).await?;
        let reached = tile(client.world_spatial().actor_position)?;
        spawns.retain(|point| !near(reached, *point));
        visits += 1;
        client.service_liveness(Duration::from_millis(100)).await?;
    };
    for (name, bytes) in files {
        std::fs::write(Path::new(&directory).join(name), bytes)?;
    }
    logout(&mut client).await
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(390), verdict::observe(journey(clients))).await {
        Ok(Ok(())) => evidence("e2e_all scenario=P19.client status=PASS evidence=live-forge-marker-replay"),
        Ok(Err(error)) => evidence(&format!("e2e_all scenario=P19.client status=FAIL evidence={error}")),
        Err(_) => evidence("e2e_all scenario=P19.client status=FAIL evidence=timeout"),
    }
}
