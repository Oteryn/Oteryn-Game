//! P12c — Cyclopedia character information and recent death/PvP-kill history.
//!
//! Concrete player scenario derived from the matrix, the re-audit, and
//! `gameplay_transport/character_info.rs`:
//! 1. After P12c's prerequisite PvP fight has been completed by the immediately preceding
//!    E2EF7 player journey, admit its killer and send `QuerySelf(offset=0, limit=50)`; require
//!    an accepted Character Info summary containing the character's real level/experience,
//!    bounded item counters, and at least one recent PvP kill.
//! 2. Send the paged `QuerySelf(offset=1, limit=1)` player command and require an accepted,
//!    strictly decoded bounded page (the history counts may be zero at that offset).
//! 3. Send `QueryVisible` for the other provisioned character without Friends/VIP consent;
//!    require the server's refusal and no disclosed payload.
//! 4. Walk the admitted character through the compiled world to a poison-spider spawn, receive
//!    real combat damage until lethal respawn, then send `QuerySelf` again and require exactly
//!    one additional recent-death entry while the PvP-kill count remains present.
//! 5. Log out normally, obtain a fresh admission, send `QuerySelf` once more, and require the
//!    same summary, death count, and PvP-kill count, proving the histories are durable.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::character_info as info;
use oteryn_protocol_oteryn::logout::LogoutResult;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Page {
    level: u64,
    experience: u64,
    counters: [u64; 6],
    deaths: u16,
    pvp_kills: u16,
}

fn decode_page(bytes: &[u8], limit: u8) -> TestResult<Page> {
    if bytes.first() != Some(&0) || bytes.len() < 71 {
        return Err(format!(
            "step character-info decode: malformed header/length={}",
            bytes.len()
        )
        .into());
    }
    let mut values = [0u64; 8];
    for (index, value) in values.iter_mut().enumerate() {
        let raw = i64::from_le_bytes(bytes[1 + index * 8..9 + index * 8].try_into()?);
        *value =
            u64::try_from(raw).map_err(|_| "step character-info decode: negative summary value")?;
    }
    let deaths = u16::from_le_bytes(bytes[65..67].try_into()?);
    let pvp_kills = u16::from_le_bytes(bytes[67..69].try_into()?);
    let title_count = usize::from(bytes[69]);
    if deaths > u16::from(limit)
        || pvp_kills > u16::from(limit)
        || title_count > info::TITLE_COUNT
        || bytes.len() != 71 + title_count
    {
        return Err(format!("step character-info decode: unbounded/trailing reply deaths={deaths} kills={pvp_kills} titles={title_count} len={}", bytes.len()).into());
    }
    let earned = &bytes[70..70 + title_count];
    if earned
        .iter()
        .any(|raw| info::CharacterTitle::from_id(*raw).is_none())
        || earned
            .iter()
            .enumerate()
            .any(|(at, raw)| earned[..at].contains(raw))
    {
        return Err("step character-info decode: invalid/duplicate title".into());
    }
    let selected = bytes[70 + title_count];
    if selected != 0 && !earned.contains(&selected) {
        return Err("step character-info decode: selected title is not earned".into());
    }
    Ok(Page {
        level: values[0],
        experience: values[1],
        counters: values[2..8].try_into()?,
        deaths,
        pvp_kills,
    })
}

fn encode(operation: info::CharacterInfoOperation) -> Vec<u8> {
    let mut bytes = Vec::new();
    match operation {
        info::CharacterInfoOperation::QuerySelf(page) => {
            bytes.push(0);
            bytes.extend_from_slice(&page.offset.to_le_bytes());
            bytes.push(page.limit);
        }
        info::CharacterInfoOperation::QueryVisible { character, page } => {
            bytes.push(1);
            bytes.extend_from_slice(&character);
            bytes.extend_from_slice(&page.offset.to_le_bytes());
            bytes.push(page.limit);
        }
        _ => {}
    }
    bytes
}

