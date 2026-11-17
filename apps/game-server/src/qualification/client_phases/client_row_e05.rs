//! E05 client replay capture: browse the real Market wares and history, create a buy offer, read
//! My Offers, cancel it and read My Offers and History again. Client owner: the Market panel in
//! `crates/client-ui/src/features/0063-market`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::market::{self as wire, MarketReply, MarketRequest};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("E05_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn command(
    client: &mut DevClientSession,
    request: MarketRequest,
) -> TestResult<(MarketReply, Vec<u8>)> {
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
        return Err(format!("market transport status {:?}", response.status).into());
    }
    let reply = wire::decode_reply(&response.payload).map_err(|error| format!("decode {error:?}"))?;
    Ok((reply, response.payload))
}

fn request(action: u8, definition: u32, offer: [u8; 16]) -> MarketRequest {
    MarketRequest {
        action,
        definition,
        offer,
        amount: u32::from(action == 5),
        price: u64::from(action == 5),
        anonymous: false,
        page: 0,
    }
}

pub(super) fn cancelled_offer(
    own: &MarketReply, history: &MarketReply, offer: [u8; 16], durable_state: i16,
) -> bool {
    own.action == 3 && own.code == 1 && history.action == 4 && history.code == 1
        && durable_state == 3
        && !own.rows.iter().any(|row| row.offer == offer)
        && history.rows.iter().any(|row| row.offer == offer && row.state == 2)
}

pub(super) async fn durable_offer_end(clients: &SeamClients<'_>, offer: [u8; 16]) -> TestResult<i16> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    // The legacy market reply projects every ended reason as state 2.
    Ok(sqlx::query_scalar("SELECT ended_as FROM game_market_offers WHERE offer_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid")
        .bind(offer.as_slice())
        .bind(clients.world.as_slice())
        .bind(clients.characters[1].as_slice())
        .fetch_one(&mut database).await?)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0x36, 1).await?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        near_locker(world, at)
    })
    .ok_or("no reachable authored depot locker")?;
    walk(&mut client, world, &path).await?;

    let (wares, wares_bytes) = command(&mut client, request(1, 0, [0; 16])).await?;
    let ware = wares.rows.first().cloned().ok_or("wares advertised nothing")?;
    if wares.code != 1 || wares.action != 1 || ware.definition == 0 {
        return Err(format!("wares reply {wares:?}").into());
    }
    let (created, created_bytes) =
        command(&mut client, request(5, ware.definition, [0; 16])).await?;
    if created.code != 1 || created.offer == [0; 16] {
        return Err(format!(
            "create refused code={}; needs premium-current and bank gold",
            created.code
        )
        .into());
    }
    let offer = created.offer;
    let observed = bounded_observation("E05 My Offers", command(&mut client, request(3, 0, [0; 16]))).await;
    let cancellation = cancel_offer(clients, world, client, offer).await;
    let (own, own_bytes) = observed?;
    let (mut client, (cancelled, cancelled_bytes)) = cancellation?;
    let mine = own.rows.iter().find(|row| row.offer == offer)
        .ok_or("created offer absent from My Offers")?;
    if own.code != 1 || own.action != 3 || (mine.definition, mine.amount, mine.price) != (ware.definition, 1, 1) {
        return Err(format!("own offers {own:?}").into());
    }
    if cancelled.code != 1 || cancelled.action != 7 {
        return Err(format!("cancel code {}", cancelled.code).into());
    }
    let (after, after_bytes) = command(&mut client, request(3, 0, [0; 16])).await?;
    let (history, history_bytes) = command(&mut client, request(4, 0, [0; 16])).await?;
    let ended = history
        .rows
        .iter()
        .find(|row| row.offer == offer)
        .cloned()
        .ok_or("cancelled offer absent from History")?;
    let durable = durable_offer_end(clients, offer).await?;
    if !cancelled_offer(&after, &history, offer, durable) {
        return Err(format!("E05 cancellation mismatch own={after:?} history={history:?} durable={durable}").into());
    }
    let expected = format!(
        "definition={}\noffer={}\nside={}\namount=1\nprice=1\nown_after_cancel={}\nhistory_state={}\n",
        ware.definition,
        hex(&offer),
        mine.side,
        after.rows.len(),
        ended.state
    );
    capture("e05-wares.bin", &wares_bytes);
    capture("e05-create.bin", &created_bytes);
    capture("e05-own.bin", &own_bytes);
    capture("e05-cancel.bin", &cancelled_bytes);
    capture("e05-own-after.bin", &after_bytes);
    capture("e05-history.bin", &history_bytes);
    capture("e05-expected.txt", expected.as_bytes());
    client_row_c08::require_logout(&mut client, Duration::from_secs(15)).await?;
    Ok(format!(
        "definition={} history_state={}",
        ware.definition, ended.state
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=E05.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E05.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=E05.client status=FAIL evidence=timeout"),
    }
}

