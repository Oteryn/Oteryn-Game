//! Live Hazard unlock, selection, refusal and durable reload evidence.
use super::*;
use oteryn_dev_client::client_runtime::boss_encounter as wire;
use std::io::Write;

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    std::fs::File::create_new(root.join(name))
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|error| format!("W10_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"))?;
    Ok(())
}

async fn hazard(
    client: &mut oteryn_dev_client::DevClientSession,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BOSS_ENCOUNTER_V1,
            wire::hazard::COMMAND_TYPE_HAZARD,
            payload,
        )
        .await?)
}

async fn probe(
    client: &mut oteryn_dev_client::DevClientSession,
    root: &Path,
    name: &str,
    request: &[u8],
    status: CommandStatus,
    expected: &[u8],
) -> TestResult {
    let session = client.join_snapshot().game_session_id;
    let reply = hazard(client, request).await?;
    if status == CommandStatus::Accepted {
        require_mechanic_acceptance(reply.status, name)?;
    }
    if client.join_snapshot().game_session_id != session
        || reply.status != status
        || reply.payload != expected
    {
        return Err(
            format!("Hazard {name} expected {status:?}/{expected:?}, got {reply:?}").into(),
        );
    }
    capture(root, &format!("w10-{name}-request.bin"), request)?;
    capture(root, &format!("w10-{name}.bin"), &reply.payload)?;
    capture(
        root,
        &format!("w10-{name}-status.bin"),
        &(reply.status as u32).to_be_bytes(),
    )?;
    let binding = serde_json::json!({
        "session_id": uuid_text(session.as_bytes()),
        "command_id": reply.command_id,
        "server_sequence": reply.server_sequence,
        "status": reply.status as u32,
    });
    capture(
        root,
        &format!("w10-{name}.status.json"),
        &serde_json::to_vec(&binding)?,
    )
}

async fn logout(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let (result, detail, _, _) = logout_once(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("W10.client first logout refused: {detail}").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("missing live capture directory")?;
    let root = Path::new(&root);
    let world = full_row_w10::primal_world(clients)?;
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_W10_ACCOUNT").await?;
    let clients = &dedicated.clients(clients);
    character_prep::declare_seeded(
        "W10.client",
        "level=500",
        "authored Primal admission minimum",
    );
    let mut client = connect_character(clients, 0xd0, 0).await?;
    let session = client.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(session.as_bytes()),
        "--profile",
        "w10-primal".into(),
    )
    .await?;
    logout(&mut client).await?;
    drop(client);
    full_row_w10::stage_primal_challenger(clients).await?;
    let mut client = connect_character(clients, 0xd1, 0).await?;
    if client.join_snapshot().game_session_id == session
        || tile(client.world_spatial().actor_position)?
            != (TilePos {
                x: 33548,
                y: 32751,
                floor: -14,
            })
    {
        return Err(
            "Primal challenger did not freshly admit at the authored lobby approach".into(),
        );
    }
    full_row_w10::enter_primal(&mut client, world, true).await?;
    let instance_session = client.join_snapshot().game_session_id;
    probe(
        &mut client,
        root,
        "initial",
        &[],
        CommandStatus::Accepted,
        &[1, 1],
    )
    .await?;
    full_row_w10::defeat_primal(&mut client).await?;
    probe(
        &mut client,
        root,
        "query",
        &[],
        CommandStatus::Accepted,
        &[2, 1],
    )
    .await?;
    probe(
        &mut client,
        root,
        "select",
        &[2],
        CommandStatus::Accepted,
        &[2, 2],
    )
    .await?;
    probe(
        &mut client,
        root,
        "locked",
        &[3],
        CommandStatus::Rejected,
        &[],
    )
    .await?;
    probe(
        &mut client,
        root,
        "unchanged",
        &[],
        CommandStatus::Accepted,
        &[2, 2],
    )
    .await?;
    if client.join_snapshot().game_session_id != instance_session {
        return Err("Hazard probes crossed an Instance session boundary".into());
    }
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_character(clients, 0xd2, 0).await?;
    if client.join_snapshot().game_session_id == instance_session {
        return Err("Hazard durability check reused the original game session".into());
    }
    probe(
        &mut client,
        root,
        "relog",
        &[],
        CommandStatus::Accepted,
        &[2, 2],
    )
    .await?;
    capture(
        root,
        "w10-expected.txt",
        b"query_unlocked=2\nquery_selected=1\nselect_unlocked=2\nselect_selected=2\n",
    )?;
    logout(&mut client).await?;
    Ok("hazard=2/1->2/2;level3=rejected;unchanged=2/2;relog=2/2".into())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W10.client status=FAIL evidence=original-command-result-frame-unavailable journey={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W10.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W10.client status=FAIL evidence=timeout"),
    }
}
