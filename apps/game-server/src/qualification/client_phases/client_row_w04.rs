//! W04 client replay capture: the real RAID_SCHEDULE_QUERY state reply and the refused malformed
//! query. The consumer is `apply` in `apps/client/src/features/0149-raid-schedule/protocol.rs`.
use super::*;
use oteryn_dev_client::client_runtime::raid_schedule as wire;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("W04_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0xfd).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_RAID_SCHEDULE_V1)
    {
        return Err("admission did not offer raid-schedule capability 149".into());
    }
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_RAID_SCHEDULE_V1,
            wire::COMMAND_TYPE_RAID_SCHEDULE_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("RAID_SCHEDULE_QUERY status={:?}", reply.status).into());
    }
    let state = wire::decode_raid_schedule_state(&reply.payload)
        .map_err(|error| format!("raid state malformed: {error:?}"))?;
    let refused = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_RAID_SCHEDULE_V1,
            wire::COMMAND_TYPE_RAID_SCHEDULE_QUERY,
            &[0],
        )
        .await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!("malformed query was not refused: {refused:?}").into());
    }
    capture("w04-state.bin", &reply.payload);
    let expected = format!(
        "key={}\nrunning={}\nsteps={}\ncreatures={}\n",
        state.raid_key, state.running, state.committed_steps, state.active_creatures
    );
    capture("w04-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W04.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W04.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W04.client status=FAIL evidence=timeout"),
    }
}
