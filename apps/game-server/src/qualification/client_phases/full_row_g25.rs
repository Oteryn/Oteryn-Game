//! G25: coin transfer quotes and coin market (cmd 6127/6128, caps 154/63).
//! Admits the character, asks for the durable operation detail (action 3), a 25-coin gift review
//! addressed to the other provisioned character (action 1) and the coin market listing (market
//! action 1), then re-reads the detail. A pinned Platform with a coin API must answer the review
//! `ok` with a quote; the node boot pins no Store API, so the server's exact `unavailable` refusal
//! is asserted with unchanged state and the row ends FAIL `needed:platform-coin-api-in-node-boot`.
use super::*;
use oteryn_dev_client::DevClientSession;
use sqlx::Connection;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(120), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G25.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn logout(client: &mut RowClient) -> TestResult {
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

async fn character_name(clients: &SeamClients<'_>, index: usize) -> TestResult<String> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let name: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id = encode($1, 'hex')::uuid",
    )
    .bind(clients.characters[index].as_slice())
    .fetch_one(&mut database)
    .await?;
    database.close().await?;
    Ok(name)
}

use oteryn_protocol_oteryn::{coin_market, coin_transfer};

async fn json_call(
    client: &mut DevClientSession,
    capability: u32,
    kind: u32,
    payload: &[u8],
) -> TestResult<serde_json::Value> {
    let response = client
        .session_mut()
        .gated_command(capability, kind, payload)
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("step=call-{kind} observed=status:{:?}", response.status).into());
    }
    let text = coin_market::decode_reply(&response.payload)
        .map_err(|error| format!("step=decode-{kind} observed={error:?}"))?;
    Ok(serde_json::from_str(&text)?)
}

fn result_of(value: &serde_json::Value) -> String {
    value["result"].as_str().unwrap_or("none").to_owned()
}

fn operation() -> [u8; 16] {
    let mut id = [0_u8; 16];
    id[0] = 1;
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = 1;
    id
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let peer = character_name(clients, 0).await?;
    let mut client = row_connect_world(clients, 0xd5).await?;
    for capability in [coin_transfer::CAPABILITY, coin_market::CAPABILITY] {
        if !client.selected_capabilities().contains(&capability) {
            return Err(format!("step=admit observed=capability-{capability}-not-selected").into());
        }
    }
    let detail = |op| {
        coin_transfer::encode(&coin_transfer::Request {
            action: 3,
            operation: op,
            ..Default::default()
        })
    };
    let detail_before = json_call(
        &mut client,
        coin_transfer::CAPABILITY,
        coin_transfer::COMMAND,
        &detail(operation()).map_err(|error| format!("step=encode observed={error:?}"))?,
    )
    .await?;
    let review = coin_transfer::encode(&coin_transfer::Request {
        action: 1,
        recipient: peer.clone(),
        amount: 25,
        ..Default::default()
    })
    .map_err(|error| format!("step=encode observed={error:?}"))?;
    let reviewed = json_call(
        &mut client,
        coin_transfer::CAPABILITY,
        coin_transfer::COMMAND,
        &review,
    )
    .await?;
    let list = coin_market::encode(&coin_market::Request::default())
        .map_err(|error| format!("step=encode observed={error:?}"))?;
    let market = json_call(
        &mut client,
        coin_market::CAPABILITY,
        coin_market::COMMAND,
        &list,
    )
    .await?;
    let detail_after = json_call(
        &mut client,
        coin_transfer::CAPABILITY,
        coin_transfer::COMMAND,
        &detail(operation()).map_err(|error| format!("step=encode observed={error:?}"))?,
    )
    .await?;
    let unchanged = detail_before == detail_after;
    logout(&mut client).await?;
    let (transfer, listed) = (result_of(&reviewed), result_of(&market));
    if transfer == "unavailable" || listed == "unavailable" {
        return Err(format!("step=quote observed=transfer:{transfer};market:{listed};state_unchanged={unchanged};needed:platform-coin-api-in-node-boot").into());
    }
    if listed == "depot_required" {
        return Err(format!("step=market observed=market:{listed};transfer:{transfer};needed:depot-access-for-coin-market").into());
    }
    if listed != "ok" {
        return Err(format!(
            "step=market observed=market:{listed};transfer:{transfer};needed=ok-listing"
        )
        .into());
    }
    if transfer != "ok" || reviewed["quote"].is_null() || !unchanged {
        return Err(format!("step=quote observed=transfer:{transfer};market:{listed};quote_present={};state_unchanged={unchanged};needed=ok-with-quote", !reviewed["quote"].is_null()).into());
    }
    Ok(format!(
        "transfer-review-ok-with-quote;market:{listed};state_unchanged"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_tokens_cover_empty_and_whitespace() {
        assert_eq!(token(""), "");
        assert!(!token("a b\tc\nd").chars().any(char::is_whitespace));
        assert_eq!(token("source:path.rs:12"), "source:path.rs:12");
    }
}
