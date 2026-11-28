//! Capture modern Weekly and Bounty replies for the production client consumer.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{logout::LogoutResult, task_board as wire};
use serde_json::{Value, json};

pub(super) async fn exchange(
    client: &mut DevClientSession,
    action: u8,
    selection: u16,
) -> TestResult<(wire::full::Reply, Value)> {
    let request = wire::full::encode_request(&wire::full::Request {
        action,
        selection,
        race: String::new(),
    })
    .map_err(|error| format!("Task Board request: {error:?}"))?;
    let result = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_TASK_BOARD_V1,
            wire::COMMAND_TYPE_TASK_BOARD,
            &request,
        )
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("Task Board action={action} result={:?}", result.status).into());
    }
    let reply = wire::full::decode_reply(&result.payload)
        .map_err(|error| format!("Task Board action={action} reply={error:?}"))?;
    let capture = json!({
        "request": request,
        "command": result.command_id,
        "sequence": result.server_sequence,
        "status": format!("{:?}", result.status),
        "payload": result.payload,
    });
    Ok((reply, capture))
}

pub(super) async fn connect_seeded(
    clients: &SeamClients<'_>,
    tag: u8,
    row: &str,
) -> TestResult<DevClientSession> {
    use oteryn_protocol_oteryn::cyclopedia_character as general;
    character_prep::declare_seeded(row, "level-8-knight", "Task Board Beginner eligibility");
    let mut client = connect_character(clients, tag, 0).await?;
    let previous = client.join_snapshot().game_session_id;
    request_fixture_control_at(
        "NODE_BOOT_CONTROL_SOCKET",
        uuid_text(previous.as_bytes()),
        "--profile",
        "p23c-knight".into(),
    )
    .await?;
    let result = client.session_mut().logout().await?;
    if result.result != Some(LogoutResult::Accepted) {
        return Err(format!("step=preparation-logout observed={result:?}").into());
    }
    drop(client);
    let mut client = connect_character(clients, tag, 0).await?;
    let vocation_selected = general_stats::selected(&client);
    let stats = client
        .session_mut()
        .gated_command(
            general::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            general::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[1, 0, 0, 1],
        )
        .await?;
    let vocation = general_stats::validate(&stats.payload, vocation_selected)?;
    let level = u32::from_le_bytes(stats.payload[9..13].try_into()?);
    let vitals = client.actor_vitals().ok_or("seeded vitals missing")?;
    if client.join_snapshot().game_session_id == previous
        || stats.status != CommandStatus::Accepted
        || level != 8
        || (vocation_selected && vocation != general::vocation_id("knight"))
        || (vitals.max_health, vitals.max_mana) != (185, 90)
    {
        return Err(format!(
            "step=prepared-stats level={level} vocation={vocation:?} vitals={vitals:?}"
        )
        .into());
    }
    evidence("P23C_CHECK step=prepared-stats level=8 max-health=185 max-mana=90");
    Ok(client)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let directory = Path::new(&required("OTERYN_P23C_CAPTURE_DIR")?).to_path_buf();
    let run = directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("invalid capture directory")?;
    let character = character_prep::dedicated_named_character(
        clients,
        "NODE_BOOT_P23C_ACCOUNT",
        Some("Bounty Task Hunter"),
    )
    .await?;
    let clients = &character.clients(clients);
    let mut client = connect_seeded(clients, 0x45, "P23c.client").await?;
    let session = format!("{:?}", client.join_snapshot().game_session_id);
    let (initial, query) = exchange(&mut client, 4, 0).await?;
    let (weekly, start) = exchange(&mut client, 5, 0).await?;
    if initial.weekly.started
        || weekly.bounty.state != 0
        || !weekly.bounty.offers.is_empty()
        || weekly.refusal != 0
        || !weekly.weekly.started
        || weekly.weekly.difficulty != 0
        || weekly.weekly.kills.is_empty()
        || weekly.weekly.pending_points != 0
        || weekly.points != initial.points
    {
        return Err(format!("step=weekly-start observed={weekly:?}").into());
    }
    let (opened, open) = exchange(&mut client, 7, 0).await?;
    let offer = opened.bounty.offers.first().ok_or_else(|| {
        format!(
            "step=bounty-open refusal={} state={} offers=0",
            opened.refusal, opened.bounty.state
        )
    })?;
    let (selected, select) = exchange(&mut client, 8, 0).await?;
    if initial.refusal != 0
        || initial.bounty.active.is_some()
        || opened.refusal != 0
        || opened.bounty.state != 1
        || opened.bounty.active.is_some()
        || offer.required == 0
        || offer.points == 0
        || selected.refusal != 0
        || selected.bounty.state != 2
        || selected.bounty.selected != 1
        || selected.bounty.current != 0
        || selected.bounty.active.as_ref() != Some(offer)
        || selected.weekly != weekly.weekly
        || selected.points != initial.points
        || selected.bounty_points != initial.bounty_points
    {
        return Err("step=bounty-select did not activate exactly the first live offer".into());
    }
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(LogoutResult::Accepted) {
        return Err(format!("step=client-logout observed={logout:?}").into());
    }
    let mut relog = connect_character(clients, 0x46, 0).await?;
    let (persisted, _) = exchange(&mut relog, 4, 0).await?;
    let relog_session = format!("{:?}", relog.join_snapshot().game_session_id);
    if session == relog_session || persisted != selected {
        return Err(format!("step=client-relog observed={persisted:?}").into());
    }
    let logout = relog.session_mut().logout().await?;
    if logout.result != Some(LogoutResult::Accepted) {
        return Err(format!("step=client-relog-logout observed={logout:?}").into());
    }
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("p23c.json"))?;
    serde_json::to_writer(
        file,
        &json!({
            "run": run,
            "session": session,
            "commands": [query, start, open, select],
        }),
    )?;
    Ok(run.to_owned())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(run)) => evidence(&format!(
            "e2e_capture scenario=P23c.client status=CAPTURED run={run}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P23c.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P23c.client status=FAIL evidence=timeout"),
    }
}
