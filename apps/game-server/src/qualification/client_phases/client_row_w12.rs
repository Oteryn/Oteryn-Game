//! Live Cyclopedia map replies for the production client reducer replay.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::cyclopedia_map as wire;
use std::collections::BTreeSet;

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    std::fs::write(root.join(name), bytes)?;
    Ok(())
}

type Reply = oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse;

fn capture_reply(root: &Path, file: &str, reply: &Reply) -> TestResult {
    let mut frame = reply.command_id.to_le_bytes().to_vec();
    frame.extend_from_slice(&reply.server_sequence.to_le_bytes());
    frame.extend_from_slice(&reply.payload);
    capture(root, file, &frame)
}

async fn exchange(
    client: &mut DevClientSession,
    operation: u8,
) -> TestResult<(wire::MapResult, Reply)> {
    let (kind, payload) = wire::request(operation, 0).map_err(|_| "map request encoder")?;
    let reply = client
        .session_mut()
        .gated_command(wire::CAPABILITY_CYCLOPEDIA_MAP_V1, kind, &payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("map operation {operation} status={:?}", reply.status).into());
    }
    let state = wire::decode_result(&reply.payload)
        .map_err(|_| format!("operation {operation} result malformed"))?;
    Ok((state, reply))
}

fn exploration_tiles(
    world: &BundleWorld,
    origin: TilePos,
    subarea: u32,
) -> TestResult<BTreeSet<TilePos>> {
    use crate::domain::cyclopedia::{BasicArea, Point};
    let sampling = BasicArea::embedded().ok_or("W12 discovery sampling unavailable")?;
    let point = |at: TilePos| Point {
        x: at.x.into(),
        y: at.y.into(),
        floor: at.floor.into(),
    };
    // Cover the eligible terrain. The selected private points and their seed are
    // available only through the game; neither is read by qualification.
    let tiles = world
        .base()
        .subareas
        .runs
        .iter()
        .filter(|run| run.subarea == subarea)
        .flat_map(|run| {
            (run.begin..run.end).map(|x| TilePos {
                x,
                y: run.y,
                floor: run.floor,
            })
        })
        .filter(|at| {
            !point(*at).in_window(point(origin))
                && !world.facts().doors().at(*at)
                && world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|tile| tile.walkable())
                && world.enterable(*at)
        })
        .take(sampling.pool_limit)
        .collect::<BTreeSet<_>>();
    if tiles.len() < sampling.generated_count {
        return Err(format!(
            "W12 exploration terrain has only {} eligible tiles",
            tiles.len()
        )
        .into());
    }
    Ok(tiles)
}

fn occupant(
    client: &DevClientSession,
    at: TilePos,
) -> Option<oteryn_dev_client::WorldSpatialEntity> {
    use oteryn_dev_client::EntityKind;
    let entities = client.world_entities()?;
    entities
        .iter()
        .find(|entity| {
            &entity.entity.identity != entities.own_identity()
                && matches!(
                    entity.kind,
                    EntityKind::Creature | EntityKind::Npc | EntityKind::Player
                )
                && tile(entity.position).ok() == Some(at)
        })
        .copied()
}

async fn exploration_step(
    client: &mut DevClientSession,
    direction: StepDirection,
    from: TilePos,
    expected: TilePos,
) -> TestResult<oteryn_dev_client::StepOutcome> {
    let first = client.session_mut().step(direction).await?;
    if first.disposition != StepDisposition::Blocked
        || first.world_spatial_delta.is_some()
        || tile(client.world_spatial().actor_position)? != from
    {
        return Ok(first);
    }
    let Some(blocking) = occupant(client, expected) else {
        return Err(format!(
            "BLOCKER-SERVER W12 blocked without occupant {from:?} -> {expected:?}: {first:?}"
        )
        .into());
    };
    use oteryn_dev_client::EntityKind;
    if !matches!(blocking.kind, EntityKind::Creature | EntityKind::Npc) {
        return Err(format!(
            "W12 blocked by another player {:?}: {first:?}",
            blocking.entity
        )
        .into());
    }
    evidence(&format!(
        "W12 movement blocked occupant={:?} at={expected:?} frame={first:?}",
        blocking.entity
    ));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(100)).await?;
        let current = client.world_entities().and_then(|entities| {
            entities
                .iter()
                .find(|entity| entity.entity == blocking.entity)
        });
        if let Some(current) = current
            && tile(current.position)? != expected
            && occupant(client, expected).is_none()
            && tile(client.world_spatial().actor_position)? == from
        {
            evidence(&format!(
                "W12 occupant={:?} moved to {:?}; entering the same planned tile {expected:?}",
                blocking.entity, current.position
            ));
            return Ok(client.session_mut().step(direction).await?);
        }
    }
    Err(format!(
        "W12 occupant {:?} did not visibly leave {expected:?}: {first:?}",
        blocking.entity
    )
    .into())
}

