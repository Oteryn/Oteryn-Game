//! E06 client replay capture: Market wares, a book query, the refusal for a non-marketable
//! definition, a created and cancelled buy offer (refund path), then My Offers and History again
//! under a fresh admission. Scheduled expiry stays a server gap (settlement.rs). Client owner: the Market panel in
//! `crates/client-ui/src/features/0063-market`.
use super::*;
use super::{
    row_connect_character as connect_character,
    row_finish as row_logout,
};
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

async fn at_market(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        near_locker(world, at)
    })
    .ok_or("no reachable authored depot locker")?;
    walk(client, world, &path).await
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    row_complete(async {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0x37, 1).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        at_market(&mut client, world).await?;
        let (wares, wares_bytes) = command(&mut client, request(1, 0, [0; 16])).await?;
        let ware = wares
            .rows
            .first()
            .cloned()
            .ok_or("wares advertised nothing")?;
        if wares.code != 1 || wares.action != 1 || ware.definition == 0 {
            return Err(format!("wares reply {wares:?}").into());
        }
        let (book, book_bytes) = command(&mut client, request(2, ware.definition, [0; 16])).await?;
        if book.code != 1
            || book.action != 2
            || book.rows.iter().any(|r| r.definition != ware.definition)
        {
            return Err(format!("book reply {book:?}").into());
        }
        let advertised: Vec<u32> = wares.rows.iter().map(|r| r.definition).collect();
        let invalid = (1..=u32::MAX)
            .rev()
            .find(|definition| !advertised.contains(definition))
            .ok_or("no unadvertised definition")?;
        let (refusal, refusal_bytes) = command(&mut client, request(2, invalid, [0; 16])).await?;
        if refusal.code != 14 {
            return Err(format!("expected refusal 14, observed {refusal:?}").into());
        }
        let (created, _) = command(&mut client, request(5, ware.definition, [0; 16])).await?;
        if created.code != 1 || created.offer == [0; 16] {
            return Err(format!(
                "create refused code={}; needs premium-current and bank gold",
                created.code
            )
            .into());
        }
        let offer = created.offer;
        let (cancelled, _) = command(&mut client, request(7, 0, offer)).await?;
        if cancelled.code != 1 {
            return Err(format!("cancel code {}", cancelled.code).into());
        }
        if client.row_logout().await?.result
            != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        {
            return Err("logout refused".into());
        }
        row_needs_logout_0 = false;
        let mut client = connect_character(clients, 0x38, 1).await?;
        let mut row_needs_logout_1 = true;
        let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
            at_market(&mut client, world).await?;
            let (own, own_bytes) = command(&mut client, request(3, 0, [0; 16])).await?;
            let (history, history_bytes) = command(&mut client, request(4, 0, [0; 16])).await?;
            let ended = history
                .rows
                .iter()
                .find(|row| row.offer == offer)
                .cloned()
                .ok_or("cancelled offer absent from fresh-admission History")?;
            if own.rows.iter().any(|row| row.offer == offer) {
                return Err("cancelled offer active after fresh admission".into());
            }
            let expected = format!(
                "definition={}\noffer={}\nrefusal=14\nfresh_own={}\nhistory_state={}\n",
                ware.definition,
                hex(&offer),
                own.rows.len(),
                ended.state
            );
            capture("e06-wares.bin", &wares_bytes)?;
            capture("e06-book.bin", &book_bytes)?;
            capture("e06-refused.bin", &refusal_bytes)?;
            capture("e06-own.bin", &own_bytes)?;
            capture("e06-history.bin", &history_bytes)?;
            capture("e06-expected.txt", expected.as_bytes())?;
            tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
            row_needs_logout_1 = false;
            Ok(format!(
                "definition={} refusal=14 history_state={} expiry=MISSING-SERVER",
                ware.definition, ended.state
            ))
        })
        .await
        .unwrap_or_else(|error| Err(error.into()));
        if row_needs_logout_1 {
            tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
        }
        row_result
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    }
    row_result

    }).await?
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=E06.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E06.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=E06.client status=FAIL evidence=timeout"),
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
