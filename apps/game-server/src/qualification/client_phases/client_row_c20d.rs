//! C20d client replay capture: temple position and restored vitals.

use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_dev_client::native_session::{Admission, qualification_replay};

pub(super) async fn run_packet(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) {
    let started = std::time::Instant::now();
    // Budget for the live walk, bounded damage observation and a fresh admission.
    let result = verdict::indexed_timeout(
        Duration::from_secs(450),
        verdict::observe(qualification_replay::capture_inbound(journey(
            clients, world, spawns,
        ))),
    )
    .await;
    let result = result.unwrap_or_else(|_| Err("C20d client capture timed out".into()));
    if let Err(error) = &result {
        evidence(&format!("C20D_CLIENT_GAP {error}"));
    }
    evidence(&format!(
        "e2e_all scenario=C20d.client status={} seconds={:.3}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64()
    ));
}

async fn journey(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    let fixture = character_prep::dedicated_character(clients, "NODE_BOOT_C20D_ACCOUNT").await?;
    let scoped = fixture.clients(clients);
    let mut client = connect_character(&scoped, 0xc2, 0).await?;
    let result: TestResult<_> = async {
        let max_health = client
            .actor_vitals()
            .ok_or("C20d admission vitals absent")?
            .max_health;
        let home = world
            .base()
            .subareas
            .default_town
            .and_then(|id| world.base().subareas.town(id))
            .and_then(|town| town.temple)
            .ok_or("C20d capture has no configured home temple")?;
        let temple = oteryn_protocol_oteryn::world_spatial::ActorPosition {
            x: i32::from(home.x),
            y: i32::from(home.y),
            floor: -i16::from(home.floor),
        };
        if client.world_spatial().actor_position == temple {
            return Err("C20d encounter must start away from the temple".into());
        }
        approach_spawn(&mut client, world, spawns).await?;
        let encounter = client.world_spatial().actor_position;
        if encounter == temple {
            return Err("C20d lethal encounter reached the temple before death".into());
        }
        client.session_mut().attack_target(None).await?;
        let mut damaged = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
        verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
        loop {
            client.service_liveness(Duration::from_millis(100)).await?;
            for event in client.take_events() {
                if let SessionEvent::ActorVitals(delta) = event {
                    damaged |= delta.value.health < delta.value.max_health;
                }
            }
            if damaged
                && client.world_spatial().actor_position == temple
                && client.actor_vitals().is_some_and(|vitals| {
                    vitals.health == max_health && vitals.max_health == max_health
                })
            {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(
                    "C20d capture did not observe damage followed by temple respawn".into(),
                );
            }
        }
        let vitals = *client.actor_vitals().ok_or("C20d respawn vitals absent")?;
        evidence(&format!(
            "C20D_TEMPLE_RESPAWN encounter={encounter:?} temple={temple:?} health={} max_health={}",
            vitals.health, vitals.max_health
        ));
        Ok((temple, vitals, client.last_server_sequence()))
    }
    .await;
    let capture = qualification_replay::finish_capture();
    let prior = client.join_snapshot().game_session_id;
    let released = logout(&mut client).await;
    let (temple, vitals, sequence) = result.and_then(|proof| released.map(|()| proof))?;
    drop(client);
    let mut fresh = connect_character(&scoped, 0xc2, 0).await?;
    if fresh.join_snapshot().game_session_id == prior
        || fresh.world_spatial().actor_position != temple
        || !fresh.actor_vitals().is_some_and(|saved| {
            saved.health == vitals.max_health && saved.max_health == vitals.max_health
        })
    {
        return Err("C20d fresh admission did not preserve the temple and restored health".into());
    }
    evidence(&format!(
        "C20D_TEMPLE_RELOG temple={temple:?} health={} max_health={}",
        vitals.health, vitals.max_health
    ));
    logout(&mut fresh).await?;
    let capture = capture.ok_or("C20d inbound capture missing or exceeded its bound")?;
    let external = std::env::var_os("E2E_ALL_CAPTURE_DIR");
    let file_name = format!("c20d-{}.wire", uuid_text(prior.as_bytes()));
    let path = external
        .as_ref()
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from)
        .join(file_name);
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?;
    use std::io::Write;
    file.write_all(&capture)?;
    drop(file);
    let replay = std::fs::read(&path);
    if external.is_none() {
        std::fs::remove_file(&path)?;
    }
    qualification_replay::replay_respawn(
        replay?,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(&scoped.characters[0])?,
            // The local replay stream discards outbound bytes.
            admission_material: b"c20d-offline-replay",
            client_build_id: "oteryn-dev-client/world-qualification",
            supported_capabilities: oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(20),
        },
        prior,
        sequence,
        temple,
        vitals.max_health,
    )
    .await
    .map_err(|error| error.to_string())?;
    evidence(&format!(
        "C20D_ORIGINAL_REPLAY session={} sequence={sequence}",
        uuid_text(prior.as_bytes())
    ));
    Ok(())
}

