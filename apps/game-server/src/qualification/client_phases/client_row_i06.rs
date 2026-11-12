//! I06 client replay capture: kill one real rat, leave its loot in the corpse, and retain the
//! exact entity-domain payloads that introduce and retire that corpse. The matching session
//! test replays them through `Session::apply_state_delta` in `crates/session/src/domain_dispatch.rs`
//! and observes the projection exposed by `Session::world_entities` in
//! `crates/session/src/session_access.rs` (and `DevClientSession::world_entities` in
//! `tools/dev-client/src/lib.rs`).
use super::*;
use oteryn_dev_client::{EntityDetail, EntityKind, SessionEvent, WorldSpatialEntitiesDelta};
use oteryn_protocol_oteryn::world_spatial_entities::encode_world_spatial_entities_delta;

fn capture(name: &str, bytes: &[u8]) {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&directory).join(name), bytes) {
        evidence(&format!("I06_CAPTURE_WRITE name={name} error={error}"));
    }
}

fn capture_delta(name: &str, delta: &WorldSpatialEntitiesDelta) {
    match encode_world_spatial_entities_delta(delta) {
        Ok(payload) => capture(name, &payload),
        Err(error) => evidence(&format!("I06_CAPTURE_ENCODE name={name} error={error:?}")),
    }
}

fn identity_text(identity: &[u8; 16]) -> String {
    identity.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xc6).await?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        "oteryn:creature.rat",
    )
    .await?;
    let position = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.others().find(|entity| {
                entity.kind == EntityKind::Creature
                    && entity.position.floor == position.floor
                    && entity.position.x.abs_diff(position.x).max(entity.position.y.abs_diff(position.y)) <= 1
            })
        })
        .copied()
        .ok_or("I06 adjacent rat absent")?;
    let attack = client.session_mut().attack_target(Some(&target.entity)).await?;
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("I06 attack refused: {attack:?}").into());
    }
    let kill_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(kill_deadline, "bounded-observation");
    let (corpse, introduced) = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        let mut found = None;
        for event in client.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = event
                && let Some(corpse) = delta.value.enter.iter().find(|entity| entity.kind == EntityKind::Corpse)
            {
                found = Some((*corpse, delta.value));
            }
        }
        if let Some(found) = found {
            break found;
        }
        if tokio::time::Instant::now() >= kill_deadline {
            return Err("I06 kill produced no corpse entity payload".into());
        }
    };
    let EntityDetail::Object { item_handle: Some(handle), .. } = corpse.detail else {
        return Err("I06 corpse has no item handle".into());
    };
    let used = client.use_item(handle).await?;
    if used.status != CommandStatus::Accepted {
        return Err(format!("I06 corpse open refused: {used:?}").into());
    }
    let open_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while client.open_container().is_none_or(|container| container.container_handle != Some(handle)) {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
        if tokio::time::Instant::now() >= open_deadline {
            return Err("I06 corpse container payload absent".into());
        }
    }
    let remaining = client.open_container().map_or(0, |container| container.entries.len());
    if remaining == 0 {
        return Err("I06 corpse had no remaining loot to retire".into());
    }
    let decay_deadline = tokio::time::Instant::now() + Duration::from_secs(75);
    let retired = loop {
        client.service_liveness(Duration::from_millis(200)).await?;
        let found = client.take_events().into_iter().find_map(|event| match event {
            SessionEvent::WorldSpatialEntities(delta)
                if delta.value.leave.contains(&corpse.entity) => Some(delta.value),
            _ => None,
        });
        if let Some(found) = found {
            break found;
        }
        if tokio::time::Instant::now() >= decay_deadline {
            return Err("I06 corpse did not retire before 75s".into());
        }
    };
    capture_delta("i06-corpse.bin", &introduced);
    capture_delta("i06-decay.bin", &retired);
    capture(
        "i06-expected.txt",
        format!("{} {}\n", identity_text(&corpse.entity.identity), remaining).as_bytes(),
    );
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=I06.client status=PASS corpse={} remaining_loot={remaining}",
        identity_text(&corpse.entity.identity)
    ));
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = verdict::observe(journey(clients)).await {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=I06.client status=FAIL evidence={}",
            error.to_string().replace([' ', '\n'], "_")
        ));
    }
}
