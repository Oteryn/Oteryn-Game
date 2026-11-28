//! G10: Cyclopedia titles, analyser, boss tracker and minimap markers.
//! 1. Admit, require the analyser capability (10) in the selected set.
//! 2. QuerySelf; select an earned title (needs at least one earned title), then a refused
//!    unearned Select, then QuerySelf again for the selected delta.
//! 3. BOSS_COOLDOWNS twice about one second apart; the server clock must advance.
//! 4. Log out, re-admit and QuerySelf: the selected title must persist.
//!
//! Replay the original admission frames for both sessions in the native client, and require
//! the local marker to survive the same World's relog before removing it.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::boss_encounter as boss;
use oteryn_dev_client::client_runtime::character_info_panel as info;
use oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse;

const ANALYSER_CAPABILITY: u32 = 10;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "server-and-live-minimap-replay".to_owned()),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G10.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn send(
    client: &mut DevClientSession,
    operation: info::CharacterInfoOperation,
) -> TestResult<ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(
            info::CAPABILITY_CHARACTER_INFO_V1,
            info::COMMAND_TYPE_CHARACTER_INFO,
            &info::encode_operation(operation),
        )
        .await?)
}

/// The earned title ids and the selected id of a QuerySelf reply.
fn titles(bytes: &[u8]) -> TestResult<(Vec<u8>, u8)> {
    if bytes.first() != Some(&0) || bytes.len() < 71 {
        return Err(format!("step=query;observed=malformed-len-{}", bytes.len()).into());
    }
    let count = usize::from(bytes[69]);
    if bytes.len() != 71 + count {
        return Err(format!("step=query;observed=count-{count}-len-{}", bytes.len()).into());
    }
    Ok((bytes[70..70 + count].to_vec(), bytes[70 + count]))
}

async fn query(client: &mut DevClientSession) -> TestResult<(Vec<u8>, u8)> {
    let page = info::PageRequest::new(0, 1).ok_or("step=query;observed=page")?;
    let reply = send(client, info::CharacterInfoOperation::QuerySelf(page)).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=query;observed={:?}", reply.status).into());
    }
    titles(&reply.payload)
}

