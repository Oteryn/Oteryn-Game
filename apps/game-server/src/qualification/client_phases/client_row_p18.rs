//! P18 client replay capture: the real Forge JSON replies for HISTORY, FUSION, DUST_TO_SLIVERS and
//! DUST_LIMIT. The consumer is `apply_result` and `panel_text` in
//! `crates/client-ui/src/features/0152-item-forge`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::item_forge::{
    CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_ITEM_FORGE, Operation, Request,
};
use std::num::NonZeroU64;

use oteryn_dev_client::client_runtime::server_mechanic::{
    CommandStatus as UiStatus, ServerMechanicResponse,
};
use oteryn_dev_client::client_ui::item_forge::{
    ItemForgePanel, apply_result, command, panel_text, reply, request, resource_dialog::Content,
};
use oteryn_protocol_oteryn::item_forge::COMMAND_TYPE_FORGE_DUST_QUERY;

type LiveReply = (u64, u64, u32, Vec<u8>, Vec<u8>);

async fn dust(client: &mut DevClientSession) -> TestResult<(u64, u64)> {
    let result = client
        .session_mut()
        .gated_command(CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_FORGE_DUST_QUERY, &[])
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("dust query refused: {:?}", result.status).into());
    }
    let body: serde_json::Value = serde_json::from_slice(&result.payload)?;
    Ok((
        body["current_dust"]
            .as_u64()
            .ok_or("dust balance missing")?,
        body["dust_limit"].as_u64().ok_or("dust limit missing")?,
    ))
}

fn replay(root: &Path, session: [u8; 16], frames: &[LiveReply]) -> TestResult {
    use std::io::Write as _;
    let path = root.join("p18-live.json");
    let mut file = std::fs::File::create_new(&path)?;
    serde_json::to_writer(&mut file, &(session, frames))?;
    file.flush()?;
    drop(file);
    let (captured_session, captured): ([u8; 16], Vec<LiveReply>) =
        serde_json::from_reader(std::fs::File::open(path)?)?;
    if captured_session != session || captured != frames {
        return Err("Forge capture session or command sequence mismatch".into());
    }
    let mut panel = ItemForgePanel {
        selected: true,
        ..Default::default()
    };
    for (index, (command_id, server_sequence, status, payload, submitted_bytes)) in
        captured.into_iter().enumerate()
    {
        if status != UiStatus::Accepted as u32
            || command_id == 0
            || server_sequence == 0
            || index > 0
                && (command_id <= frames[index - 1].0 || server_sequence <= frames[index - 1].1)
        {
            return Err(format!("invalid Forge reply metadata at frame {index}").into());
        }
        let submitted =
            Request::decode(&submitted_bytes).ok_or("captured Forge request invalid")?;
        request(&mut panel, command(submitted_bytes)).ok_or("Forge UI refused submission")?;
        if !panel.pending || panel.submitted != Some(submitted) {
            return Err("Forge UI did not track the submitted request".into());
        }
        let previous_dialog = panel.resource_dialog.serial;
        apply_result(
            &mut panel,
            &ServerMechanicResponse {
                command_id,
                server_sequence,
                status: UiStatus::Accepted,
                payload: payload.clone(),
            },
        );
        if panel.pending
            || panel.submitted.is_some()
            || panel.retry.is_some()
            || panel.feedback.is_some()
            || panel.response != payload
        {
            return Err("Forge panel did not consume the live reply".into());
        }
        let expected = match (index, &panel.decoded) {
            (0, Some(reply::Reply::History(rows))) if rows.is_empty() => "No forge operations yet",
            (1, Some(reply::Reply::Result(receipt)))
                if receipt.reason.as_deref() == Some("INVALID_ITEM_HANDLE") =>
            {
                "Fusion: failed; no changes applied (INVALID_ITEM_HANDLE)"
            }
            (2, Some(reply::Reply::Result(receipt)))
                if receipt.reason.as_deref() == Some("INSUFFICIENT_DUST") =>
            {
                "DustToSlivers: failed; no changes applied (INSUFFICIENT_DUST)"
            }
            (3, Some(reply::Reply::Result(receipt)))
                if receipt.reason.as_deref() == Some("INSUFFICIENT_DUST") =>
            {
                "DustLimit: failed; no changes applied (INSUFFICIENT_DUST)"
            }
            _ => return Err(format!("unexpected Forge panel decode at frame {index}").into()),
        };
        if index != 0 {
            let reason = if index == 1 {
                "INVALID_ITEM_HANDLE"
            } else {
                "INSUFFICIENT_DUST"
            };
            let body = serde_json::from_slice(&payload)?;
            check_refusal(&body, reason)?;
        }
        let dialog_matches = match (&panel.resource_dialog.content, &panel.decoded) {
            (
                Some(Content::Exaltation {
                    receipt,
                    target: None,
                    donor: None,
                }),
                Some(reply::Reply::Result(decoded)),
            ) if index == 1 => receipt == decoded,
            (Some(Content::Result(dialog)), Some(decoded)) if index != 1 => dialog == decoded,
            _ => false,
        };
        if panel.resource_dialog.serial == previous_dialog || !dialog_matches {
            return Err(
                format!("Forge dialog did not show the submitted result at frame {index}").into(),
            );
        }
        if panel_text(&panel).lines().nth(3) != Some(expected) {
            return Err(format!("unexpected Forge panel text at frame {index}").into());
        }
    }
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
) -> TestResult<(LiveReply, serde_json::Value)> {
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
    Ok((
        (
            reply.command_id,
            reply.server_sequence,
            reply.status as u32,
            reply.payload,
            payload.to_vec(),
        ),
        body,
    ))
}

