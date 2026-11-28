//! W01b live journey and original inbound frame capture.
use super::full_row_w01b::{
    SPAWN, authored_spawn_cells, is_rat, logout, rat_points, walk_first_attempt,
};
use super::*;
use oteryn_dev_client::{
    DevClientSession, EntityDetail, EntityRef, SessionEvent, WorldSpatialEntity,
};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
fn capture(name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write as _;

    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .ok_or("W01b client capture: E2E_ALL_CAPTURE_DIR missing")?;
    let result = std::fs::File::create_new(Path::new(&root).join(name))
        .and_then(|mut file| file.write_all(bytes));
    result.map_err(|error| format!("W01b client capture: write {name}: {error}").into())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn text_of(entity: Option<&WorldSpatialEntity>) -> String {
    let Some(entity) = entity else {
        return "absent".into();
    };
    let health = match entity.detail {
        EntityDetail::Actor { health_percent, .. }
        | EntityDetail::ActorWithIcons { health_percent, .. } => health_percent.to_string(),
        _ => "-".into(),
    };
    format!(
        "{},{},{}|{health}",
        entity.position.x, entity.position.y, entity.position.floor
    )
}

fn record_target(events: Vec<SessionEvent>, target: EntityRef, trace: &mut Vec<String>) {
    for event in events {
        let SessionEvent::WorldSpatialEntities(delta) = event else {
            continue;
        };
        let value = delta.value;
        let line = if value.leave.contains(&target) {
            "absent".to_owned()
        } else if let Some(entity) = value
            .enter
            .iter()
            .chain(&value.update)
            .find(|entity| entity.entity == target)
        {
            text_of(Some(entity))
        } else {
            "same".to_owned()
        };
        trace.push(line);
    }
}

fn target_state(client: &DevClientSession, target: EntityRef) -> Option<WorldSpatialEntity> {
    client
        .world_entities()
        .and_then(|view| view.get(&target))
        .copied()
}

fn tile_distance(a: TilePos, b: TilePos) -> u16 {
    if a.floor == b.floor {
        a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
    } else {
        u16::MAX
    }
}

fn creature_at(client: &DevClientSession, at: TilePos) -> Option<WorldSpatialEntity> {
    client.world_entities().and_then(|entities| {
        entities
            .iter()
            .find(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Creature
                    && tile(entity.position).is_ok_and(|position| position == at)
            })
            .copied()
    })
}