async fn approach_spawn(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let points: Vec<_> = spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().filter_map(move |point| {
                spawns
                    .creatures
                    .get(point.creature as usize)
                    .is_some_and(|entry| entry.key == "oteryn:creature.poison_spider")
                    .then_some((source.floor, point.x, point.y))
            })
        })
        .collect();
    let path = route(world, from, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
            && points.iter().any(|&(floor, x, y)| {
                at.floor == floor && at.x.abs_diff(x) <= 1 && at.y.abs_diff(y) <= 1
            })
    })
    .ok_or("C20d no reachable authored poison spider spawn")?;
    for direction in path {
        step_exact(client, world, direction).await?;
    }
    let origin = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .filter(|entity| {
                    entity.kind == oteryn_dev_client::EntityKind::Creature
                        && entity.position.floor == origin.floor
                })
                .min_by_key(|entity| {
                    entity
                        .position
                        .x
                        .abs_diff(origin.x)
                        .max(entity.position.y.abs_diff(origin.y))
                })
                .map(|entity| entity.entity)
        })
        .ok_or("C20d authored spawn has no visible creature")?;
    for _ in 0..32 {
        let current = client
            .world_entities()
            .and_then(|entities| entities.get(&target))
            .ok_or("C20d lost sight of the encounter creature")?;
        let target_tile = tile(current.position)?;
        let here = tile(client.world_spatial().actor_position)?;
        let approach = route(world, here, |at| {
            at.floor == target_tile.floor
                && at
                    .x
                    .abs_diff(target_tile.x)
                    .max(at.y.abs_diff(target_tile.y))
                    == 1
        })
        .ok_or("C20d encounter creature is unreachable")?;
        let Some(&direction) = approach.first() else {
            evidence(&format!(
                "C20D_ENCOUNTER entity={target:?} player={here:?} creature={target_tile:?}"
            ));
            return Ok(());
        };
        step_exact(client, world, direction).await?;
    }
    Err("C20d did not reach the exact encounter creature".into())
}

async fn step_exact(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    direction: StepDirection,
) -> TestResult {
    let before = client.world_spatial().actor_position;
    let expected =
        destination(world, tile(before)?, direction).ok_or("C20d invalid planned step")?;
    let outcome = client.step(direction).await?;
    let outcome = if outcome.disposition == StepDisposition::Blocked {
        if client.world_spatial().actor_position != before {
            return Err("C20d blocked step changed position".into());
        }
        let occupant = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entity| {
                    entity.entity.identity != *entities.own_identity()
                        && tile(entity.position).ok() == Some(expected)
                })
            })
            .ok_or_else(|| {
                format!(
                    "BLOCKER-SERVER C20d blocked tile without an occupant: {expected:?} {outcome:?}"
                )
            })?;
        if occupant.kind != oteryn_dev_client::EntityKind::Creature {
            return Err(format!(
                "C20d planned step blocked by {:?} at {expected:?}",
                occupant.entity
            )
            .into());
        }
        let blocker = occupant.entity;
        evidence(&format!(
            "C20D_BLOCKED entity={blocker:?} tile={expected:?} sequence={}",
            outcome.result_server_sequence
        ));
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            client.service_liveness(Duration::from_millis(100)).await?;
            let current = client
                .world_entities()
                .and_then(|entities| entities.get(&blocker))
                .ok_or("C20d lost sight of the exact blocking creature")?;
            if tile(current.position)? != expected {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("C20d creature {blocker:?} did not clear {expected:?}").into());
            }
        }
        client.step(direction).await?
    } else {
        outcome
    };
    if outcome.disposition != StepDisposition::Moved
        || outcome
            .world_spatial_delta
            .as_ref()
            .map(|delta| tile(delta.value.actor_position))
            .transpose()?
            != Some(expected)
        || tile(client.world_spatial().actor_position)? != expected
    {
        return Err(
            format!("C20d step from {before:?} to {expected:?} failed: {outcome:?}").into(),
        );
    }
    verdict::Harness::global().progress("walk");
    if outcome.step_wait_ms != 0 {
        client
            .service_liveness(Duration::from_millis(u64::from(outcome.step_wait_ms)))
            .await?;
    }
    client.take_events();
    Ok(())
}

async fn logout(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C20d capture logout refused: {reply:?}").into());
    }
    Ok(())
}
