//! Client replay capture for SCENARIO_MATRIX M10.
//!
//! The payload owner is `oteryn-session`: admission applies the initial value in
//! `crates/session/src/snapshot_state.rs::SnapshotState::decode`, pushed drowning deltas pass
//! through `crates/session/src/domain_dispatch.rs::Session::apply_delta`, and the visible value
//! is returned by `crates/session/src/session_access.rs::Session::actor_vitals`. Neither the
//! client-ui nor client-runtime feature directories own another ActorVitals reducer.
use super::*;
use crate::map::view::MapFacts;
use oteryn_dev_client::{DevClientSession, SessionEvent};

const UNDERWATER: [u16; 12] = [
    5404, 5405, 5406, 5407, 5408, 5409, 5743, 5764, 8755, 8756, 8757, 9291,
];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await;
    match result {
        Ok(Ok(())) => evidence("SEAM_EVIDENCE e2e_all scenario=M10.client status=PASS evidence=real-drowning-ActorVitals-payload-captured"),
        Ok(Err(error)) => evidence(&format!("SEAM_EVIDENCE e2e_all scenario=M10.client status=FAIL evidence={error}")),
        Err(_) => evidence("SEAM_EVIDENCE e2e_all scenario=M10.client status=FAIL evidence=timeout-waiting-for-drowning-payload"),
    }
}

fn is_underwater(world: &BundleWorld, at: TilePos) -> bool {
    world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| {
        tile.ground()
            .and_then(|(ordinal, id)| world.facts().base_entry(at, ordinal, id))
            .is_some_and(|entry| UNDERWATER.contains(&entry.appearance_id))
    })
}

async fn drown_delta(client: &mut DevClientSession, before: u32) -> TestResult<oteryn_dev_client::AppliedDelta<oteryn_protocol_oteryn::actor_spell::ActorVitals>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        if let Some(delta) = client.take_events().into_iter().find_map(|event| match event {
            SessionEvent::ActorVitals(delta) if delta.value.health < before => Some(delta),
            _ => None,
        }) {
            return Ok(delta);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("no drowning ActorVitals delta".into());
        }
    }
}

fn capture(initial: &oteryn_protocol_oteryn::actor_spell::ActorVitals, delta: &oteryn_dev_client::AppliedDelta<oteryn_protocol_oteryn::actor_spell::ActorVitals>) {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    let result = (|| -> TestResult {
        let path = Path::new(&path);
        std::fs::write(path.join("m10-snapshot.bin"), oteryn_protocol_oteryn::actor_spell::encode_actor_vitals(initial).map_err(|error| format!("M10 snapshot codec: {error:?}"))?)?;
        std::fs::write(path.join("m10-drown.bin"), oteryn_protocol_oteryn::actor_spell::encode_actor_vitals(&delta.value).map_err(|error| format!("M10 delta codec: {error:?}"))?)?;
        std::fs::write(path.join("m10-expected.txt"), format!("{} {} {} {} {} {}\n", initial.health, delta.value.health, delta.value.mana, delta.base_revision, delta.new_revision, delta.server_sequence))?;
        Ok(())
    })();
    if let Err(error) = result {
        evidence(&format!("M10_CAPTURE_WRITE_ERROR {error}"));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let mut client = connect_world(clients, 0xa0).await?;
    let start = tile(client.world_spatial().actor_position)?;
    let mut path = route(world, start, |at| is_underwater(world, at))
        .ok_or("no reachable underwater tile for M10 client capture")?;
    let enter = path.pop().ok_or("M10 client capture started underwater")?;
    walk(&mut client, world, &path).await?;
    let initial = *client.actor_vitals().ok_or("M10 initial ActorVitals absent")?;
    let moved = client.step_retrying(enter).await?;
    if moved.disposition != StepDisposition::Moved || moved.world_spatial_delta.is_none() {
        return Err("M10 underwater entry did not move with a spatial delta".into());
    }
    let delta = drown_delta(&mut client, initial.health).await?;
    if initial.health.saturating_sub(delta.value.health) != 20 {
        return Err(format!("M10 drowning damage was not 20: {initial:?} -> {:?}", delta.value).into());
    }
    capture(&initial, &delta);
    let _ = client.session_mut().logout().await?;
    Ok(())
}
