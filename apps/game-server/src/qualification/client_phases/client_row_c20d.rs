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
    let result = row_complete(journey(clients, world, spawns)).await;
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
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(240);
    let mut client = connect_world(clients, 0xc2).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let result: TestResult = async {
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
            travel_to_spawn(&mut client, world, spawns, "oteryn:creature.poison_spider").await?;
            client.session_mut().attack_target(None).await?;
            let mut damaged = false;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
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
                    return Err(
                        "C20d capture did not observe damage followed by temple respawn".into(),
                    );
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
            )?;
            Ok(())
        }
        .await;
        let restore: TestResult = async {
            let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
                at == TOWN
            })
            .ok_or("C20d capture return route unavailable")?;
            walk(&mut client, world, &path).await
        }
        .await;
        result.and(restore)
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

fn capture(spatial: Vec<u8>, vitals: Vec<u8>, expected: String) -> TestResult {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let path = Path::new(&path);
    for (name, bytes) in [
        ("c20d-spatial.bin", spatial.as_slice()),
        ("c20d-vitals.bin", vitals.as_slice()),
        ("c20d-expected.txt", expected.as_bytes()),
    ] {
        std::fs::write(path.join(name), bytes)?;
    }

    Ok(())
}
