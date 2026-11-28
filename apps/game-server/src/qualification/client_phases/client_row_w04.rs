//! W04 client replay capture: the real RAID_SCHEDULE_QUERY state reply and the refused malformed
//! query. The consumer is `apply` in `apps/client/src/features/0149-raid-schedule/protocol.rs`.
use super::*;
use oteryn_dev_client::client_runtime::raid_schedule as wire;

use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::io::Write as _;

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
    let after_refusal = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_RAID_SCHEDULE_V1,
            wire::COMMAND_TYPE_RAID_SCHEDULE_QUERY,
            &[],
        )
        .await?;
    if after_refusal.status != CommandStatus::Accepted || after_refusal.payload != reply.payload {
        return Err("raid schedule changed across the malformed refusal".into());
    }
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("missing live capture directory")?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let bytes = serde_json::to_vec(&serde_json::json!({
        "session": session,
        "accepted": (reply.command_id, reply.server_sequence, reply.status as u32, &reply.payload),
        "refused": (refused.command_id, refused.server_sequence, refused.status as u32, &refused.payload),
        "expected": format!(
            "key={}\nrunning={}\nsteps={}\ncreatures={}\n",
            state.raid_key, state.running, state.committed_steps, state.active_creatures
        ),
    }))?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(Path::new(&root).join("w04-exchange.json"))?
        .write_all(&bytes)?;
    let mut digest = String::with_capacity(64);
    for byte in Sha256::digest(&bytes) {
        write!(digest, "{byte:02x}")?;
    }
    Ok(digest)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W04.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W04.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W04.client status=FAIL evidence=timeout"),
    }
}
