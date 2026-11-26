//! P18 client replay capture: the real Forge JSON replies for HISTORY, FUSION, DUST_TO_SLIVERS and
//! DUST_LIMIT. The consumer is `apply_result` and `panel_text` in
//! `crates/client-ui/src/features/0152-item-forge`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::item_forge::{
    CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_ITEM_FORGE, Operation, Request,
};
use std::num::NonZeroU64;

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    std::fs::write(root.join(name), bytes)?;
    Ok(())
}

fn occurrence(session: oteryn_protocol_oteryn::GameSessionId, ordinal: u8) -> [u8; 16] {
    use sha2::Digest;
    let digest = sha2::Sha256::new()
        .chain_update(b"E2E-P18-CLIENT/occurrence")
        .chain_update(session.as_bytes())
        .chain_update([ordinal])
        .finalize();
    let mut value = [0; 16];
    value.copy_from_slice(&digest[..16]);
    value[6] = (value[6] & 15) | 0x70;
    value[8] = (value[8] & 63) | 0x80;
    value
}

fn same_item_pair(client: &DevClientSession) -> TestResult<(NonZeroU64, NonZeroU64)> {
    let inventory = client.inventory().ok_or("inventory frame missing")?;
    let first = inventory
        .entries
        .iter()
        .chain(inventory.main_backpack.iter())
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
        .next()
        .ok_or("no carried item handle")?
        .handle;
    Ok((first, first))
}

async fn forge(
    client: &mut DevClientSession,
    operation: Operation,
    occurrence: [u8; 16],
    pair: Option<(NonZeroU64, NonZeroU64)>,
    history_limit: u8,
) -> TestResult<(Vec<u8>, serde_json::Value)> {
    let payload = Request {
        operation,
        occurrence,
        first_item: pair.map(|value| value.0),
        second_item: pair.map(|value| value.1),
        boost: false,
        protect: false,
        history_limit,
    }
    .encode()
    .ok_or("forge request was not canonical")?;
    let reply = client
        .session_mut()
        .gated_command(CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_ITEM_FORGE, &payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("{operation:?} status={:?}", reply.status).into());
    }
    let body: serde_json::Value = serde_json::from_slice(&reply.payload)?;
    Ok((reply.payload, body))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("Forge capture directory missing")?;
    let root = Path::new(&root);
    let mut client = connect_world(clients, 0x44).await?;
    let session = client.join_snapshot().game_session_id;
    let pair = same_item_pair(&client)?;
    let mut expected = String::new();
    let (history_bytes, history) =
        forge(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    let rows = history
        .as_array()
        .ok_or("HISTORY payload was not an array")?;
    if !rows.is_empty() {
        return Err("fresh Forge history was not empty".into());
    }
    capture(root, "p18-history.bin", &history_bytes)?;
    expected.push_str(&format!("history_rows={}\n", rows.len()));
    for (ordinal, (name, operation, expected_reason)) in [
        // A repeated item handle is rejected before a Forge pair can be reserved.
        ("fusion", Operation::Fusion, "INVALID_ITEM_HANDLE"),
        (
            "dust_to_slivers",
            Operation::DustToSlivers,
            "INSUFFICIENT_DUST",
        ),
        ("dust_limit", Operation::DustLimit, "INSUFFICIENT_DUST"),
    ]
    .into_iter()
    .enumerate()
    {
        // The item-forge-v1 contract gives new attempts and history a shared 490 ms cooldown.
        client.service_liveness(Duration::from_millis(490)).await?;
        let pair = operation.requires_pair().then_some(pair);
        let (bytes, body) = forge(
            &mut client,
            operation,
            occurrence(session, u8::try_from(ordinal + 1)?),
            pair,
            0,
        )
        .await?;
        let succeeded = body["succeeded"].as_bool().ok_or("reply lacks succeeded")?;
        let applied = body["applied"].as_bool().ok_or("reply lacks applied")?;
        let reason = body["reason"].as_str().unwrap_or("");
        if succeeded || applied || reason != expected_reason {
            return Err(format!("{name} expected refusal {expected_reason}, got {body}").into());
        }
        capture(root, &format!("p18-{}.bin", name.replace('_', "-")), &bytes)?;
        expected.push_str(&format!(
            "{name}.succeeded={succeeded}\n{name}.applied={applied}\n{name}.reason={reason}\n"
        ));
    }
    capture(root, "p18-expected.txt", expected.as_bytes())?;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("Forge capture logout refused".into());
    }
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_capture scenario=P18.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P18.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P18.client status=FAIL evidence=timeout"),
    }
}
