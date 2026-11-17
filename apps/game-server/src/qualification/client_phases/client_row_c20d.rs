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

pub(super) fn respawn_matches(damaged: bool, temple: bool, healthy: bool, before: Option<[u8; 16]>, after: Option<[u8; 16]>) -> bool {
    damaged && temple && healthy && after.is_some() && after != before
}

pub(super) async fn death_occurrence(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<Option<[u8; 16]>> {
    use oteryn_protocol_oteryn::death_protection as wire;
    let reply = client.session_mut().gated_command(
        wire::CAPABILITY_DEATH_PROTECTION_V1, wire::COMMAND_TYPE_DEATH_PROTECTION_QUERY, &[],
    ).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("death occurrence query refused: {:?}", reply.status).into());
    }
    let receipt = wire::DeathProtectionReply::decode(&reply.payload)
        .ok_or("death occurrence reply was malformed")?;
    Ok(receipt.0.map(|death| death.occurrence))
}

async fn journey(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    let mut client = connect_world(clients, 0xc2).await?;
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
        let before_death = death_occurrence(&mut client).await?;
        travel_to_spawn(&mut client, world, spawns, "oteryn:creature.poison_spider").await?;
        client.session_mut().attack_target(None).await?;
        if client.world_spatial().actor_position == temple {
            return Err("C20d lethal encounter starts at the respawn temple".into());
        }
        client.take_events();
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
            let at_temple = client.world_spatial().actor_position == temple;
            let healthy = client.actor_vitals().is_some_and(|vitals| vitals.health > 0 && vitals.health == vitals.max_health);
            if damaged && at_temple && healthy {
                let after_death = death_occurrence(&mut client).await?;
                if respawn_matches(damaged, at_temple, healthy, before_death, after_death) {
                    break;
                }
                return Err("C20d temple arrival has no new committed death occurrence".into());
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
        Ok(())
    }
    .await;
    let restore: TestResult = async {
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| at == TOWN)
            .ok_or("C20d capture return route unavailable")?;
        walk(&mut client, world, &path).await
    }
    .await;
    result.and(restore).and(logout(&mut client).await)
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
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C20d capture logout refused: {reply:?}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respawn_requires_a_new_death_occurrence() {
        assert!(!respawn_matches(true, true, true, None, None));
        assert!(!respawn_matches(true, true, true, Some([1; 16]), Some([1; 16])));
        assert!(respawn_matches(true, true, true, Some([1; 16]), Some([2; 16])));
        assert!(!respawn_matches(true, false, true, Some([1; 16]), Some([2; 16])));
    }
}