// Stage only the dedicated hunter's initial position in the disposable node-boot database.
// The two real admissions and the final saved position still come from Game.
async fn seed_client_placement(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    entry: TilePos,
) -> TestResult {
    use sqlx::Connection as _;

    let frame_digest = world.frame_binding_digest(WorldId::decode(&clients.world)?);
    let map_digest = world.map_revision_digest();
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let seeded = sqlx::query(
        "INSERT INTO game_character_last_positions (
             character_id, world_id, tile_x, tile_y, tile_floor, coordinate_frame_digest,
             map_revision_digest, server_artifact_digest, character_lease_generation, write_sequence
         )
         SELECT r.character_id, r.world_id, $4, $5, $6, $7, $8,
                a.server_artifact_digest, 1, 1
         FROM game_character_roots r
         CROSS JOIN LATERAL (
             SELECT server_artifact_digest
             FROM game_content_activations
             WHERE world_id = r.world_id AND channel_id = encode($3, 'hex')::uuid
             ORDER BY activation_sequence DESC LIMIT 1
         ) a
         WHERE r.character_id = encode($1, 'hex')::uuid
           AND r.world_id = encode($2, 'hex')::uuid
           AND r.account_id = $9::uuid
           AND NOT EXISTS (
               SELECT 1 FROM game_durability_reconnect_sessions previous
               WHERE previous.character_id = r.character_id
           )",
    )
    .bind(clients.characters[0].as_slice())
    .bind(clients.world.as_slice())
    .bind(clients.channel.as_slice())
    .bind(i32::from(entry.x))
    .bind(i32::from(entry.y))
    .bind(-i16::from(entry.floor))
    .bind(frame_digest.as_slice())
    .bind(map_digest.as_slice())
    .bind(&clients.accounts[0])
    .execute(&mut database)
    .await?;
    if seeded.rows_affected() != 1 {
        return Err("W01b fixture did not place the dedicated hunter exactly once".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_W01B_ACCOUNT").await?;
    let clients = &dedicated.clients(clients);
    let spawn = SPAWN;
    let mut points = rat_points(&metadata.spawns).filter(|(point, _)| *point == spawn);
    let (_, respawn_ms) = points.next().ok_or("W01b authored Rat point absent")?;
    if points.next().is_some() {
        return Err("W01b authored Rat point ambiguous".into());
    }
    let spawn_cells = authored_spawn_cells(&metadata.spawns);
    let available = |at| !spawn_cells.contains(&at);
    let entry = TilePos {
        x: 32467,
        y: 32169,
        floor: -7,
    };
    if tile_distance(entry, spawn) < 20
        || !available(entry)
        || !world
            .base()
            .tile(entry.x, entry.y, entry.floor)
            .is_some_and(|cell| cell.walkable())
    {
        return Err(
            "W01b fixture entry is not a walkable authored ground cell outside Rat interest".into(),
        );
    }
    seed_client_placement(clients, world, entry).await?;
    let mut client = character_prep::w01b_rat_hunter("W01b.client", clients, 0x0c).await?;
    let joined_position = client.world_spatial().actor_position;
    let initial_position = tile(joined_position)?;
    if initial_position != entry {
        return Err(
            format!("W01b fixture entry expected={entry:?} observed={initial_position:?}").into(),
        );
    }
    evidence(&format!(
        "W01B_CLIENT_PLACEMENT row=W01b.client position={entry:?}"
    ));
    let approach = route_avoiding(
        world,
        initial_position,
        |at| {
            tile_distance(at, spawn) == 1
                && world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|cell| !cell.protection_zone())
        },
        available,
    )
    .filter(|route| !route.is_empty())
    .ok_or("W01b initial route absent")?;
    walk_first_attempt(&mut client, world, &approach).await?;
    let target = creature_at(&client, spawn).ok_or("W01b authored cell has no live Rat")?;
    if !is_rat(&target)
        || !matches!(
            target.detail,
            EntityDetail::Actor {
                health_percent: 100,
                ..
            } | EntityDetail::ActorWithIcons {
                health_percent: 100,
                ..
            }
        )
    {
        return Err(format!("W01b expected a full-health Rat before attack: {target:?}").into());
    }

    let prior_corpses: Vec<_> = client
        .world_entities()
        .ok_or("W01b entity view absent")?
        .iter()
        .filter(|entity| entity.kind == oteryn_dev_client::EntityKind::Corpse)
        .map(|entity| entity.entity)
        .collect();

    // Kill: attack and record the damage, removal and corpse deltas.
    client.take_events();
    let before = client.last_server_sequence();
    let initial = text_of(Some(&target));
    let mut trace = Vec::new();
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("W01b client capture: attack refused: {attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut last = target.position;
    let corpse = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        let events = client.take_events();
        for event in &events {
            if let SessionEvent::WorldSpatialEntities(delta) = event
                && let Some(entity) = delta
                    .value
                    .update
                    .iter()
                    .find(|entry| entry.entity == target.entity)
            {
                last = entity.position;
            }
        }
        record_target(events, target.entity, &mut trace);
        if let Some(corpse) = client.world_entities().and_then(|entities| {
            entities
                .iter()
                .find(|entity| {
                    entity.kind == oteryn_dev_client::EntityKind::Corpse
                        && entity.position == last
                        && !prior_corpses.contains(&entity.entity)
                })
                .copied()
        }) && target_state(&client, target.entity).is_none()
        {
            break corpse;
        }
        if tokio::time::Instant::now() >= deadline {
            let live = target_state(&client, target.entity);
            return Err(format!(
                "W01b client capture: no corpse and removal before the deadline: last={last:?} target={live:?} attacker={:?}",
                client.world_spatial().actor_position
            )
            .into());
        }
    };
    let killed = client.last_server_sequence();
    if !trace.iter().any(|line| line == "absent")
        || !trace.iter().any(|line| {
            line.rsplit('|')
                .next()
                .and_then(|health| health.parse::<u32>().ok())
                .is_some_and(|health| (1..100).contains(&health))
        })
    {
        return Err("W01b client capture: kill lacked a damage delta or a removal delta".into());
    }

    // Respawn: leave interest, wait out the authored interval, return, require a new actor.
    let from = tile(client.world_spatial().actor_position)?;
    let away = route_avoiding(world, from, |at| tile_distance(at, spawn) >= 20, available)
        .filter(|route| !route.is_empty())
        .ok_or("W01b client capture: no route outside spawn interest")?;
    walk_first_attempt(&mut client, world, &away).await?;
    // Keep answering liveness while the interval runs out; a bare sleep lets
    // the server drop the session as unresponsive.
    let until = tokio::time::Instant::now() + Duration::from_millis(u64::from(respawn_ms) + 1_000);
    verdict::Harness::global().bounded_wait(until, "bounded-observation");
    while tokio::time::Instant::now() < until {
        client.service_liveness(Duration::from_millis(500)).await?;
    }
    let back = route_avoiding(
        world,
        tile(client.world_spatial().actor_position)?,
        |at| tile_distance(at, spawn) == 1,
        available,
    )
    .filter(|route| !route.is_empty())
    .ok_or("W01b client capture: no route back to the rat point")?;
    walk_first_attempt(&mut client, world, &back).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let replacement = loop {
        if let Some(entity) = creature_at(&client, spawn)
            && entity.entity != target.entity
        {
            break entity;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("W01b client capture: no replacement at the authored point".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    };
    if target_state(&client, target.entity).is_some() {
        return Err("W01b client capture: killed actor returned".into());
    }
    if !is_rat(&replacement) {
        return Err("W01b client capture: replacement at the authored point is not a rat".into());
    }
    let returned = client.last_server_sequence();
    let first_session = client.join_snapshot().game_session_id;
    let mut expected = format!(
        "kill.target={}:{}\nkill.initial={initial}\nkill.trace={}\nbefore={before}\nattack.sequence={}\nkilled={killed}\nreturned={returned}\nfirst.session={}\n",
        hex(&target.entity.identity),
        target.entity.generation,
        trace.join(";"),
        attack.result_server_sequence,
        hex(first_session.as_bytes()),
    );
    expected.push_str(&format!(
        "corpse={}:{}\ncorpse.position={},{},{}\nspawn={},{},{}\nreplacement={}:{}\n",
        hex(&corpse.entity.identity),
        corpse.entity.generation,
        corpse.position.x,
        corpse.position.y,
        corpse.position.floor,
        spawn.x,
        spawn.y,
        // TilePos uses native negative floors. The replay compares ActorPosition wire floors.
        -i16::from(spawn.floor),
        hex(&replacement.entity.identity),
        replacement.entity.generation,
    ));
    // Leave the replacement behind before waiting for the combat lock. Use the same
    // authored route rule as the first departure and prove the Rat left interest.
    let departure = route_avoiding(
        world,
        tile(client.world_spatial().actor_position)?,
        |at| tile_distance(at, spawn) >= 20,
        available,
    )
    .filter(|route| !route.is_empty())
    .ok_or("W01b no logout route outside spawn interest")?;
    walk_first_attempt(&mut client, world, &departure).await?;
    if target_state(&client, replacement.entity).is_some() {
        return Err("W01b replacement is still in interest at the logout position".into());
    }
    let position = client.world_spatial().actor_position;
    if tile(position)? == initial_position {
        return Err("W01b logout position must differ from initial placement".into());
    }
    let logout_sequence = logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0x0d, 0).await?;
    let fresh_session = fresh.join_snapshot().game_session_id;
    if fresh_session == first_session
        || fresh.world_spatial().actor_position != position
        || target_state(&fresh, target.entity).is_some()
        || target_state(&fresh, replacement.entity).is_some()
    {
        return Err("W01b relog did not restore the saved position outside spawn interest".into());
    }
    let check = route_avoiding(
        world,
        tile(position)?,
        |at| tile_distance(at, spawn) == 1,
        available,
    )
    .filter(|route| !route.is_empty())
    .ok_or("W01b restored position has no route to spawn")?;
    walk_first_attempt(&mut fresh, world, &check).await?;
    let restored_rat = target_state(&fresh, replacement.entity)
        .ok_or("W01b replacement missing after returning from the restored position")?;
    if !is_rat(&restored_rat) || target_state(&fresh, target.entity).is_some() {
        return Err("W01b relog changed the observed Rat identity".into());
    }
    let fresh_sequence = fresh.last_server_sequence();
    expected.push_str(&format!(
        "fresh.sequence={fresh_sequence}\nlogout.sequence={}\nfresh.session={}\nposition={},{},{}\njoined.position={},{},{}\n",
        logout_sequence, hex(fresh_session.as_bytes()),
        position.x, position.y, position.floor,
        joined_position.x, joined_position.y, joined_position.floor,
    ));
    Ok(expected)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        evidence("e2e_all scenario=W01b.client status=FAIL evidence=missing-capture-directory");
        return;
    }
    match verdict::indexed_timeout(
        Duration::from_secs(420),
        verdict::observe(async {
            let (outcome, frames) =
                oteryn_dev_client::native_session::capture_server_frames(journey(clients)).await;
            let expected = outcome?;
            capture(
                "w01b-server-frames.bin",
                &frames.ok_or("W01b server frame capture overflowed")?,
            )?;
            capture("w01b-expected.txt", expected.as_bytes())?;
            TestResult::Ok(expected)
        }),
    )
    .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_capture scenario=W01b.client status=CAPTURED evidence={:?}",
            expected.replace('\n', ";")
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W01b.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W01b.client status=FAIL evidence=timeout"),
    }
}
