//! P18 client replay capture: the real Forge JSON replies for HISTORY, FUSION, DUST_TO_SLIVERS and
//! DUST_LIMIT. The consumer is `apply_result` and `panel_text` in
//! `crates/client-ui/src/features/0152-item-forge`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::item_forge::{
    CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_ITEM_FORGE, Operation, Request,
};
use std::num::NonZeroU64;

use oteryn_protocol_oteryn::item_forge::COMMAND_TYPE_FORGE_DUST_QUERY;

type LiveReply = (u64, u64, u32, Vec<u8>, Vec<u8>);

pub(super) async fn dust(client: &mut DevClientSession) -> TestResult<(u64, u64)> {
    Ok(dust_capture(client).await?.0)
}

async fn dust_capture(client: &mut DevClientSession) -> TestResult<((u64, u64), LiveReply)> {
    let result = client
        .session_mut()
        .gated_command(CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_FORGE_DUST_QUERY, &[])
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("dust query refused: {:?}", result.status).into());
    }
    let body: serde_json::Value = serde_json::from_slice(&result.payload)?;
    let balance = (
        body["current_dust"]
            .as_u64()
            .ok_or("dust balance missing")?,
        body["dust_limit"].as_u64().ok_or("dust limit missing")?,
    );
    Ok((
        balance,
        (
            result.command_id,
            result.server_sequence,
            result.status as u32,
            result.payload,
            Vec::new(),
        ),
    ))
}

