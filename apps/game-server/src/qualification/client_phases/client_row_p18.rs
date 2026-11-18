//! P18 client replay capture: the real Forge JSON replies for HISTORY, FUSION, DUST_TO_SLIVERS and
//! DUST_LIMIT. The consumer is `apply_result` and `panel_text` in
//! `crates/client-ui/src/features/0152-item-forge`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_gameplay_domain::domain::forge_dust::{DustLimit, ForgeDustError};
use oteryn_protocol_oteryn::item_forge::{
    CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_ITEM_FORGE, Operation, Request,
};
use std::num::NonZeroU64;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "P18_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
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
    let backpack = inventory
        .main_backpack
        .ok_or("no carried backpack for the wrong-items probe")?;
    let second = inventory
        .entries
        .iter()
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
        .find(|entry| entry.handle != backpack.handle)
        .ok_or("no distinct carried item for the wrong-items probe")?;
    Ok((backpack.handle, second.handle))
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

fn forge_outcome(operation: &str, succeeded: bool, applied: bool, reason: &str) -> bool {
    matches!(
        (operation, succeeded, applied, reason),
        ("fusion", false, false, "WRONG_ITEMS")
            | ("dust_to_slivers", false, false, "INSUFFICIENT_DUST")
            | ("dust_limit", false, false, "INSUFFICIENT_DUST")
    )
}

#[test]
fn forge_requires_the_named_business_outcome() {
    assert!(forge_outcome(
        "dust_limit",
        false,
        false,
        "INSUFFICIENT_DUST"
    ));
    assert!(!forge_outcome("dust_limit", true, true, ""));
    assert!(!forge_outcome("fusion", true, true, ""));
    assert!(!forge_outcome(
        "dust_to_slivers",
        false,
        false,
        "WRONG_ITEMS"
    ));
    assert!(forge_outcome(
        "dust_to_slivers",
        false,
        false,
        "INSUFFICIENT_DUST"
    ));
}

async fn dust(
    connection: &mut sqlx::PgConnection,
    clients: &SeamClients<'_>,
) -> TestResult<(u32, u16)> {
    let row: Option<(i64, i16)> = sqlx::query_as("SELECT balance, dust_limit FROM game_character_forge_dust WHERE character_id=encode($1,'hex')::uuid")
        .bind(clients.characters[1].as_slice()).fetch_optional(connection).await?;
    let (balance, limit) = row.unwrap_or((0, 100));
    Ok((u32::try_from(balance)?, u16::try_from(limit)?))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x44).await?;
    let session = client.join_snapshot().game_session_id;
    let pair = carried_pair(&client)?;
    use sqlx::Connection as _;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let mut expected = String::new();
    let (history_bytes, history) =
        forge(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    let rows = history
        .as_array()
        .ok_or("HISTORY payload was not an array")?;
    capture("p18-history.bin", &history_bytes);
    expected.push_str(&format!("history_rows={}\n", rows.len()));
    for (ordinal, (name, operation)) in [
        ("fusion", Operation::Fusion),
        ("dust_to_slivers", Operation::DustToSlivers),
        ("dust_limit", Operation::DustLimit),
    ]
    .into_iter()
    .enumerate()
    {
        let before = dust(&mut connection, clients).await?;
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
        let after = dust(&mut connection, clients).await?;
        let matches_expected = if operation == Operation::DustLimit {
            let limit =
                DustLimit::new(before.1).map_err(|error| format!("dust limit: {error:?}"))?;
            match limit.next_upgrade() {
                Ok((cost, next)) if before.0 >= cost => {
                    succeeded
                        && applied
                        && reason.is_empty()
                        && after == (before.0 - cost, next.get())
                }
                Ok(_) => forge_outcome(name, succeeded, applied, reason) && after == before,
                Err(ForgeDustError::MaximumLimitReached) => {
                    !succeeded && !applied && reason == "MAXIMUM_LIMIT" && after == before
                }
                Err(error) => return Err(format!("dust limit plan: {error:?}").into()),
            }
        } else {
            forge_outcome(name, succeeded, applied, reason) && after == before
        };
        if body["operation"].as_str() != Some(name) || !matches_expected {
            return Err(format!(
                "{name} unexpected forge outcome dust={before:?}->{after:?}: {body}"
            )
            .into());
        }
        capture(&format!("p18-{}.bin", name.replace('_', "-")), &bytes);
        expected.push_str(&format!(
            "{name}.succeeded={succeeded}\n{name}.applied={applied}\n{name}.reason={reason}\n"
        ));
    }
    capture("p18-expected.txt", expected.as_bytes());
    let _ = client.session_mut().logout().await?;
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P18.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P18.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P18.client status=FAIL evidence=timeout"),
    }
}
