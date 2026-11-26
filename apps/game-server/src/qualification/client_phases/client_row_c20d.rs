//! C20d client replay capture: temple position and restored vitals.

use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::{actor_spell, world_spatial};

pub(super) async fn run_packet(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients, world, spawns))).await;
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
        travel_to_spawn(&mut client, world, spawns, "oteryn:creature.poison_spider").await?;
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
                && client
                    .actor_vitals()
                    .is_some_and(|vitals| vitals.health == vitals.max_health)
            {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("C20d capture did not observe damage followed by temple respawn".into());
            }
        }
        let vitals = *client.actor_vitals().ok_or("C20d respawn vitals absent")?;
        capture(
            world_spatial::encode_world_spatial(client.world_spatial()),
            actor_spell::encode_actor_vitals(&vitals).map_err(|error| format!("{error:?}"))?,
            format!(
                "{} {} {} {} {}\n",
                temple.x, temple.y, temple.floor, vitals.health, vitals.max_health
            ),
        );
        Ok((temple, vitals.max_health))
    }
    .await;
    let prior = client.join_snapshot().game_session_id;
    let released = logout(&mut client).await;
    let (temple, max_health) = result.and_then(|proof| released.map(|()| proof))?;
    drop(client);
    let mut fresh = connect_character(&scoped, 0xc2, 0).await?;
    if fresh.join_snapshot().game_session_id == prior
        || fresh.world_spatial().actor_position != temple
        || !fresh.actor_vitals().is_some_and(|vitals| {
            vitals.health == max_health && vitals.max_health == max_health
        })
    {
        return Err("C20d fresh admission did not preserve the temple and restored health".into());
    }
    logout(&mut fresh).await
}

fn capture(spatial: Vec<u8>, vitals: Vec<u8>, expected: String) {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    let path = Path::new(&path);
    for (name, bytes) in [
        ("c20d-spatial.bin", spatial.as_slice()),
        ("c20d-vitals.bin", vitals.as_slice()),
        ("c20d-expected.txt", expected.as_bytes()),
    ] {
        if let Err(error) = std::fs::write(path.join(name), bytes) {
            evidence(&format!("C20D_CAPTURE_WRITE_ERROR file={name} error={error}"));
        }
    }
}

async fn logout(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let (result, detail, _, _) = logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C20d capture logout refused: {detail}").into());
    }
    Ok(())
}
