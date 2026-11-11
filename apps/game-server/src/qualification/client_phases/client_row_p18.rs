//! P18 client replay capture: the real Forge JSON replies for HISTORY, FUSION, DUST_TO_SLIVERS and
//! DUST_LIMIT. The consumer is `apply_result` and `panel_text` in
//! `crates/client-ui/src/features/0152-item-forge`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::item_forge::{
    CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_ITEM_FORGE, Operation, Request,
};
use std::num::NonZeroU64;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

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

fn carried_pair(client: &DevClientSession) -> TestResult<(NonZeroU64, NonZeroU64)> {
    let inventory = client.inventory().ok_or("inventory frame missing")?;
    let mut values = inventory
        .entries
        .iter()
        .chain(inventory.main_backpack.iter())
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
        .map(|entry| entry.handle);
    let first = values.next().ok_or("no carried item handle")?;
    let second = values.next().unwrap_or(first);
    Ok((first, second))
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
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut client = connect_world(clients, 0x44).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let session = client.join_snapshot().game_session_id;
        let pair = carried_pair(&client)?;
        let mut expected = String::new();
        let (history_bytes, history) =
            forge(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
        let rows = history
            .as_array()
            .ok_or("HISTORY payload was not an array")?;
        capture("p18-history.bin", &history_bytes)?;
        expected.push_str(&format!("history_rows={}\n", rows.len()));
        for (ordinal, (name, operation)) in [
            ("fusion", Operation::Fusion),
            ("dust_to_slivers", Operation::DustToSlivers),
            ("dust_limit", Operation::DustLimit),
        ]
        .into_iter()
        .enumerate()
        {
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
            let applied = body["applied"].as_bool().unwrap_or(succeeded);
            let reason = body["reason"].as_str().unwrap_or("");
            if !applied && reason.is_empty() && operation != Operation::DustLimit {
                return Err(format!("{name} refusal lacked a reason: {body}").into());
            }
            capture(&format!("p18-{}.bin", name.replace('_', "-")), &bytes)?;
            expected.push_str(&format!(
                "{name}.succeeded={succeeded}\n{name}.applied={applied}\n{name}.reason={reason}\n"
            ));
        }
        capture("p18-expected.txt", expected.as_bytes())?;

        Ok(expected.replace('\n', ";"))
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P18.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P18.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P18.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forge_occurrences_are_fenced_by_session_and_ordinal() -> TestResult {
        let session = oteryn_protocol_oteryn::GameSessionId::decode(&v7(1, 2))?;
        let first = occurrence(session, 0);
        assert_ne!(first, occurrence(session, 1));
        assert_ne!(
            first,
            occurrence(oteryn_protocol_oteryn::GameSessionId::decode(&v7(1, 3))?, 0)
        );
        assert_eq!(first[6] >> 4, 7);
        assert_eq!(first[8] >> 6, 2);
        Ok(())
    }
}
