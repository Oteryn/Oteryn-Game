//! E06 — Market expiry/refunds, scheduling, and all marketable content.
//!
//! Player scenario derived from the E06 audit/matrix row and the market settlement owner:
//! 1. Walk the provisioned character to an authored depot locker, send MARKET wares pages, and
//!    require accepted replies whose rows are bounded and contain distinct definition references.
//! 2. Send a MARKET book query for every definition advertised by those pages and require the
//!    accepted, matching book reply; this exercises all marketable content served by this world.
//! 3. Send a book query for an unadvertised definition and require refusal code 14, proving the
//!    non-marketable-content path rather than treating an empty book as acceptance.
//! 4. Send a one-unit buy-offer command for an advertised definition and require its exact offer
//!    id in My Offers; a refusal reports the observed code and the missing player prerequisite.
//! 5. Send cancel for that offer, require it to disappear from My Offers and appear in History;
//!    this is the player-observable buy-escrow refund path owned by market settlement.
//! 6. Log out, obtain a fresh admission, walk back to the locker, and re-check My Offers and
//!    History so the ended offer and refund outcome are shown to be durable.
//! 7. Having driven the real settlement path, require independent scheduled expiry. The current
//!    server exposes only command/query-driven advancement, so this step reports MISSING-SERVER
//!    at the production owner rather than fabricating a 30-day clock advance or a PASS.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::market::{self as wire, MarketReply, MarketRequest};
use std::collections::BTreeSet;

const GAP: &str = "source:crates/gameplay/durability/src/durability/market/settlement.rs:240";

async fn command(client: &mut DevClientSession, request: MarketRequest) -> TestResult<MarketReply> {
    let payload =
        wire::encode_request(request).map_err(|error| format!("market input: {error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_MARKET_V1,
            wire::COMMAND_TYPE_MARKET,
            &payload,
        )
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("market command transport status={:?}", response.status).into());
    }
    wire::decode_reply(&response.payload)
        .map_err(|error| format!("market reply decode: {error:?}").into())
}

fn request(action: u8, definition: u32, offer: [u8; 16], page: u32) -> MarketRequest {
    MarketRequest {
        action,
        definition,
        offer,
        amount: u32::from(action == 5),
        price: u64::from(action == 5),
        anonymous: false,
        page,
    }
}

async fn at_market(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        near_locker(world, at)
    })
    .ok_or("E06 step 1: no reachable authored depot locker")?;
    walk(client, world, &path).await
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<&'static str> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = row_connect_character(clients, 0xe6, 1).await?;
    at_market(&mut client, world).await?;

    let mut definitions = Vec::new();
    let mut seen = BTreeSet::new();
    for page in 0..=100_000 {
        let reply = command(&mut client, request(1, 0, [0; 16], page)).await?;
        if reply.code != 1 || reply.action != 1 || reply.rows.len() > 32 {
            return Err(format!("E06 step 1: wares page={page} observed={reply:?}").into());
        }
        let count = reply.rows.len();
        for row in reply.rows {
            if row.definition == 0 || !seen.insert(row.definition) {
                return Err(format!("E06 step 1: duplicate/zero ware observed={row:?}").into());
            }
            definitions.push(row.definition);
        }
        if count < 32 {
            break;
        }
    }
    if definitions.is_empty() {
        return Err("E06 step 1: accepted wares query advertised no marketable content".into());
    }
    evidence(&format!(
        "E06_CHECK step=1 status=PASS definitions={}",
        definitions.len()
    ));

    for &definition in &definitions {
        let reply = command(&mut client, request(2, definition, [0; 16], 0)).await?;
        if reply.code != 1
            || reply.action != 2
            || reply.rows.iter().any(|r| r.definition != definition)
        {
            return Err(format!("E06 step 2: definition={definition} observed={reply:?}").into());
        }
    }
    evidence("E06_CHECK step=2 status=PASS all-advertised-books-accepted");

    let invalid = (1..=u32::MAX)
        .rev()
        .find(|definition| !seen.contains(definition))
        .ok_or("E06 step 3: no unadvertised definition reference")?;
    let refusal = command(&mut client, request(2, invalid, [0; 16], 0)).await?;
    if refusal.code != 14 {
        return Err(format!("E06 step 3: expected code=14 observed={refusal:?}").into());
    }
    evidence("E06_CHECK step=3 status=PASS observed-code=14");

    let created = command(&mut client, request(5, definitions[0], [0; 16], 0)).await?;
    if created.code != 1 {
        return Err(format!("E06 step 4: create observed-code={}; need premium-current and at least fee+escrow bank gold", created.code).into());
    }
    let offer = created.offer;
    let own = command(&mut client, request(3, 0, [0; 16], 0)).await?;
    if own.code != 1
        || !own
            .rows
            .iter()
            .any(|row| row.offer == offer && row.amount == 1 && row.price == 1)
    {
        return Err(format!("E06 step 4: created offer missing observed={own:?}").into());
    }
    evidence("E06_CHECK step=4 status=PASS offer-visible");

    let cancelled = command(&mut client, request(7, 0, offer, 0)).await?;
    if cancelled.code != 1 {
        return Err(format!("E06 step 5: cancel observed={cancelled:?}").into());
    }
    let own = command(&mut client, request(3, 0, [0; 16], 0)).await?;
    let history = command(&mut client, request(4, 0, [0; 16], 0)).await?;
    if own.rows.iter().any(|row| row.offer == offer)
        || !history
            .rows
            .iter()
            .any(|row| row.offer == offer && row.state == 2)
    {
        return Err(format!(
            "E06 step 5: refund/history delta missing own={own:?} history={history:?}"
        )
        .into());
    }
    evidence("E06_CHECK step=5 status=PASS cancelled-refund-history");

    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("E06 step 6: logout refused".into());
    }
    let mut client = row_connect_character(clients, 0xe7, 1).await?;
    at_market(&mut client, world).await?;
    let own = command(&mut client, request(3, 0, [0; 16], 0)).await?;
    let history = command(&mut client, request(4, 0, [0; 16], 0)).await?;
    if own.rows.iter().any(|row| row.offer == offer)
        || !history
            .rows
            .iter()
            .any(|row| row.offer == offer && row.state == 2)
    {
        return Err(format!(
            "E06 step 6: fresh-admission state mismatch own={own:?} history={history:?}"
        )
        .into());
    }
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("E06 step 6: final logout refused".into());
    }
    evidence("E06_CHECK step=6 status=PASS durable-ended-offer");
    Ok(GAP)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(240), journey(clients)).await;
    match result {
        Ok(Ok(source)) => evidence(&format!(
            "e2e_all scenario=E06.full status=MISSING-SERVER seconds={:.3} evidence={source}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E06.full status=FAIL seconds={:.3} evidence={}",
            started.elapsed().as_secs_f64(),
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence(&format!(
            "e2e_all scenario=E06.full status=FAIL seconds={:.3} evidence=step-deadline-timeout",
            started.elapsed().as_secs_f64()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_shapes_match_market_actions() {
        assert!(wire::encode_request(request(1, 0, [0; 16], 4)).is_ok());
        assert!(wire::encode_request(request(2, 7, [0; 16], 0)).is_ok());
        assert!(wire::encode_request(request(5, 7, [0; 16], 0)).is_ok());
        let mut offer = [0; 16];
        offer[6] = 0x70;
        offer[8] = 0x80;
        assert!(wire::encode_request(request(7, 0, offer, 0)).is_ok());
    }
}
