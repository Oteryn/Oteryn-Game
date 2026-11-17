//! G25: coin transfer quotes and coin market (cmd 6127/6128, caps 154/63).
//! Review a 25-coin gift to the other fixture character, query the coin market, and require
//! both Platform-backed wallet projections to be unchanged. The quote must bind the requested
//! recipient, amount, and world in the Game-owned quote table.
use super::*;
use oteryn_dev_client::DevClientSession;
use sqlx::Connection;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await;
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

fn review_matches(value: &serde_json::Value, recipient: &str, amount: u64) -> bool {
    result_of(value) == "ok"
        && value["quote"].as_str().is_some_and(|id| !id.is_empty())
        && value["recipient"].as_str() == Some(recipient)
        && value["amount"].as_str().and_then(|amount| amount.parse::<u64>().ok()) == Some(amount)
        && value["expires_in_seconds"].as_u64() == Some(60)
}

async fn coin_balance(client: &mut DevClientSession) -> TestResult<[u64; 3]> {
    let request = coin_market::Request { action: 9, ..Default::default() };
    let payload = coin_market::encode(&request).map_err(|error| format!("step=balance encode={error:?}"))?;
    let reply = json_call(client, coin_market::CAPABILITY, coin_market::COMMAND, &payload).await?;
    if result_of(&reply) != "ok" {
        return Err(format!("step=balance observed={};needed=platform-coin-api-and-depot-access", result_of(&reply)).into());
    }
    let field = |name: &str| -> TestResult<u64> {
        reply[name].as_str().and_then(|value| value.parse().ok())
            .ok_or_else(|| format!("step=balance observed=invalid-{name}").into())
    };
    Ok([field("transferable")?, field("non_transferable")?, field("reserved")?])
}

async fn at_depot(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| near_locker(world, at)).ok_or("step=depot needed=reachable-locker")?;
    walk(client, world, &path).await
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let peer = character_name(clients, 0).await?;
    let mut client = connect_world(clients, 0xd5).await?;
    for capability in [coin_transfer::CAPABILITY, coin_market::CAPABILITY] {
        if !client.selected_capabilities().contains(&capability) {
            return Err(format!("step=admit observed=capability-{capability}-not-selected").into());
        }
    }
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let mut recipient = connect_character(clients, 0xd5, 0).await?;
    at_depot(&mut client, world).await?;
    at_depot(&mut recipient, world).await?;
    let before = [coin_balance(&mut client).await?, coin_balance(&mut recipient).await?];
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
    let after = [coin_balance(&mut client).await?, coin_balance(&mut recipient).await?];
    let unchanged = before == after;
    logout(&mut client).await?;
    logout(&mut recipient).await?;
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
    if !review_matches(&reviewed, &peer, 25) || !unchanged {
        return Err(format!("step=quote observed=transfer:{transfer};market:{listed};quote_present={};state_unchanged={unchanged};needed=ok-with-quote", !reviewed["quote"].is_null()).into());
    }
    let quote = reviewed["quote"].as_str().ok_or("step=quote observed=missing-quote-id")?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let bound: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_coin_gift_quotes
         WHERE quote_id::text=$1 AND actor=encode($2,'hex')::uuid AND recipient=encode($3,'hex')::uuid
           AND world_id=encode($4,'hex')::uuid AND amount=25 AND consumed_by IS NULL)",
    ).bind(quote).bind(clients.characters[1].as_slice()).bind(clients.characters[0].as_slice())
        .bind(clients.world.as_slice()).fetch_one(&mut database).await?;
    if !bound {
        return Err("step=quote observed=no-unconsumed-quote-for-fixture-actors".into());
    }
    Ok(format!("transfer-review-ok-with-bound-quote;market:{listed};balances-unchanged"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coin_quote_requires_the_requested_recipient_and_amount() {
        let mut quote = serde_json::json!({
            "result": "ok", "quote": "01935f3b-2c4d-7a11-8b22-334455667788", "recipient": "Other",
            "amount": "25", "expires_in_seconds": 60
        });
        assert!(!review_matches(&quote, "Peer", 25));
        quote["recipient"] = "Peer".into();
        assert!(!review_matches(&quote, "Peer", 24));
        assert!(review_matches(&quote, "Peer", 25));
    }
}
