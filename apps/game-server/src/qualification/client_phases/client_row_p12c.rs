//! P12c client replay capture: real CHARACTER_INFO QuerySelf replies (full and paged) and the
//! refused QueryVisible of another character. The consumer is `apply` in
//! `apps/client/src/features/0097-character-info/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::character_info_panel as info;

fn frame(
    reply: &oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse,
) -> serde_json::Value {
    serde_json::json!({
        "command_id": reply.command_id,
        "server_sequence": reply.server_sequence,
        "status": format!("{:?}", reply.status),
        "payload": reply.payload,
    })
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

/// The summary values, deaths and PvP kills of a QuerySelf reply.
fn summary(bytes: &[u8]) -> TestResult<String> {
    if bytes.first() != Some(&0) || bytes.len() < 71 {
        return Err(format!("malformed character-info reply len={}", bytes.len()).into());
    }
    let mut values = Vec::new();
    for at in 0..8 {
        values.push(i64::from_le_bytes(bytes[1 + at * 8..9 + at * 8].try_into()?).to_string());
    }
    Ok(format!(
        "values={}|deaths={}|kills={}",
        values.join(","),
        u16::from_le_bytes(bytes[65..67].try_into()?),
        u16::from_le_bytes(bytes[67..69].try_into()?)
    ))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<(String, bool)> {
    let mut client = connect_character(clients, 0x31, 0).await?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let mut summaries = Vec::new();
    let mut frames = Vec::new();
    for (name, offset, limit) in [("query", 0, 50), ("paged", 1, 1)] {
        let page = info::PageRequest::new(offset, limit).ok_or("page")?;
        let reply = send(&mut client, info::CharacterInfoOperation::QuerySelf(page)).await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!("QuerySelf {name} status={:?}", reply.status).into());
        }
        frames.push(frame(&reply));
        summaries.push(summary(&reply.payload)?);
    }
    let target = clients.characters[1];
    let refused = send(
        &mut client,
        info::CharacterInfoOperation::QueryVisible {
            character: target,
            page: info::PageRequest::new(0, 1).ok_or("page")?,
        },
    )
    .await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!("QueryVisible was not rejected without payload: {refused:?}").into());
    }
    let target: String = target.iter().map(|byte| format!("{byte:02x}")).collect();
    let expected = format!(
        "query={}\npaged={}\nvisible_target={target}\nrefused_status=Rejected\n",
        summaries[0], summaries[1]
    );
    frames.push(frame(&refused));
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("P12c capture directory absent")?;
    let capture = serde_json::json!({ "session": session, "frames": frames, "expected": expected });
    evidence(&format!(
        "P12C_CLIENT_CAPTURE session={session} last_command={} last_sequence={}",
        refused.command_id, refused.server_sequence
    ));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(Path::new(&root).join("p12c-capture.json"))?;
    serde_json::to_writer(file, &capture)?;

    let mut replayed = false;
    if let Some(binary) = std::env::var_os("E2E_P12C_REPLAY_BIN") {
        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new(binary)
                .args([
                    "e2e_p12c_replays_paged_summary_and_refused_visible_query",
                    "--nocapture",
                ])
                .env("E2E_ALL_CAPTURE_DIR", root)
                .env("E2E_P12C_CAPTURE_SESSION", session)
                .output()
        })
        .await??;
        let stdout = String::from_utf8(output.stdout)?;
        let result = stdout
            .lines()
            .find(|line| line.starts_with("test result: ok. 1 passed; 0 failed;"));
        if !output.status.success()
            || !stdout.lines().any(|line| {
                line.starts_with("test ")
                    && line.ends_with(
                        "::e2e_p12c_replays_paged_summary_and_refused_visible_query ... ok",
                    )
            })
            || result.is_none()
        {
            return Err(format!(
                "P12c client replay failed: {stdout} {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        evidence(&format!(
            "P12C_CLIENT_REPLAY {}",
            result.ok_or("P12c replay result absent")?
        ));
        replayed = true;
    }
    Ok((expected.replace('\n', ";"), replayed))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok((expected, replayed))) => evidence(&format!(
            "e2e_all scenario=P12c.client status={} evidence={expected}",
            if replayed { "PASS" } else { "CAPTURED" }
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P12c.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P12c.client status=FAIL evidence=timeout"),
    }
}
