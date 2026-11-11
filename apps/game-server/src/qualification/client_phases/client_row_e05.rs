//! E05 client replay capture: browse the real Market wares and history, create a buy offer, read
//! My Offers, cancel it and read My Offers and History again. Client owner: the Market panel in
//! `crates/client-ui/src/features/0063-market`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::market::{self as wire, MarketReply, MarketRequest};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
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
    let reply =
        wire::decode_reply(&response.payload).map_err(|error| format!("decode {error:?}"))?;
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0x36, 1).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
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
    let (own, own_bytes) = command(&mut client, request(3, 0, [0; 16])).await?;
    let mine = own
        .rows
        .iter()
        .find(|row| row.offer == offer)
        .cloned()
        .ok_or("created offer absent from My Offers")?;
    if own.code != 1 || (mine.definition, mine.amount, mine.price) != (ware.definition, 1, 1) {
        return Err(format!("own offers {own:?}").into());
    }
    let (cancelled, cancelled_bytes) = command(&mut client, request(7, 0, offer)).await?;
    if cancelled.code != 1 {
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
    if after.rows.iter().any(|row| row.offer == offer) {
        return Err("cancelled offer still in My Offers".into());
    }
    let expected = format!(
        "definition={}\noffer={}\nside={}\namount=1\nprice=1\nown_after_cancel={}\nhistory_state={}\n",
        ware.definition,
        hex(&offer),
        mine.side,
        after.rows.len(),
        ended.state
    );
    capture("e05-wares.bin", &wares_bytes)?;
    capture("e05-create.bin", &created_bytes)?;
    capture("e05-own.bin", &own_bytes)?;
    capture("e05-cancel.bin", &cancelled_bytes)?;
    capture("e05-own-after.bin", &after_bytes)?;
    capture("e05-history.bin", &history_bytes)?;
    capture("e05-expected.txt", expected.as_bytes())?;

    Ok(format!(
        "definition={} history_state={}",
        ware.definition, ended.state
    ))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_preserves_zero_padding_and_high_bytes() {
        assert_eq!(hex(&[]), "");
        assert_eq!(hex(&[0, 15, 128, 255]), "000f80ff");
    }
}