fn check_refusal(body: &serde_json::Value, reason: &str) -> TestResult {
    if body["succeeded"].as_bool() != Some(false)
        || body["applied"].as_bool() != Some(false)
        || body["reason"].as_str() != Some(reason)
        || body["bonus"].as_u64() != Some(0)
        || body.get("result") != Some(&serde_json::Value::Null)
    {
        return Err(format!(
            "expected Forge refusal {reason} with zero bonus and null result, got {body}"
        )
        .into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("Forge capture directory missing")?;
    let root = Path::new(&root);
    let mut client = connect_world(clients, 0x44).await?;
    let session = client.join_snapshot().game_session_id;
    let pair = same_item_pair(&client)?;
    let inventory = client
        .inventory()
        .cloned()
        .ok_or("pre-state inventory missing")?;
    let balance = dust(&mut client).await?;
    if balance != (0, 100) {
        return Err(format!("unexpected Forge pre-state {balance:?}").into());
    }
    let (history_bytes, history) =
        forge(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    let rows = history
        .as_array()
        .ok_or("HISTORY payload was not an array")?;
    if !rows.is_empty() {
        return Err("fresh Forge history was not empty".into());
    }
    let mut frames = vec![history_bytes];
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
        check_refusal(&body, expected_reason).map_err(|error| format!("{name}: {error}"))?;
        if dust(&mut client).await? != balance || client.inventory() != Some(&inventory) {
            return Err(format!("{name} refusal changed inventory or Forge balance").into());
        }
        frames.push(bytes);
    }
    client.service_liveness(Duration::from_millis(490)).await?;
    let (_, final_history) = forge(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    if final_history != history {
        return Err("Forge refusal history changed before logout".into());
    }
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("Forge capture logout refused".into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0x45).await?;
    if fresh.join_snapshot().game_session_id == session
        || fresh
            .inventory()
            .cloned()
            .map(super::full_row_p18::durable_inventory)
            != Some(super::full_row_p18::durable_inventory(inventory))
        || dust(&mut fresh).await? != balance
    {
        return Err("Forge refusal state changed after relog".into());
    }
    fresh.service_liveness(Duration::from_millis(490)).await?;
    let (_, persisted_history) =
        forge(&mut fresh, Operation::QueryHistory, [0; 16], None, 50).await?;
    if persisted_history != history {
        return Err("Forge refusal history changed after relog".into());
    }
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("Forge relog session logout refused".into());
    }
    replay(root, *session.as_bytes(), &frames)?;
    Ok(format!(
        "live-capture-replayed session={}",
        uuid_text(session.as_bytes())
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P18.client status=PASS evidence={expected}"
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
    fn refusal_rejects_value_bonus_and_missing_result() -> TestResult {
        let valid = serde_json::json!({
            "succeeded": false, "applied": false, "reason": "INSUFFICIENT_DUST",
            "bonus": 0, "result": null,
        });
        check_refusal(&valid, "INSUFFICIENT_DUST")?;
        for (field, value) in [
            ("result", serde_json::json!({ "dust": 0 })),
            ("bonus", serde_json::json!(1)),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            assert!(check_refusal(&invalid, "INSUFFICIENT_DUST").is_err());
        }
        let mut missing = valid;
        missing
            .as_object_mut()
            .ok_or("refusal object")?
            .remove("result");
        assert!(check_refusal(&missing, "INSUFFICIENT_DUST").is_err());
        Ok(())
    }
}
