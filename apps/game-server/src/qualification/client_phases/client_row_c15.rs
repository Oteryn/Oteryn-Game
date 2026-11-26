//! Client replay capture for C15 — hunting stamina drain, recovery, and XP.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{attack::AttackIntentDisposition, cyclopedia_character as character};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Progress {
    xp: u64,
    level: u32,
    stamina: u16,
}

async fn progress(client: &mut DevClientSession) -> TestResult<(Progress, Vec<u8>)> {
    let reply = client
        .session_mut()
        .gated_command(
            character::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            character::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[character::CharacterTab::GeneralStats as u8, 0, 0, 1],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("C15 client GeneralStats refused: {reply:?}").into());
    }
    if general_stats::validate(&reply.payload, general_stats::selected(client)).is_err()
        || reply.payload.first() != Some(&(character::CharacterTab::GeneralStats as u8))
        || reply.payload[33] > 1
    {
        return Err(format!("C15 client malformed GeneralStats: {} bytes", reply.payload.len()).into());
    }
    let value = Progress {
        xp: u64::from_le_bytes(reply.payload[1..9].try_into()?),
        level: u32::from_le_bytes(reply.payload[9..13].try_into()?),
        stamina: u16::from_le_bytes(reply.payload[29..31].try_into()?),
    };
    if value.level == 0 || value.stamina > 2_520 {
        return Err(format!("C15 client invalid GeneralStats values: {value:?}").into());
    }
    Ok((value, reply.payload))
}

fn capture(name: &str, bytes: &[u8]) {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        && let Err(error) = std::fs::write(Path::new(&path).join(name), bytes)
    {
        evidence(&format!("C15_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

pub(super) async fn reach_rat(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let mut spawn_cells = BTreeSet::new();
    let mut rats = Vec::new();
    for source in &metadata.spawns.sources {
        for point in &source.points {
            let cell = TilePos {
                x: point.x,
                y: point.y,
                floor: source.floor,
            };
            spawn_cells.insert(cell);
            if metadata
                .spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|creature| creature.key == "oteryn:creature.rat")
            {
                rats.push(cell);
            }
        }
    }
    // Plan the entire walk before sending STEP. A compiled spawn can put a
    // creature on walkable ground, so reserve those cells for their occupants.
    let path = route_avoiding(
        world,
        tile(client.world_spatial().actor_position)?,
        |at| {
            world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
                && rats.iter().any(|rat| {
                    rat.floor == at.floor && rat.x.abs_diff(at.x).max(rat.y.abs_diff(at.y)) == 1
                })
        },
        |at| !spawn_cells.contains(&at),
    )
    .ok_or("C15 no route to an unprotected Rat approach outside compiled spawn cells")?;
    for direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, before, direction);
        let outcome = client.step(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || Some(actual) != expected
        {
            return Err(format!(
                "C15 Rat approach STEP {direction:?} from={before:?} expected={expected:?} actual={actual:?}: {outcome:?}"
            ).into());
        }
        verdict::Harness::global().progress("walk");
        client.take_events();
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let at = client.world_spatial().actor_position;
        if let Some(rat) = client.world_entities().and_then(|entities| {
            entities.iter().find(|entity| full_row_c15::adjacent_hunt_target(entity, at))
        }) {
            evidence(&format!("C15_RAT_APPROACH player={at:?} rat={:?}", rat.position));
            return Ok(*rat);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("C15 Rat absent at the selected approach tile: {at:?}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn hunt(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let target = reach_rat(client, world).await?;
    let attack = client.session_mut().attack_target(Some(&target.entity)).await?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("C15 client ATTACK refused: {attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
        if client.world_entities().is_some_and(|entities| entities.get(&target.entity).is_none()) {
            client.session_mut().attack_target(None).await?;
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            client.session_mut().attack_target(None).await?;
            return Err("C15 client ATTACK timed out".into());
        }
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(920);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    loop {
        let reply = client.session_mut().logout().await?;
        match reply.result {
            Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
                if reply.status == CommandStatus::Accepted => return Ok(()),
            Some(oteryn_protocol_oteryn::logout::LogoutResult::InFight { .. })
                if tokio::time::Instant::now() < deadline =>
            {
                client.service_liveness(Duration::from_millis(250)).await?;
                client.take_events();
            }
            _ => return Err(format!("C15 client LOGOUT refused: {reply:?}").into()),
        }
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xc7).await?;
    let (before, before_payload) = progress(&mut client).await?;
    general_stats::record_selection("c15-before.bin", &before_payload, general_stats::selected(&client))?;
    capture("c15-before.bin", &before_payload);

    tokio::time::sleep(Duration::from_secs(2)).await;
    hunt(&mut client, world).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let (after, after_payload) = loop {
        let current = progress(&mut client).await?;
        if current.0.xp > before.xp || tokio::time::Instant::now() >= deadline {
            break current;
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    };
    if after.xp <= before.xp || after.level < before.level || after.stamina + 1 != before.stamina {
        return Err(format!("C15 client kill mismatch: before={before:?} after={after:?}").into());
    }
    general_stats::record_selection("c15-after.bin", &after_payload, general_stats::selected(&client))?;
    capture("c15-after.bin", &after_payload);
    logout(&mut client).await?;
    drop(client);

    let recovery = full_row_c15::recovery_interval(after.stamina);
    verdict::Harness::global().bounded_wait(tokio::time::Instant::now() + recovery, "offline-recovery");
    tokio::time::sleep(recovery).await;
    let mut fresh = connect_world(clients, 0xc8).await?;
    let (recovered, recovered_payload) = progress(&mut fresh).await?;
    if recovered.xp != after.xp
        || recovered.level != after.level
        || recovered.stamina != after.stamina + 1
    {
        return Err(format!("C15 client recovery mismatch: saved={after:?} fresh={recovered:?}").into());
    }
    general_stats::record_selection("c15-recovered.bin", &recovered_payload, general_stats::selected(&fresh))?;
    capture("c15-recovered.bin", &recovered_payload);
    capture(
        "c15-expected.txt",
        format!(
            "{} {} {}\n{} {} {}\n{} {} {}\n",
            before.xp,
            before.level,
            before.stamina,
            after.xp,
            after.level,
            after.stamina,
            recovered.xp,
            recovered.level,
            recovered.stamina
        )
        .as_bytes(),
    );
    logout(&mut fresh).await?;
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(Duration::from_secs(1_800), verdict::observe(journey(clients))).await;
    let result = result.unwrap_or_else(|_| Err("C15 client journey deadline".into()));
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=C15.client status={}{}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        result
            .as_ref()
            .err()
            .map(|error| format!(" evidence={error}").replace(' ', "_"))
            .unwrap_or_default()
    ));
}
