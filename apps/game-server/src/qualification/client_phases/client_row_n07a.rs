//! N07a client replay capture: the real CHARACTER_INFO QuerySelf, accepted Select, refused
//! Select of an unearned title and follow-up QuerySelf that feed the Cyclopedia title selector.
//! The consumer is `apply` in `apps/client/src/features/0097-character-info/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::character_info_panel as info;

use std::io::Write;

fn capture(
    session: [u8; 16],
    reply: &oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse,
) -> (String, u64, u64, u8, &[u8]) {
    (uuid_text(&session), reply.command_id, reply.server_sequence, reply.status as u8, &reply.payload)
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("N07a logout result={:?}", reply.result).into());
    }
    Ok(())
}

async fn send(
    client: &mut DevClientSession,
    operation: info::CharacterInfoOperation,
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
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
        return Err(format!("malformed character-info reply len={}", bytes.len()).into());
    }
    let count = usize::from(bytes[69]);
    if bytes.len() != 71 + count {
        return Err(format!("title projection count={count} len={}", bytes.len()).into());
    }
    Ok((bytes[70..70 + count].to_vec(), bytes[70 + count]))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("missing N07a capture directory")?;
    character_prep::declare_seeded("N07a.client", "sorcerer-level-50", "Trolltrasher eligibility");
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_N07A_ACCOUNT").await?;
    let scope = identity.clients(clients);
    let mut prep = connect_character(&scope, 0xf7, 0).await?;
    request_fixture_control(
        uuid_text(prep.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "n07a-titles".into(),
    ).await?;
    logout(&mut prep).await?;
    drop(prep);

    let mut client = connect_character(&scope, 0xf8, 0).await?;
    let session = *client.join_snapshot().game_session_id.as_bytes();
    let level = full_e2ef7::player_level(&mut client).await?;
    let (hp, mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer", 50, oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    ).ok_or("missing level-50 sorcerer vitals")?;
    if level != 50 || !client.actor_vitals().is_some_and(|v| v.max_health == hp && v.max_mana == mana) {
        return Err(format!("N07a level={level} vitals={:?}", client.actor_vitals()).into());
    }
    let operation = info::CharacterInfoOperation::QuerySelf(info::PageRequest::new(0, 1).ok_or("page")?);
    let query = send(&mut client, operation).await?;
    if query.status != CommandStatus::Accepted {
        return Err(format!("QuerySelf status={:?}", query.status).into());
    }
    let (earned, before) = titles(&query.payload)?;
    let chosen = info::CharacterTitle::Trolltrasher;
    let unearned = info::CharacterTitle::AdeptArmorer;
    if before != 0 || !earned.contains(&chosen.id()) || earned.contains(&unearned.id()) {
        return Err(format!("N07a pre-state earned={earned:?} selected={before}").into());
    }
    let select = send(&mut client, info::CharacterInfoOperation::Select(Some(chosen))).await?;
    if select.status != CommandStatus::Accepted || select.payload.len() != 9 || select.payload[0] != 3 {
        return Err(format!("N07a Select reply {select:?}").into());
    }
    let selected = send(&mut client, operation).await?;
    if selected.status != CommandStatus::Accepted || titles(&selected.payload)? != (earned.clone(), chosen.id()) {
        return Err(format!("N07a selected state {selected:?}").into());
    }
    let refused = send(&mut client, info::CharacterInfoOperation::Select(Some(unearned))).await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!("N07a unearned Select reply {refused:?}").into());
    }
    let after = send(&mut client, operation).await?;
    if after.status != CommandStatus::Accepted || after.payload != selected.payload {
        return Err(format!("N07a state changed after refusal {after:?}").into());
    }
    logout(&mut client).await?;
    drop(client);
    let mut again = connect_character(&scope, 0xf9, 0).await?;
    let fresh = *again.join_snapshot().game_session_id.as_bytes();
    if session == fresh {
        return Err("N07a relog reused the original session".into());
    }
    let persisted = send(&mut again, operation).await?;
    if persisted.status != CommandStatus::Accepted || titles(&persisted.payload)? != (earned, chosen.id()) {
        return Err(format!("N07a persisted state {persisted:?}").into());
    }
    logout(&mut again).await?;
    let file = Path::new(&root).join("n07a-wire.json");
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(file)?;
    serde_json::to_writer(&mut file, &[
        capture(session, &query), capture(session, &select), capture(session, &selected),
        capture(session, &refused), capture(session, &after), capture(fresh, &persisted),
    ])?;
    file.flush()?;
    Ok(format!("session={} relog={} selected_before=0 selected_after={} refused_title={}",
        uuid_text(&session), uuid_text(&fresh), chosen.id(), unearned.id()))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=N07a.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=N07a.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=N07a.client status=FAIL evidence=timeout"),
    }
}