pub(super) async fn bounded_observation<T>(phase: &str, observation: impl Future<Output = TestResult<T>>) -> TestResult<T> {
    tokio::time::timeout(Duration::from_secs(8), observation).await
        .map_err(|error| verdict::context(format!("{phase} deadline"), error))?
}

pub(super) async fn cancel_offer(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    client: DevClientSession,
    offer: [u8; 16],
) -> TestResult<(DevClientSession, (MarketReply, Vec<u8>))> {
    async fn cancel(mut client: DevClientSession, offer: [u8; 16]) -> TestResult<(DevClientSession, (MarketReply, Vec<u8>))> {
        let (reply, bytes) = bounded_observation("Market cancellation", command(&mut client, request(7, 0, offer))).await?;
        if reply.code != 1 || reply.action != 7 {
            return Err(format!("Market cancellation refused: {reply:?}").into());
        }
        Ok((client, (reply, bytes)))
    }

    let first = cancel(client, offer).await;
    recover_market_cancel(first, async {
        let mut fresh = connect_character(clients, 0x39, 1).await?;
        let path = route(world, tile(fresh.world_spatial().actor_position)?, |at| near_locker(world, at))
            .ok_or("Market cleanup: no reachable depot locker after re-admission")?;
        walk(&mut fresh, world, &path).await?;
        cancel(fresh, offer).await
    }).await
}

async fn recover_market_cancel<T>(first: TestResult<T>, retry: impl Future<Output = TestResult<T>>) -> TestResult<T> {
    match first {
        Ok(value) => Ok(value),
        Err(initial) => match retry.await {
            Ok(value) => Ok(value),
            Err(retry_error) => {
                evidence(&format!("Market cancellation retry failed: {retry_error}"));
                Err(initial)
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_cancellation_requires_successful_queries_and_cancelled_status() {
        let offer = [1; 16];
        let mut own = MarketReply { action: 3, code: 9, offer: [0; 16], rows: Vec::new() };
        let mut history = MarketReply { action: 4, code: 1, offer: [0; 16], rows: vec![wire::MarketRow {
            offer, definition: 1, side: 1, amount: 1, price: 1, state: 2, anonymous: false,
        }] };
        assert!(!cancelled_offer(&own, &history, offer, 3));
        own.code = 1;
        assert!(!cancelled_offer(&own, &history, offer, 4));
        history.rows[0].state = 5;
        assert!(!cancelled_offer(&own, &history, offer, 3));
        history.rows[0].state = 2;
        assert!(cancelled_offer(&own, &history, offer, 3));
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;

    #[tokio::test]
    async fn cancellation_command_failure_retries_after_fresh_admission() -> TestResult {
        let failed = Err("session closed during cancellation".into());
        let recovered = recover_market_cancel(failed, async { Ok(7) }).await?;
        assert_eq!(recovered, 7);
        Ok(())
    }
}