async fn cooldown_now(client: &mut DevClientSession) -> TestResult<u64> {
    let query = boss::query_taints("").ok_or("step=cooldowns;observed=encoder")?;
    let reply = client
        .session_mut()
        .gated_command(query.capability, query.kind, &query.payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=cooldowns;observed={:?}", reply.status).into());
    }
    let snapshot = boss::decode(&reply.payload).ok_or("step=cooldowns;observed=malformed")?;
    Ok(snapshot.now)
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout;observed=refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_G10_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    character_prep::declare_seeded("G10.full", "sorcerer-level-50", "Trolltrasher eligibility");
    let mut setup = connect_character(clients, 0xd0, 0).await?;
    let setup_session = setup.join_snapshot().game_session_id;
    request_fixture_control(uuid_text(setup_session.as_bytes()), "--profile", "g10-title".into()).await?;
    logout(&mut setup).await?;
    drop(setup);
    let mut first_frames = Vec::new();
    let mut client = connect_character_recording(clients, 0xd1, 0, Some(&mut first_frames)).await?;
    let first_sequence = client.session_mut().last_server_sequence();
    let session = client.join_snapshot().game_session_id;
    if setup_session == session {
        return Err("step=admission;observed=reused-setup-session".into());
    }
    let level = full_e2ef7::player_level(&mut client).await?;
    let (health, mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer", 50,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    ).ok_or("step=fixture;observed=no-derived-vitals")?;
    if level != 50 || !client.actor_vitals().is_some_and(|vitals|
        vitals.max_health == health && vitals.max_mana == mana) {
        return Err(format!("step=fixture;observed=level-{level}-or-vitals-mismatch").into());
    }
    if !client
        .selected_capabilities()
        .contains(&ANALYSER_CAPABILITY)
    {
        return Err("step=admission;needed:analyser-capability-10-selected".into());
    }
    let (earned, before) = query(&mut client).await?;
    let title = info::CharacterTitle::Trolltrasher;
    let chosen = title.id();
    if !earned.contains(&chosen) || before != 0 {
        return Err(format!("step=select-prestate;observed=earned-{earned:?}-selected-{before}").into());
    }
    evidence("G10_CHECK fixture-level-50-derived-vitals-and-earned-trolltrasher");
    let select = send(
        &mut client,
        info::CharacterInfoOperation::Select(Some(title)),
    )
    .await?;
    if select.status != CommandStatus::Accepted {
        return Err(format!("step=select;observed={:?}", select.status).into());
    }
    if query(&mut client).await? != (earned.clone(), chosen) {
        return Err("step=select-delta;observed=incorrect-title-state".into());
    }
    let unearned = info::CharacterTitle::Exalted;
    if earned.contains(&unearned.id()) {
        return Err("step=refuse-prestate;observed=exalted-already-earned".into());
    }
    let refused = send(
        &mut client,
        info::CharacterInfoOperation::Select(Some(unearned)),
    )
    .await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!("step=refuse;observed={:?}", refused.status).into());
    }
    if query(&mut client).await? != (earned.clone(), chosen) {
        return Err("step=refusal-state;observed=title-state-changed".into());
    }
    let first = cooldown_now(&mut client).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_millis(1_100);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    let second = cooldown_now(&mut client).await?;
    if second <= first {
        return Err(format!("step=cooldowns;observed=clock-{first}-then-{second}").into());
    }
    logout(&mut client).await?;
    drop(client);
    let mut second_frames = Vec::new();
    let mut again = connect_character_recording(clients, 0xd2, 0, Some(&mut second_frames)).await?;
    let second_sequence = again.session_mut().last_server_sequence();
    let second_session = again.join_snapshot().game_session_id;
    if again.join_snapshot().game_session_id == session {
        return Err("step=persist;observed=reused-session".into());
    }
    let (_, persisted) = query(&mut again).await?;
    if persisted != chosen {
        return Err(format!("step=persist;observed={persisted};need={chosen}").into());
    }
    logout(&mut again).await?;
    let root = std::path::PathBuf::from(std::env::var_os("NODE_BOOT_G10_REPLAY_DIR")
        .ok_or("step=minimap;needed=fresh-replay-directory")?);
    std::fs::create_dir(&root)?;
    for (name, frames) in [("entry.frames", first_frames), ("relog.frames", second_frames)] {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(root.join(name))?;
        for frame in frames {
            let length = oteryn_protocol_oteryn::FrameLength::new(u32::try_from(frame.len())?)?;
            file.write_all(&length.to_prefix())?;
            file.write_all(&frame)?;
        }
    }
    let executable = std::env::var_os("NODE_BOOT_G10_REPLAY_BIN")
        .ok_or("step=minimap;needed=native-client-replay-binary")?;
    let child = tokio::process::Command::new(executable)
        .args(["g10_replays_live_admissions_and_persists_exact_marker", "--ignored", "--nocapture"])
        .env("G10_REPLAY_DIR", &root)
        .env("G10_REPLAY_CHARACTER", uuid_text(&clients.characters[0]))
        .env("G10_REPLAY_WORLD", uuid_text(&clients.world))
        .env("G10_REPLAY_ENTRY_SESSION", uuid_text(session.as_bytes()))
        .env("G10_REPLAY_ENTRY_SEQUENCE", first_sequence.to_string())
        .env("G10_REPLAY_RELOG_SESSION", uuid_text(second_session.as_bytes()))
        .env("G10_REPLAY_RELOG_SEQUENCE", second_sequence.to_string())
        .kill_on_drop(true)
        .output().await?;
    let stdout = String::from_utf8(child.stdout)?;
    let checks: Vec<_> = stdout.lines().filter(|line| line.starts_with("G10_MINIMAP_REPLAY ")).collect();
    if !child.status.success() || checks.len() != 1 {
        return Err(format!("step=minimap;observed=replay-failed;stdout={stdout};stderr={}", String::from_utf8_lossy(&child.stderr)).into());
    }
    evidence(checks[0]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_rejects_a_short_reply_and_reads_a_valid_one() {
        assert!(titles(&[0; 10]).is_err());
        let mut reply = vec![0u8; 70];
        reply[69] = 1;
        reply.extend_from_slice(&[7, 7]);
        assert_eq!(titles(&reply).ok(), Some((vec![7], 7)));
    }

    #[test]
    fn evidence_token_has_no_spaces() {
        assert_eq!(token("a\nb"), "a-b");
    }
}