async fn explore(
    client: &mut DevClientSession,
    world: &BundleWorld,
    subarea: u32,
    completion_percent: u8,
    mut pending: BTreeSet<TilePos>,
    mut observed: wire::MapResult,
    root: &Path,
) -> TestResult<wire::MapResult> {
    use crate::domain::cyclopedia::Point;
    let point = |at: TilePos| Point {
        x: at.x.into(),
        y: at.y.into(),
        floor: at.floor.into(),
    };
    let mut known = BTreeSet::new();
    for _ in 0..512 {
        let from = tile(client.world_spatial().actor_position)?;
        for &(x, y, floor) in &observed.visible {
            let at = TilePos {
                x,
                y,
                floor: -floor,
            };
            if !pending.contains(&at) && !known.contains(&at) {
                return Err(
                    format!("W12 revealed a point outside its terrain coverage: {at:?}").into(),
                );
            }
            known.insert(at);
        }
        pending.retain(|at| !point(*at).in_window(point(from)));
        let path = route_avoiding(
            world,
            from,
            |at| {
                if known.is_empty() {
                    pending.iter().any(|candidate| point(*candidate).in_window(point(at)))
                } else {
                    known.contains(&at)
                }
            },
            |at| world.base().subareas.resolve(at.x, at.y, at.floor)
                .is_some_and(|(_, id)| id == subarea)
                && (known.contains(&at) || occupant(client, at).is_none()),
        ).ok_or_else(|| format!(
            "W12 has no route to a revealed point or uncovered viewport from {from:?}: known={known:?} uncovered={}",
            pending.len()
        ))?;
        let direction = *path
            .first()
            .ok_or("W12 exploration planned an empty step")?;
        let expected =
            destination(world, from, direction).ok_or("W12 planned landing disappeared")?;
        let stepped = exploration_step(client, direction, from, expected).await?;
        let landed = tile(client.world_spatial().actor_position)?;
        if stepped.disposition != StepDisposition::Moved
            || stepped.world_spatial_delta.is_none()
            || landed != expected
        {
            return Err(format!(
                "W12 movement {from:?} -> {expected:?} landed {landed:?}: {stepped:?}"
            )
            .into());
        }
        verdict::Harness::global().progress("w12-exploration-step");
        let (next, reply) = exchange(client, 0).await?;
        let expected_found = observed.found + u8::from(known.remove(&landed));
        if next.code != 0
            || next.found != expected_found
            || (next.found > observed.found && next.revision <= observed.revision)
            || next.discovered != (next.found == 7)
        {
            return Err(format!(
                "W12 invalid movement discovery at {landed:?} expected_found={expected_found}: {observed:?} -> {next:?}"
            ).into());
        }
        if next.found == 6 && observed.found == 5 {
            capture_reply(root, "w12-progress.bin", &reply)?;
            capture(
                root,
                "w12-progress.txt",
                describe("progress", &next).as_bytes(),
            )?;
        }
        if next.discovered {
            if observed.found != 6
                || next.area_percent != completion_percent
                || !next.visible.is_empty()
            {
                return Err(format!(
                    "W12 incomplete completion evidence: {observed:?} -> {next:?}"
                )
                .into());
            }
            capture_reply(root, "w12-complete.bin", &reply)?;
            return Ok(next);
        }
        observed = next;
        tokio::time::sleep(Duration::from_millis(stepped.step_wait_ms.into())).await;
    }
    Err(format!("W12 exhausted its 512-step exploration budget: {observed:?}").into())
}