fn write_capture(root: &Path, session: [u8; 16], frames: &[LiveReply]) -> TestResult {
    use std::io::Write as _;
    if frames.len() != 8 {
        return Err(format!(
            "Forge capture requires eight live frames, got {}",
            frames.len()
        )
        .into());
    }
    let mut file = std::fs::File::create_new(root.join("p18-live.json"))?;
    serde_json::to_writer(&mut file, &(session, frames))?;
    file.flush()?;
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

fn distinct_dagger_pair(
    client: &DevClientSession,
    definition: std::num::NonZeroU32,
) -> TestResult<(NonZeroU64, NonZeroU64)> {
    let inventory = client.inventory().ok_or("inventory frame missing")?;
    let mut daggers = inventory
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == definition && entry.count.get() == 1);
    let first = daggers.next().ok_or("first Forge dagger missing")?.handle;
    let second = daggers.next().ok_or("second Forge dagger missing")?.handle;
    if first == second || daggers.next().is_some() {
        return Err("expected exactly two distinct Forge daggers".into());
    }
    Ok((first, second))
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

fn check_funded(body: &serde_json::Value, operation: Operation) -> TestResult {
    let expected = match operation {
        Operation::DustToSlivers => serde_json::json!({
            "operation":"dust_to_slivers","succeeded":true,"applied":true,"reason":null,"bonus":0,
            "result":{"first_item":null,"first_tier":null,"second_item":null,"second_tier":null,
            "dust":40,"dust_limit":100,"gold_spent":0,"cores_spent":0,
            "slivers_spent":0,"slivers_created":3,"cores_created":0}
        }),
        Operation::DustLimit => serde_json::json!({
            "operation":"dust_limit","succeeded":true,"result":{"dust":15,"dust_limit":120}
        }),
        _ => return Err("unexpected funded Forge action".into()),
    };
    if *body != expected {
        return Err(
            format!("funded Forge {operation:?} result: expected {expected}, got {body}").into(),
        );
    }
    Ok(())
}

fn check_funded_inventory(
    before: &oteryn_protocol_oteryn::item_view::CharacterInventory,
    mut after: oteryn_protocol_oteryn::item_view::CharacterInventory,
    operation: Operation,
    sliver: std::num::NonZeroU32,
) -> TestResult {
    if operation == Operation::DustToSlivers {
        after
            .entries
            .retain(|entry| entry.item_definition_ref != sliver);
    }
    if *before != after {
        return Err(format!("funded {operation:?} changed unrelated inventory").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("Forge capture directory missing")?;
    let root = Path::new(&root);
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_Q_P18_ACCOUNT").await?;
    let scope = dedicated.clients(clients);
    let clients = &scope;
    let mut fixture = connect_character(clients, 0x43, 0).await?;
    character_prep::grant_item(&fixture, 3267, 2).await?;
    if fixture.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("Forge dagger fixture logout refused".into());
    }
    drop(fixture);
    let mut client = connect_character(clients, 0x44, 0).await?;
    let session = client.join_snapshot().game_session_id;
    let definitions = provisioning::item_index()?;
    let pair = distinct_dagger_pair(&client, provisioning::item_reference(&definitions, 3267)?)?;

    let balance = dust(&mut client).await?;
    if balance != (100, 100) {
        if balance == (0, 100) {
            let (_, before_history) =
                forge(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
            if before_history != serde_json::json!([]) {
                return Err("unfunded distinct-pair Fusion requires empty history".into());
            }
            client.service_liveness(Duration::from_millis(490)).await?;
            let before = client
                .inventory()
                .cloned()
                .ok_or("Fusion pre-state inventory missing")?;
            let (_, refusal) = forge(
                &mut client,
                Operation::Fusion,
                occurrence(session, 9),
                Some(pair),
                0,
            )
            .await?;
            check_refusal(&refusal, "INSUFFICIENT_DUST")?;
            if dust(&mut client).await? != balance || client.inventory() != Some(&before) {
                return Err("unfunded distinct-pair Fusion changed state".into());
            }
            if client.session_mut().logout().await?.result
                != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
            {
                return Err("unfunded distinct-pair Fusion logout refused".into());
            }
            drop(client);
            let mut fresh = connect_character(clients, 0x45, 0).await?;
            let normalize = super::full_row_p18::durable_inventory;
            if fresh.join_snapshot().game_session_id == session
                || fresh.inventory().cloned().map(normalize) != Some(normalize(before))
                || dust(&mut fresh).await? != balance
            {
                return Err("unfunded distinct-pair Fusion changed durable state".into());
            }
            let (_, history) =
                forge(&mut fresh, Operation::QueryHistory, [0; 16], None, 50).await?;
            if history != before_history {
                return Err("unfunded distinct-pair Fusion wrote a history entry".into());
            }
            if fresh.session_mut().logout().await?.result
                != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
            {
                return Err("unfunded distinct-pair Fusion relog logout refused".into());
            }
            evidence(
                "P18_CLIENT_DISTINCT_PAIR_CHECK refusal=INSUFFICIENT_DUST durable_unchanged=true",
            );
        }
        return Err(format!(
            "funded Forge requires 100 earned dust at cap 100; observed {balance:?}"
        )
        .into());
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
    let sliver_key = "oteryn:item.tibia.i37109";
    let revision = definitions
        .revision(sliver_key)
        .ok_or("Forge sliver definition missing")?;
    let sliver = definitions
        .definition_ref("Item", sliver_key, revision)
        .ok_or("Forge sliver reference missing")?;
    if super::full_e2ef7::units(&client, sliver) != 0 {
        return Err("funded Forge pre-state already contains slivers".into());
    }
    let mut applied = Vec::new();
    for (ordinal, operation, expected_balance) in [
        (10, Operation::DustToSlivers, (40, 100)),
        (11, Operation::DustLimit, (15, 120)),
    ] {
        client.service_liveness(Duration::from_millis(490)).await?;
        let before = client
            .inventory()
            .cloned()
            .ok_or("funded inventory missing")?;
        let (frame, body) = forge(
            &mut client,
            operation,
            occurrence(session, ordinal),
            None,
            0,
        )
        .await?;
        check_funded(&body, operation)?;
        if dust(&mut client).await? != expected_balance
            || super::full_e2ef7::units(&client, sliver) != 3
        {
            return Err(format!(
                "funded {operation:?} did not change balance, cap and slivers to expected values"
            )
            .into());
        }
        let after = client
            .inventory()
            .cloned()
            .ok_or("funded inventory missing")?;
        check_funded_inventory(&before, after, operation, sliver)?;
        applied.push(body);
        frames.push(frame);
    }
    let balance = (15, 120);
    let inventory = client
        .inventory()
        .cloned()
        .ok_or("funded inventory missing")?;
    for (ordinal, (name, operation, expected_reason)) in [
        // The valid, distinct pair is evaluated against the remaining dust.
        ("fusion", Operation::Fusion, "INSUFFICIENT_DUST"),
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
    let (populated_history, final_history) =
        forge(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    let expected_history = serde_json::json!([
        {"ordinal":2,"receipt":applied[1]},
        {"ordinal":1,"receipt":applied[0]}
    ]);
    if final_history != expected_history {
        return Err("Forge history does not contain exactly the funded outcomes".into());
    }
    frames.push(populated_history);
    let (captured_balance, current_dust) = dust_capture(&mut client).await?;
    if captured_balance != balance {
        return Err("Forge current dust changed before logout".into());
    }
    frames.push(current_dust);
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("Forge capture logout refused".into());
    }
    drop(client);
    let mut fresh = connect_character(clients, 0x45, 0).await?;
    if fresh.join_snapshot().game_session_id == session
        || fresh
            .inventory()
            .cloned()
            .map(super::full_row_p18::durable_inventory)
            != Some(super::full_row_p18::durable_inventory(inventory))
        || dust(&mut fresh).await? != balance
    {
        return Err("Forge funded state changed after relog".into());
    }
    fresh.service_liveness(Duration::from_millis(490)).await?;
    let (_, persisted_history) =
        forge(&mut fresh, Operation::QueryHistory, [0; 16], None, 50).await?;
    if persisted_history != expected_history {
        return Err("Forge funded history changed after relog".into());
    }
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("Forge relog session logout refused".into());
    }
    write_capture(root, *session.as_bytes(), &frames)?;
    Ok(format!(
        "live-capture session={}",
        uuid_text(session.as_bytes())
    ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::item_view::{CharacterInventory, ItemEntry};
    use std::num::NonZeroU32;

    #[test]
    fn funded_inventory_rejects_collateral_additions_and_deletions() -> TestResult {
        let dagger = ItemEntry {
            appearance_id: 0,
            handle: NonZeroU64::MIN,
            item_definition_ref: NonZeroU32::MIN,
            count: NonZeroU32::MIN,
            sub_type: 0,
        };
        let sliver = ItemEntry {
            handle: NonZeroU64::new(2).unwrap(),
            item_definition_ref: NonZeroU32::new(2).unwrap(),
            count: NonZeroU32::new(3).unwrap(),
            ..dagger
        };
        let unrelated = ItemEntry {
            handle: NonZeroU64::new(3).unwrap(),
            ..dagger
        };
        let before = CharacterInventory {
            entries: vec![dagger],
            ..Default::default()
        };
        for operation in [Operation::DustToSlivers, Operation::DustLimit] {
            let mut expected = before.clone();
            if operation == Operation::DustToSlivers {
                expected.entries.push(sliver);
            }
            let definition = sliver.item_definition_ref;
            check_funded_inventory(&before, expected.clone(), operation, definition)?;
            let mut added = expected.clone();
            added.entries.push(unrelated);
            assert!(check_funded_inventory(&before, added, operation, definition).is_err());
            expected.entries.remove(0);
            assert!(check_funded_inventory(&before, expected, operation, definition).is_err());
        }
        Ok(())
    }

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
