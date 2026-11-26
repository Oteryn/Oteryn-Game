//! I06 client replay capture: kill one real rat, leave its loot in the corpse, and retain the
//! exact entity-domain payloads that introduce and retire that corpse. The matching session
//! test replays them through `Session::apply_state_delta` in `crates/session/src/domain_dispatch.rs`
//! and observes the projection exposed by `Session::world_entities` in
//! `crates/session/src/session_access.rs` (and `DevClientSession::world_entities` in
//! `tools/dev-client/src/lib.rs`).
use super::*;
use oteryn_dev_client::{EntityDetail, EntityKind, SessionEvent, WorldSpatialEntitiesDelta};
use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, ChaseMode, FightMode, FightModes};
use oteryn_protocol_oteryn::world_spatial_entities::encode_world_spatial_entities_delta;

fn capture(
    introduced: &WorldSpatialEntitiesDelta,
    retired: &WorldSpatialEntitiesDelta,
    expected: &str,
) -> TestResult {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let directory = Path::new(&directory);
    for (name, delta) in [("i06-corpse.bin", introduced), ("i06-decay.bin", retired)] {
        let bytes = encode_world_spatial_entities_delta(delta)
            .map_err(|error| format!("I06 capture encode {name}: {error:?}"))?;
        std::fs::write(directory.join(name), bytes)?;
    }
    std::fs::write(directory.join("i06-expected.txt"), expected)?;
    Ok(())
}

fn identity_text(identity: &[u8; 16]) -> String {
    identity.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    const LEVEL: u32 = 100;
    const PREPARATION: character_prep::Seeded = character_prep::Seeded {
        profile: "knight",
        reason: "survive the rat encounter while observing corpse retirement",
    };
    character_prep::declare_seeded("I06.client", "knight-level-100", PREPARATION.reason);
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_Q_I06_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let mut client = PREPARATION.connect(clients, 0xc6, 0).await?;
    let level = full_e2ef7::player_level(&mut client).await?;
    let (max_health, max_mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "knight",
        LEVEL,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("I06 knight vitals unavailable")?;
    if level != LEVEL
        || !client.actor_vitals().is_some_and(|vitals| {
            vitals.health == max_health
                && vitals.max_health == max_health
                && vitals.max_mana == max_mana
        })
    {
        return Err(format!(
            "I06 seeded knight stats: level={level}, vitals={:?}",
            client.actor_vitals()
        )
        .into());
    }
    let modes = FightModes {
        fight_mode: FightMode::Offensive,
        chase: ChaseMode::Chase,
        ..FightModes::DEFAULT
    };
    let reply = client.session_mut().set_fight_modes(&modes).await?;
    if reply.status != CommandStatus::Accepted || reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("I06 fight modes refused: {reply:?}").into());
    }
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let position = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.others().find(|entity| {
                entity.kind == EntityKind::Creature
                    && entity
                        .display_name
                        .as_ref()
                        .and_then(|name| name.as_str().ok())
                        == Some("Rat")
                    && entity.position.floor == position.floor
                    && entity
                        .position
                        .x
                        .abs_diff(position.x)
                        .max(entity.position.y.abs_diff(position.y))
                        <= 1
            })
        })
        .copied()
        .ok_or("I06 adjacent rat absent")?;
    client.take_events();
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.status != CommandStatus::Accepted || attack.disposition != AttackIntentDisposition::Ok
    {
        return Err(format!("I06 attack refused: {attack:?}").into());
    }
    let kill_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(kill_deadline, "bounded-observation");
    let mut target_position = target.position;
    let (corpse, introduced) = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        let mut found = None;
        for event in client.take_events() {
            let SessionEvent::WorldSpatialEntities(delta) = event else {
                continue;
            };
            if let Some(updated) = delta
                .value
                .update
                .iter()
                .find(|entity| entity.entity == target.entity)
            {
                target_position = updated.position;
            }
            if delta.value.leave.contains(&target.entity) {
                let mut corpses = delta.value.enter.iter().filter(|entity| {
                    entity.kind == EntityKind::Corpse && entity.position == target_position
                });
                let corpse = corpses
                    .next()
                    .copied()
                    .ok_or("I06 target left without its corpse")?;
                if corpses.next().is_some() {
                    return Err("I06 target left with ambiguous corpse evidence".into());
                }
                found = Some((corpse, delta.value));
            }
        }
        if let Some(found) = found {
            break found;
        }
        if tokio::time::Instant::now() >= kill_deadline {
            return Err("I06 kill produced no corpse entity payload".into());
        }
    };
    let EntityDetail::Object {
        item_handle: Some(handle),
        ..
    } = corpse.detail
    else {
        return Err("I06 corpse has no item handle".into());
    };
    let used = client.use_item(handle).await?;
    if used.status != CommandStatus::Accepted {
        return Err(format!("I06 corpse open refused: {used:?}").into());
    }
    let open_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while client
        .open_container()
        .is_none_or(|container| container.container_handle != Some(handle))
    {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
        if tokio::time::Instant::now() >= open_deadline {
            return Err("I06 corpse container payload absent".into());
        }
    }
    let remaining = client
        .open_container()
        .map_or(0, |container| container.entries.len());
    if remaining == 0 {
        return Err("I06 corpse had no remaining loot to retire".into());
    }
    let decay_deadline = tokio::time::Instant::now() + Duration::from_secs(75);
    let retired = loop {
        client.service_liveness(Duration::from_millis(200)).await?;
        let found = client
            .take_events()
            .into_iter()
            .find_map(|event| match event {
                SessionEvent::WorldSpatialEntities(delta)
                    if delta.value.leave.contains(&corpse.entity) =>
                {
                    Some(delta.value)
                }
                _ => None,
            });
        if let Some(found) = found {
            break found;
        }
        if tokio::time::Instant::now() >= decay_deadline {
            return Err("I06 corpse did not retire before 75s".into());
        }
    };
    if retired.actor_position != introduced.actor_position {
        return Err("I06 observer moved before corpse retirement".into());
    }
    capture(
        &introduced,
        &retired,
        &format!("{} {remaining}\n", identity_text(&corpse.entity.identity)),
    )?;
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
