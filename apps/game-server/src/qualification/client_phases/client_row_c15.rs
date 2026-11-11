//! Client replay capture for C15 — hunting stamina drain, recovery, and XP.

use super::*;
use super::{
    row_connect_world as connect_world,
    row_finish as row_logout,
};
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{attack::AttackIntentDisposition, cyclopedia_character as character};

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
    if reply.payload.len() != 42
        || reply.payload.first() != Some(&(character::CharacterTab::GeneralStats as u8))
        || reply.payload[33] > 1
    {
        return Err(format!(
            "C15 client malformed GeneralStats: {} bytes",
            reply.payload.len()
        )
        .into());
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

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        std::fs::write(Path::new(&path).join(name), bytes)?;
    }

    Ok(())
}

async fn hunt(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let rats: Vec<_> = metadata
        .spawns
        .sources
        .iter()
        .flat_map(|source| source.points.iter().map(move |point| (source.floor, point)))
        .filter(|(_, point)| {
            metadata
                .spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|creature| creature.key == "oteryn:creature.rat")
        })
        .map(|(floor, point)| TilePos {
            x: point.x,
            y: point.y,
            floor,
        })
        .collect();
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
            && rats.iter().any(|rat| {
                rat.floor == at.floor && rat.x.abs_diff(at.x).max(rat.y.abs_diff(at.y)) <= 1
            })
    })
    .ok_or("C15 client no route to an unprotected rat")?;
    walk(client, world, &path).await?;
    let at = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Creature
                    && entity.position.floor == at.floor
                    && entity
                        .position
                        .x
                        .abs_diff(at.x)
                        .max(entity.position.y.abs_diff(at.y))
                        <= 1
            })
        })
        .copied()
        .ok_or("C15 client rat reached without an adjacent creature")?;
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("C15 client ATTACK refused: {attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
        if client
            .world_entities()
            .is_some_and(|entities| entities.get(&target.entity).is_none())
        {
            client.session_mut().attack_target(None).await?;
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            client.session_mut().attack_target(None).await?;
            return Err("C15 client ATTACK timed out".into());
        }
    }
}

async fn logout(client: &mut RowClient) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(920);
    loop {
        let reply = client.row_logout().await?;
        match reply.result {
            Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
                if reply.status == CommandStatus::Accepted =>
            {
                return Ok(());
            }
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
    row_complete(async {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(1_800);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xc7).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let (before, before_payload) = progress(&mut client).await?;
        capture("c15-before.bin", &before_payload)?;

        let wait_until = tokio::time::Instant::now() + Duration::from_secs(2);
        while tokio::time::Instant::now() < wait_until {
            client.service_liveness(Duration::from_millis(20)).await?;
        }
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
        if after.xp <= before.xp
            || after.level < before.level
            || after.stamina + 1 != before.stamina
        {
            return Err(
                format!("C15 client kill mismatch: before={before:?} after={after:?}").into(),
            );
        }
        capture("c15-after.bin", &after_payload)?;
        logout(&mut client).await?;
        row_needs_logout_0 = false;
        if row_needs_logout_0 {
            tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
            row_needs_logout_0 = false;
        }

        tokio::time::sleep(Duration::from_secs(780)).await;
        let mut fresh = connect_world(clients, 0xc8).await?;
        let mut row_needs_logout_1 = true;
        let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
            let (recovered, recovered_payload) = progress(&mut fresh).await?;
            if recovered.xp != after.xp
                || recovered.level != after.level
                || recovered.stamina != after.stamina + 1
            {
                return Err(format!(
                    "C15 client recovery mismatch: saved={after:?} fresh={recovered:?}"
                )
                .into());
            }
            capture("c15-recovered.bin", &recovered_payload)?;
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
            )?;
            logout(&mut fresh).await?;
            row_needs_logout_1 = false;
            Ok(())
        })
        .await
        .unwrap_or_else(|error| Err(error.into()));
        if row_needs_logout_1 {
            tokio::time::timeout(Duration::from_secs(10), row_logout(&mut fresh)).await??;
        }
        row_result
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    }
    row_result

    }).await?
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = row_complete(journey(clients)).await;
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