async fn query(client: &mut DevClientSession, offset: u16, limit: u8) -> TestResult<Page> {
    let operation = info::CharacterInfoOperation::QuerySelf(
        info::PageRequest::new(offset, limit).ok_or("step query: invalid page")?,
    );
    let reply = client
        .session_mut()
        .gated_command(
            info::CAPABILITY_CHARACTER_INFO_V1,
            info::COMMAND_TYPE_CHARACTER_INFO,
            &encode(operation),
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step query-self: observed status={:?} payload={:?}; need accepted Character Info reply", reply.status, reply.payload).into());
    }
    decode_page(&reply.payload, limit)
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        match client.session_mut().logout().await?.result {
            Some(LogoutResult::Accepted) => return Ok(()),
            Some(LogoutResult::Busy | LogoutResult::InFight { .. })
                if tokio::time::Instant::now() < deadline =>
            {
                client.service_liveness(Duration::from_millis(100)).await?;
                client.take_events();
            }
            observed => {
                return Err(format!(
                    "step logout: observed reply={observed:?}; need accepted logout"
                )
                .into());
            }
        }
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0xca, 0).await?;
    let baseline = query(&mut client, 0, 50).await?;
    if baseline.level == 0 || baseline.pvp_kills == 0 {
        return Err(format!("step 1 query-self: observed={baseline:?}; need nonzero native level and the preceding real PvP frag").into());
    }
    let _paged = query(&mut client, 1, 1).await?;

    let visible = info::CharacterInfoOperation::QueryVisible {
        character: clients.characters[1],
        page: info::PageRequest::new(0, 1).ok_or("step refusal page")?,
    };
    let refused = client
        .session_mut()
        .gated_command(
            info::CAPABILITY_CHARACTER_INFO_V1,
            info::COMMAND_TYPE_CHARACTER_INFO,
            &encode(visible),
        )
        .await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!(
            "step 3 refusal: observed status={:?} payload={:?}; need rejected empty disclosure",
            refused.status, refused.payload
        )
        .into());
    }

    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        "oteryn:creature.poison_spider",
    )
    .await
    .map_err(|error| {
        format!("step 4 walk-to-combat: observed={error}; need reachable authored lethal spawn")
    })?;
    client.session_mut().attack_target(None).await?;
    let encounter = client.world_spatial().actor_position;
    let health = client
        .actor_vitals()
        .ok_or("step 4 combat: missing entry vitals")?
        .health;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut damaged = false;
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event {
                damaged |= delta.value.health < health;
            }
        }
        if damaged
            && client.world_spatial().actor_position != encounter
            && client
                .actor_vitals()
                .is_some_and(|vitals| vitals.health == vitals.max_health)
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step 4 combat: timeout damaged={damaged} position={:?}; need lethal damage and respawn deltas", client.world_spatial().actor_position).into());
        }
    }
    let after = query(&mut client, 0, 50).await?;
    if after.deaths != baseline.deaths.saturating_add(1) || after.pvp_kills < baseline.pvp_kills {
        return Err(format!("step 4 history delta: before={baseline:?} after={after:?}; need one death and retained PvP kill").into());
    }
    let old_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    client = connect_character(clients, 0xcb, 0).await?;
    if client.join_snapshot().game_session_id == old_session {
        return Err("step 5 fresh admission: server reused GameSession".into());
    }
    let durable = query(&mut client, 0, 50).await?;
    if durable != after {
        return Err(format!("step 5 durable recheck: before-logout={after:?} fresh={durable:?}; need identical summary/history counts").into());
    }
    logout(&mut client).await?;
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(360), verdict::observe(journey(clients))).await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "e2e_all scenario=P12c.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P12c.full status=FAIL seconds={:.3} evidence={}",
            started.elapsed().as_secs_f64(),
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence(&format!(
            "e2e_all scenario=P12c.full status=FAIL seconds={:.3} evidence=step_deadline_observed=timeout_need=server_reply_or_delta",
            started.elapsed().as_secs_f64()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_decoder_is_bounded_and_strict() -> TestResult {
        let mut bytes = vec![0];
        bytes.extend_from_slice(&8i64.to_le_bytes());
        bytes.extend_from_slice(&4_200i64.to_le_bytes());
        for value in 1..=6i64 {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&[
            1,
            info::CharacterTitle::AdeptArmorer.id(),
            info::CharacterTitle::AdeptArmorer.id(),
        ]);
        let page = decode_page(&bytes, 1)?;
        assert_eq!(
            (page.level, page.experience, page.deaths, page.pvp_kills),
            (8, 4_200, 1, 1)
        );
        for end in 0..bytes.len() {
            assert!(decode_page(&bytes[..end], 1).is_err());
        }
        bytes.push(0);
        assert!(decode_page(&bytes, 1).is_err());
        Ok(())
    }
}