fn describe(prefix: &str, result: &wire::MapResult) -> String {
    format!(
        "{prefix}_code={}\n{prefix}_revision={}\n{prefix}_discovered={}\n{prefix}_found={}\n{prefix}_percent={}\n{prefix}_pool={}\n{prefix}_epoch={}\n{prefix}_improved={}\n{prefix}_visible={:?}\n",
        result.code,
        result.revision,
        result.discovered,
        result.found,
        result.area_percent,
        result.pool,
        result.epoch,
        result.improved,
        result.visible
    )
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0xfc).await?;
    let root = std::path::PathBuf::from(
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("missing capture directory")?,
    )
    .join("w12");
    std::fs::create_dir(&root)?;
    let world = load_world(
        std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?,
        &serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let origin = tile(client.world_spatial().actor_position)?;
    let (region, subarea) = world
        .base()
        .subareas
        .resolve(origin.x, origin.y, origin.floor)
        .ok_or("W12 origin has no admitted subarea")?;
    let region_size = world
        .base()
        .subareas
        .areas
        .iter()
        .filter(|area| area.parent == Some(region))
        .count();
    let completion_percent = u8::try_from(
        100usize
            .checked_div(region_size)
            .ok_or("W12 empty region")?,
    )?;
    let terrain = exploration_tiles(world, origin, subarea)?;
    let (query, initial_reply) = exchange(&mut client, 0).await?;
    if query.code != 0 || query.found != 0 || query.discovered || query.area_percent != 0 {
        return Err(format!("W12 expected a new exploration: {query:?}").into());
    }
    capture_reply(&root, "w12-query.bin", &initial_reply)?;
    let complete = explore(
        &mut client,
        world,
        subarea,
        completion_percent,
        terrain,
        query.clone(),
        &root,
    )
    .await?;
    let (find, find_reply) = exchange(&mut client, 3).await?;
    if find != complete {
        return Err(format!(
            "stationary FIND changed completed exploration: {complete:?} -> {find:?}"
        )
        .into());
    }
    capture_reply(&root, "w12-find.bin", &find_reply)?;
    let (start, start_reply) = exchange(&mut client, 1).await?;
    let mut expected_refusal = complete.clone();
    expected_refusal.code = 3;
    if start != expected_refusal {
        return Err(format!("legacy START did not refuse a completed area: {start:?}").into());
    }
    capture_reply(&root, "w12-start.bin", &start_reply)?;
    let refused = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_MAP_V1,
            wire::COMMAND_TYPE_CYCLOPEDIA_INTENT,
            &[1, 2, 0, 0, 0, 0, 0, 0, 0],
        )
        .await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!("malformed intent was not refused: {refused:?}").into());
    }
    capture_reply(&root, "w12-refused.bin", &refused)?;
    let (retained, _) = exchange(&mut client, 0).await?;
    if retained != complete {
        return Err(format!("W12 refusal changed discovery: {complete:?} -> {retained:?}").into());
    }
    let session = client.join_snapshot().game_session_id;
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("W12 first logout refused: {logout:?}").into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0xfd).await?;
    let fresh_session = fresh.join_snapshot().game_session_id;
    let landed = tile(fresh.world_spatial().actor_position)?;
    if fresh_session == session
        || world
            .base()
            .subareas
            .resolve(landed.x, landed.y, landed.floor)
            .is_none_or(|(_, id)| id != subarea)
    {
        return Err(
            format!("W12 re-admission did not restore the test subarea: {landed:?}").into(),
        );
    }
    let (after, after_reply) = exchange(&mut fresh, 0).await?;
    let mut expected_after = complete.clone();
    expected_after.revision = after.revision;
    if after != expected_after || after.revision < complete.revision {
        return Err(
            format!("W12 discovery changed across relog: {complete:?} -> {after:?}").into(),
        );
    }
    capture_reply(&root, "w12-relog.bin", &after_reply)?;
    let binding = format!(
        "session={session:?} sequence={} relog_session={fresh_session:?} relog_sequence={}",
        refused.server_sequence, after_reply.server_sequence
    );
    let expected = format!(
        "binding={binding}\n{}{}{}{}{}{}",
        describe("query", &query),
        std::fs::read_to_string(root.join("w12-progress.txt"))?,
        describe("complete", &complete),
        describe("find", &find),
        describe("start", &start),
        describe("relog", &after),
    );
    capture(&root, "w12-expected.txt", expected.as_bytes())?;
    Ok(binding)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(360), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_capture scenario=W12.client status=CAPTURED {observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W12.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W12.client status=FAIL evidence=timeout"),
    }
}
