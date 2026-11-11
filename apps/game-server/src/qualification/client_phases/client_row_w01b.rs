//! W01b client replay capture: a rat is killed (damage, removal and corpse deltas) and, after the
//! authored respawn interval, a replacement actor is present at the authored point. The session retains typed entity deltas
//! only, so every captured delta is the server-delivered `WorldSpatialEntitiesDelta` encoded with
//! the production encoder (the same convention as C09 and S04b), preceded by an entity snapshot of
//! the view at the segment start. The consumer is `Session::apply_state_delta` and
//! `Session::world_entities` in `crates/session`.
use super::*;
use oteryn_dev_client::{
    DevClientSession, EntityDetail, EntityRef, SessionEvent, WorldSpatialEntity,
};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::world_spatial_entities::{
    WorldSpatialEntitiesSnapshot, encode_world_spatial_entities_delta,
    encode_world_spatial_entities_snapshot,
};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

fn distance(a: ActorPosition, b: ActorPosition) -> u32 {
    a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn text_of(entity: Option<&WorldSpatialEntity>) -> String {
    let Some(entity) = entity else {
        return "absent".into();
    };
    let health = match entity.detail {
        EntityDetail::Actor { health_percent, .. } => health_percent.to_string(),
        _ => "-".into(),
    };
    format!(
        "{},{},{}|{health}",
        entity.position.x, entity.position.y, entity.position.floor
    )
}

fn plain(mut entity: WorldSpatialEntity) -> WorldSpatialEntity {
    if let EntityDetail::Object {
        item_definition_ref,
        quantity,
        ..
    } = entity.detail
    {
        entity.detail = EntityDetail::Object {
            item_definition_ref,
            quantity,
            item_handle: None,
        };
    }
    entity
}

/// An entity snapshot of the view at the segment start plus the entity deltas applied after it.
struct Segment {
    name: &'static str,
    target: EntityRef,
    snapshot: Vec<u8>,
    initial: String,
    deltas: Vec<Vec<u8>>,
    trace: Vec<String>,
}

impl Segment {
    fn begin(client: &DevClientSession, name: &'static str, target: EntityRef) -> TestResult<Self> {
        let view = client.world_entities().ok_or("entity view absent")?;
        let observation = client.world_spatial();
        let snapshot = encode_world_spatial_entities_snapshot(&WorldSpatialEntitiesSnapshot {
            content_generation: observation.content_generation,
            actor_position: observation.actor_position,
            own_identity: *view.own_identity(),
            entities: view.iter().copied().map(plain).collect(),
        })
        .map_err(|error| format!("encode snapshot: {error:?}"))?;
        Ok(Self {
            name,
            target,
            snapshot,
            initial: text_of(view.get(&target)),
            deltas: Vec::new(),
            trace: Vec::new(),
        })
    }

    fn record(&mut self, events: Vec<SessionEvent>) -> TestResult {
        for event in events {
            let SessionEvent::WorldSpatialEntities(delta) = event else {
                continue;
            };
            let mut value = delta.value;
            value.enter = value.enter.into_iter().map(plain).collect();
            value.update = value.update.into_iter().map(plain).collect();
            let line = if value.leave.contains(&self.target) {
                "absent".to_owned()
            } else if let Some(entity) = value
                .enter
                .iter()
                .chain(&value.update)
                .find(|entity| entity.entity == self.target)
            {
                text_of(Some(entity))
            } else {
                "same".to_owned()
            };
            self.deltas.push(
                encode_world_spatial_entities_delta(&value)
                    .map_err(|error| format!("encode delta: {error:?}"))?,
            );
            self.trace.push(line);
        }
        Ok(())
    }

    fn write(&self, expected: &mut String) -> TestResult {
        capture(&format!("w01b-{}-snapshot.bin", self.name), &self.snapshot)?;
        for (index, bytes) in self.deltas.iter().enumerate() {
            capture(&format!("w01b-{}-delta{index}.bin", self.name), bytes)?;
        }
        expected.push_str(&format!(
            "{n}.target={}:{}\n{n}.initial={}\n{n}.trace={}\n",
            hex(&self.target.identity),
            self.target.generation,
            self.initial,
            self.trace.join(";"),
            n = self.name
        ));
        Ok(())
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(420);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0x0c).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let here = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .filter(|entry| {
                    entry.kind == oteryn_dev_client::EntityKind::Creature
                        && entry.position.floor == here.floor
                })
                .min_by_key(|entry| distance(entry.position, here))
        })
        .copied()
        .ok_or("W01b client capture: no live rat")?;
    let target_at = tile(target.position)?;
    let creatures = &metadata.spawns.creatures;
    let (spawn, respawn_ms) = metadata
        .spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().filter_map(move |point| {
                creatures
                    .get(point.creature as usize)
                    .filter(|creature| creature.key == "oteryn:creature.rat")
                    .map(|_| {
                        (
                            TilePos {
                                x: point.x,
                                y: point.y,
                                floor: source.floor,
                            },
                            point.respawn_ms,
                        )
                    })
            })
        })
        .filter(|(point, _)| tile_distance(*point, target_at) <= 8)
        .min_by_key(|(point, _)| tile_distance(*point, tile(here).unwrap_or(target_at)))
        .ok_or("W01b client capture: no authored rat point near the live rat")?;

    // Kill: attack and record the damage, removal and corpse deltas.
    client.take_events();
    let mut kill = Segment::begin(&client, "kill", target.entity)?;
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("W01b client capture: attack refused: {attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut last = target.position;
    let corpse = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        let events = client.take_events();
        for event in &events {
            if let SessionEvent::WorldSpatialEntities(delta) = event
                && let Some(entity) = delta.value.update.iter().find(|entry| entry.entity == target.entity)
            {
                last = entity.position;
            }
        }
        kill.record(events)?;
        if let Some(corpse) = client.world_entities().and_then(|entities| {
            entities
                .iter()
                .find(|entity| entity.kind == oteryn_dev_client::EntityKind::Corpse && entity.position == last)
                .copied()
        }) && target_state(&client, target.entity).is_none()
        {
            break corpse;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("W01b client capture: no corpse and removal before the deadline".into());
        }
    };
    if !kill.trace.iter().any(|line| line.ends_with("|-") || line == "absent")
        || !kill.trace.iter().any(|line| {
            line.rsplit('|').next().and_then(|health| health.parse::<u32>().ok()).is_some_and(|health| health < 100)
        })
    {
        return Err("W01b client capture: kill lacked a damage delta or a removal delta".into());
    }

    // Respawn: leave interest, wait out the authored interval, return, require a new actor.
    let from = tile(client.world_spatial().actor_position)?;
    let away = route(world, from, |at| tile_distance(at, spawn) >= 20)
        .ok_or("W01b client capture: no route outside spawn interest")?;
    walk(&mut client, world, &away).await?;
    // Keep answering liveness while the interval runs out; a bare sleep lets
    // the server drop the session as unresponsive.
    let until = tokio::time::Instant::now() + Duration::from_millis(u64::from(respawn_ms) + 1_000);
    while tokio::time::Instant::now() < until {
        client.service_liveness(Duration::from_millis(500)).await?;
    }
    let back = route(world, tile(client.world_spatial().actor_position)?, |at| {
        tile_distance(at, spawn) <= 1
    })
    .ok_or("W01b client capture: no route back to the rat point")?;
    walk(&mut client, world, &back).await?;
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
    let respawn = Segment::begin(&client, "respawn", replacement.entity)?;
    capture("w01b-respawn-snapshot.bin", &respawn.snapshot)?;

    let mut expected = String::new();
    kill.write(&mut expected)?;
    expected.push_str(&format!(
        "corpse={}:{}\ncorpse.position={},{},{}\nspawn={},{},{}\nreplacement={}:{}\nreplacement.position={},{},{}\nkilled.absent=true\n",
        hex(&corpse.entity.identity),
        corpse.entity.generation,
        corpse.position.x,
        corpse.position.y,
        corpse.position.floor,
        spawn.x,
        spawn.y,
        spawn.floor,
        hex(&replacement.entity.identity),
        replacement.entity.generation,
        replacement.position.x,
        replacement.position.y,
        replacement.position.floor,
    ));
    capture("w01b-expected.txt", expected.as_bytes())?;
    Ok(expected)

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=W01b.client status=CAPTURED evidence={:?}",
            expected.replace('\n', ";")
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W01b.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W01b.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_preserves_zero_padding_and_high_bytes() {
        assert_eq!(hex(&[]), "");
        assert_eq!(hex(&[0, 15, 128, 255]), "000f80ff");
    }

    #[test]
    fn distance_handles_signed_coordinate_boundaries() {
        let a = ActorPosition {
            x: i32::MIN,
            y: 0,
            floor: 0,
        };
        assert_eq!(distance(a, a), 0);
        assert_eq!(distance(a, ActorPosition { x: i32::MAX, ..a }), u32::MAX);
    }
}
