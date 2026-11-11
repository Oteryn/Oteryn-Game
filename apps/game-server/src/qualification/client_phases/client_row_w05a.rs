//! W05a client replay capture: the real domain-40 server-save countdown snapshot delivered at
//! admission, the locally refused player reset command and, when the server's planned reset enters its
//! five-minute window during the poll, the countdown delta. The session retains typed deltas, so a
//! delta capture is the server-delivered value encoded with the production encoder (the C09
//! convention). The consumer is `Session::apply_state_delta` via
//! `crates/session/src/server_save_countdown.rs`.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::server_save_countdown as wire;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut client = connect_world(clients, 0xfe).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_SERVER_SAVE_COUNTDOWN_V1)
    {
        return Err("admission did not select capability 75".into());
    }
    let snapshot = client
        .session_mut()
        .gated_snapshot(wire::STATE_DOMAIN_SERVER_SAVE_COUNTDOWN)
        .ok_or("domain-40 snapshot missing")?
        .clone();
    if snapshot.revision != 0 || snapshot.payload != wire::SERVER_SAVE_COUNTDOWN_SNAPSHOT {
        return Err(format!("unexpected countdown snapshot {snapshot:?}").into());
    }
    client.take_events();
    client.service_liveness(Duration::from_secs(7)).await?;
    let notice = client
        .take_events()
        .into_iter()
        .find_map(|event| match event {
            SessionEvent::ServerSaveCountdown(delta) => Some(delta),
            _ => None,
        });
    capture("w05a-snapshot.bin", &snapshot.payload)?;
    let mut expected = format!("snapshot_revision={}\n", snapshot.revision);
    if let Some(delta) = notice {
        if delta.new_revision != delta.base_revision + 1
            || delta.value.ending_epoch == 0
            || !(1..=wire::SERVER_SAVE_COUNTDOWN_MAX_MINUTES).contains(&delta.value.minutes_remaining)
        {
            return Err(format!("invalid countdown delta {delta:?}").into());
        }
        capture(
            "w05a-countdown.bin",
            &wire::encode_server_save_countdown(delta.value).map_err(|error| format!("{error:?}"))?,
        )?;
        expected.push_str(&format!(
            "countdown=present\nbase_revision={}\nnew_revision={}\nending_epoch={}\nminutes_remaining={}\n",
            delta.base_revision, delta.new_revision, delta.value.ending_epoch, delta.value.minutes_remaining
        ));
    } else {
        expected.push_str("countdown=absent\n");
    }
    capture("w05a-expected.txt", expected.as_bytes())?;
    let refusal = client
        .session_mut()
        .gated_command(wire::CAPABILITY_SERVER_SAVE_COUNTDOWN_V1, 1, &[])
        .await;
    if !matches!(&refusal, Err(error) if format!("{error:?}") == "CapabilityNotSelected { capability: 75 }") {
        return Err(format!("receive-only reset gate did not refuse locally: {refusal:?}").into());
    }

    Ok(expected.replace('\n', ";"))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W05a.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W05a.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W05a.client status=FAIL evidence=timeout"),
    }
}
