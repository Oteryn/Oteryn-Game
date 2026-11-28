//! G25: review a 25-coin gift, then read the Platform wallet and the coin market
//! listing from an authored depot. The review must leave the wallet and operation unchanged.
use super::*;
use oteryn_dev_client::DevClientSession;
use sqlx::Connection;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await;
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

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let peer = character_name(clients, 0).await?;
    let world = full_e2ef7::qualification_world(clients)?;
    let mut client = connect_world(clients, 0xd5).await?;
    let outcome = verdict::observe(body(&mut client, world, &peer)).await;
    let closed = verdict::observe(logout(&mut client)).await;
    let detail = outcome?;
    closed?;
    Ok(detail)
}

async fn body(client: &mut DevClientSession, world: &BundleWorld, peer: &str) -> TestResult<String> {
    for capability in [
        coin_transfer::CAPABILITY,
        coin_market::CAPABILITY,
        oteryn_protocol_oteryn::depot::CAPABILITY_DEPOT_V1,
    ] {
        if !client.selected_capabilities().contains(&capability) {
            return Err(format!("step=admit observed=capability-{capability}-not-selected").into());
        }
    }
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| near_locker(world, at))
        .ok_or("step=market observed=no-reachable-authored-depot-locker")?;
    walk(client, world, &path).await?;

    let amount = 25;
    let detail = coin_transfer::encode(&coin_transfer::Request {
        action: 3,
        operation: crate::node::operator_files::uuid_v7()?,
        ..Default::default()
    })
    .map_err(|error| format!("step=encode-detail observed={error:?}"))?;
    let wallet = coin_market::encode(&coin_market::Request {
        action: 9,
        ..Default::default()
    })
    .map_err(|error| format!("step=encode-balance observed={error:?}"))?;
    let balance_before =
        json_call(client, coin_market::CAPABILITY, coin_market::COMMAND, &wallet).await?;
    if balance_before["result"] != "ok" {
        return Err(format!(
            "step=balance observed={} needed=platform-coin-api-at-depot",
            result_of(&balance_before)
        )
        .into());
    }
    let detail_before = json_call(
        client,
        coin_transfer::CAPABILITY,
        coin_transfer::COMMAND,
        &detail,
    )
    .await?;
    let review = coin_transfer::encode(&coin_transfer::Request {
        action: 1,
        recipient: peer.to_owned(),
        amount,
        ..Default::default()
    })
    .map_err(|error| format!("step=encode observed={error:?}"))?;
    let reviewed = json_call(
        client,
        coin_transfer::CAPABILITY,
        coin_transfer::COMMAND,
        &review,
    )
    .await?;
    let list = coin_market::encode(&coin_market::Request::default())
        .map_err(|error| format!("step=encode observed={error:?}"))?;
    let market = json_call(
        client,
        coin_market::CAPABILITY,
        coin_market::COMMAND,
        &list,
    )
    .await?;
    let detail_after = json_call(
        client,
        coin_transfer::CAPABILITY,
        coin_transfer::COMMAND,
        &detail,
    )
    .await?;
    let balance_after =
        json_call(client, coin_market::CAPABILITY, coin_market::COMMAND, &wallet).await?;
    let (transfer, listed) = (result_of(&reviewed), result_of(&market));
    if listed != "ok" || !market["offers"].is_array() {
        return Err(format!("step=market observed=market:{listed} needed=ok-listing").into());
    }
    let quote_present = reviewed["quote"]
        .as_str()
        .is_some_and(|quote| !quote.is_empty());
    let recipient_matches =
        reviewed["recipient"] == peer && reviewed["amount"] == amount.to_string();
    let unchanged = detail_before["result"] == "not_found"
        && detail_before == detail_after
        && balance_before == balance_after
        && reviewed["transferable"] == balance_before["transferable"]
        && reviewed["non_transferable"] == balance_before["non_transferable"];
    if transfer != "ok" || !quote_present || !recipient_matches || !unchanged {
        return Err(format!(
            "step=quote observed=transfer:{transfer};quote_present={quote_present};recipient_matches={recipient_matches};state_unchanged={unchanged};needed=accepted-funded-platform-quote"
        )
        .into());
    }
    Ok("transfer-review-ok-with-quote;market:ok;wallet-and-operation-unchanged".into())
}
