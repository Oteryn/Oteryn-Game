//! C15 — hunting stamina drain, offline recovery, and XP effects.
//!
//! Concrete player scenario derived from matrix row C15, the parity audit, and
//! `hunting_stamina.rs` / `stamina.rs`:
//! 1. Admit the node-boot character and send the native GeneralStats tab command;
//!    expect an accepted frame containing XP, level, and bounded hunting stamina.
//! 2. Send ATTACK against the character's own visible actor (the refusal/no-kill path);
//!    expect a non-OK native disposition and prove GeneralStats has no XP or stamina delta.
//! 3. Walk through the compiled world to an ordinary unprotected rat spawn and send
//!    ATTACK for the visible creature; expect an accepted attack, creature removal,
//!    positive XP settlement, and exactly one minute of stamina debit in the same hunt.
//!    The positive award also proves the non-empty stamina XP band is applied.
//! 4. Send LOGOUT until the combat lock permits it; expect Accepted, then remain
//!    genuinely offline for the ten-minute delay plus one six-minute bonus-band recovery period.
//! 5. Freshly admit the same character; expect a new game-session id and durable XP,
//!    plus exactly one recovered stamina minute, then send LOGOUT and expect Accepted.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{attack::AttackIntentDisposition, cyclopedia_character as character};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Progress {
    xp: u64,
    level: u32,
    stamina: u16,
}

async fn progress(client: &mut DevClientSession) -> TestResult<Progress> {
    let reply = client
        .session_mut()
        .gated_command(
            character::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            character::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[character::CharacterTab::GeneralStats as u8, 0, 0, 1],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step-1 GeneralStats refused: {reply:?}").into());
    }
    decode_progress(&reply.payload)
}

fn decode_progress(payload: &[u8]) -> TestResult<Progress> {
    if payload.len() != 42
        || payload.first() != Some(&(character::CharacterTab::GeneralStats as u8))
        || payload[33] > 1
    {
        return Err(format!("step-1 malformed GeneralStats frame: len={}", payload.len()).into());
    }
    let value = Progress {
        xp: u64::from_le_bytes(payload[1..9].try_into()?),
        level: u32::from_le_bytes(payload[9..13].try_into()?),
        stamina: u16::from_le_bytes(payload[29..31].try_into()?),
    };
    if value.level == 0 || value.stamina > 2_520 {
        return Err(format!("step-1 invalid GeneralStats values: {value:?}").into());
    }
    Ok(value)
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
    .ok_or("step-3 no player-walkable route to an unprotected rat spawn")?;
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
        .ok_or("step-3 rat spawn reached but no adjacent creature frame observed")?;
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("step-3 native ATTACK refused: {attack:?}").into());
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
            return Err(format!(
                "step-3 ATTACK timed out without creature removal; reply={attack:?}"
            )
            .into());
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
            _ => return Err(format!("step-4 LOGOUT refused: {reply:?}").into()),
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
    let mut client = row_connect_world(clients, 0xc5).await?;
    let first_session = client.join_snapshot().game_session_id;
    let before = progress(&mut client).await?;

    let own = client
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .find(|entity| entity.entity.identity == *entities.own_identity())
        })
        .map(|entity| entity.entity)
        .ok_or("step-2 own visible actor missing")?;
    let refusal = client.session_mut().attack_target(Some(&own)).await?;
    if refusal.disposition == AttackIntentDisposition::Ok {
        return Err(format!("step-2 self-target ATTACK was not refused: {refusal:?}").into());
    }
    client.service_liveness(Duration::from_millis(100)).await?;
    if progress(&mut client).await? != before {
        return Err("step-2 refused self-target ATTACK changed XP, level, or stamina".into());
    }

    // HuntingClock's first due boundary is the admission second. Cross it
    // naturally so this credited kill must exercise (rather than merely arm)
    // the one-minute debit, while remaining well below the two-minute branch.
    let wait_until = tokio::time::Instant::now() + Duration::from_secs(2);
    while tokio::time::Instant::now() < wait_until {
        client.service_liveness(Duration::from_millis(20)).await?;
    }
    hunt(&mut client, world).await?;
    let settlement_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let after = loop {
        let current = progress(&mut client).await?;
        if current.xp > before.xp || tokio::time::Instant::now() >= settlement_deadline {
            break current;
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    };
    if after.xp <= before.xp || after.level < before.level || after.stamina + 1 != before.stamina {
        return Err(
            format!("step-3 kill settlement mismatch: before={before:?} after={after:?}").into(),
        );
    }
    logout(&mut client).await?;
    drop(client);

    tokio::time::sleep(Duration::from_secs(960)).await;
    let mut fresh = row_connect_world(clients, 0xc6).await?;
    if fresh.join_snapshot().game_session_id == first_session {
        return Err("step-5 fresh admission reused the previous game session".into());
    }
    let recovered = progress(&mut fresh).await?;
    if recovered.xp != after.xp
        || recovered.level != after.level
        || recovered.stamina != after.stamina + 1
    {
        return Err(format!(
            "step-5 durable/offline recovery mismatch: saved={after:?} fresh={recovered:?}"
        )
        .into());
    }
    logout(&mut fresh).await?;
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(1_800), journey(clients)).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err("C15 journey deadline after a real native lifecycle attempt".into()),
    };
    if let Err(error) = &result {
        evidence(&format!("C15_GAP {error}"));
    }
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=C15.full status={} seconds={:.3}{}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64(),
        result
            .as_ref()
            .err()
            .map(|error| format!(" evidence=step-observation:{error}").replace(' ', "_"))
            .unwrap_or_default(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn general_stats_decoder_reads_xp_level_and_stamina() -> TestResult {
        let payload = character::TabView::GeneralStats(character::GeneralStats {
            experience: 123,
            level: 7,
            stamina_minutes: 2_519,
            ..character::GeneralStats::default()
        })
        .encode()
        .map_err(|error| format!("encode fixture: {error:?}"))?;
        assert_eq!(
            decode_progress(&payload)?,
            Progress {
                xp: 123,
                level: 7,
                stamina: 2_519
            }
        );
        assert!(decode_progress(&payload[..41]).is_err());
        Ok(())
    }
}
