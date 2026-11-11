//! E10: Store catalogue, entitlement purchase, refusal, history, and relog durability.
//!
//! Concrete player scenario derived from the E10 audit/matrix and the Store dispatch owner:
//! 1. The admitted player requests the exercise-weapons catalogue and checks the accepted
//!    reply's revision, bounded offers, prices, currencies, and authoritative coin balances.
//! 2. The player buys an affordable catalogue offer using its exact quote and checks an
//!    accepted purchase result containing Platform order and operation identifiers.
//! 3. The player sends a second purchase with a deliberately stale price and checks the
//!    typed `STALE_QUOTE` refusal, with no order or operation identifiers.
//! 4. The player requests Game's purchase history and checks that the accepted purchase is
//!    present with the same order, offer, revision, price, and currency.
//! 5. After ordinary logout and a fresh admission, the player requests history again and
//!    checks that the same durable purchase remains present, then logs out normally.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::store as wire;

const CATEGORY: &str = "consumables.exercise-weapons";
const CURRENCY: &str = "TIBIA_COIN";
const EXERCISE_WEAPON_PRICES: [u64; 3] = [25, 90, 720];

struct Row {
    started: std::time::Instant,
    step: &'static str,
}

impl Row {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            step: "admission",
        }
    }

    fn at(&mut self, step: &'static str) {
        self.step = step;
    }

    fn finish(self, result: TestResult) {
        let (status, detail) = match result {
            Ok(()) => (
                "PASS",
                "catalogue-purchase-refusal-history-relog".to_owned(),
            ),
            Err(error) => (
                "FAIL",
                format!(
                    "step-{}-observed-{}",
                    self.step,
                    evidence_token(&error.to_string())
                ),
            ),
        };
        evidence(&format!(
            "e2e_all scenario=E10.full status={status} seconds={:.3} evidence={detail}",
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = row_complete_with_timeout(Duration::from_secs(180), journey(&mut row, clients)).await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err("timeout-waiting-for-server-reply".into()),
    });
}

async fn command(
    client: &mut DevClientSession,
    capability: u32,
    kind: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let reply = client
        .session_mut()
        .gated_command(capability, kind, payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "reply-status-{:?}-payload-{}-bytes",
            reply.status,
            reply.payload.len()
        )
        .into());
    }
    Ok(reply.payload)
}

async fn catalogue(client: &mut DevClientSession) -> TestResult<wire::StoreCataloguePage> {
    let payload = wire::encode_catalogue_request(&wire::StoreCatalogueRequest {
        category: CATEGORY.into(),
        page_cursor: String::new(),
    })
    .map_err(|error| format!("catalogue-encode-{error:?}"))?;
    let bytes = command(
        client,
        wire::CAPABILITY_STORE_V1,
        wire::COMMAND_TYPE_STORE_CATALOGUE_PAGE,
        &payload,
    )
    .await?;
    let page = wire::decode_catalogue_page(&bytes)
        .map_err(|error| format!("catalogue-decode-{error:?}"))?;
    if page.offers.is_empty()
        || page.offers.len() > wire::MAX_STORE_PAGE_ENTRIES
        || page.catalog_revision.is_empty()
        || page.transferable_coins != page.total_coins
    {
        return Err(format!("invalid-catalogue-page-{page:?}").into());
    }
    if let Some(offer) = page
        .offers
        .iter()
        .find(|offer| offer.currency != CURRENCY || !EXERCISE_WEAPON_PRICES.contains(&offer.price))
    {
        return Err(format!("unexpected-catalogue-offer-{offer:?}").into());
    }
    Ok(page)
}

async fn purchase(
    client: &mut DevClientSession,
    request: &wire::StorePurchaseRequest,
) -> TestResult<wire::StorePurchaseResult> {
    let payload = wire::encode_purchase_request(request)
        .map_err(|error| format!("purchase-encode-{error:?}"))?;
    let bytes = command(
        client,
        wire::CAPABILITY_STORE_V1,
        wire::COMMAND_TYPE_STORE_PURCHASE,
        &payload,
    )
    .await?;
    wire::decode_purchase_result(&bytes)
        .map_err(|error| format!("purchase-decode-{error:?}").into())
}

async fn history(client: &mut DevClientSession) -> TestResult<wire::StoreHistoryPage> {
    let payload = wire::encode_history_request(&wire::StoreHistoryRequest {
        page_cursor: String::new(),
    })
    .map_err(|error| format!("history-encode-{error:?}"))?;
    let bytes = command(
        client,
        wire::CAPABILITY_STORE_PURCHASE_HISTORY_V1,
        wire::COMMAND_TYPE_STORE_PURCHASE_HISTORY_PAGE,
        &payload,
    )
    .await?;
    wire::decode_history_page(&bytes).map_err(|error| format!("history-decode-{error:?}").into())
}

async fn logout(client: &mut RowClient) -> TestResult {
    let reply = client.row_logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("logout-reply-{reply:?}").into());
    }
    Ok(())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let mut client = row_connect_world(clients, 0xe0).await?;
    let original_session = client.join_snapshot().game_session_id;

    row.at("catalogue");
    let page = catalogue(&mut client).await?;
    let offer = page
        .offers
        .iter()
        .filter(|offer| offer.price <= page.total_coins)
        .min_by_key(|offer| offer.price)
        .ok_or_else(|| format!("no-affordable-offer-balance-{}", page.total_coins))?;

    row.at("purchase");
    let accepted_request = request(0xe1, offer, &page.catalog_revision, offer.price);
    let accepted = purchase(&mut client, &accepted_request).await?;
    if accepted.code != wire::StorePurchaseCode::Accepted
        || accepted.order_id.is_none()
        || accepted.operation_id.is_none()
    {
        return Err(format!("purchase-reply-{accepted:?}").into());
    }
    let balance_after_purchase = catalogue(&mut client).await?;
    if page.total_coins.checked_sub(accepted_request.price)
        != Some(balance_after_purchase.total_coins)
    {
        return Err(format!(
            "purchase-balance-before-{}-price-{}-after-{}",
            page.total_coins, accepted_request.price, balance_after_purchase.total_coins
        )
        .into());
    }

    row.at("stale-quote-refusal");
    let stale_price = offer.price.checked_add(1).ok_or("offer-price-overflow")?;
    let stale = purchase(
        &mut client,
        &request(0xe2, offer, &page.catalog_revision, stale_price),
    )
    .await?;
    if stale.code != wire::StorePurchaseCode::StaleQuote
        || stale.order_id.is_some()
        || stale.operation_id.is_some()
    {
        return Err(format!("stale-quote-reply-{stale:?}").into());
    }

    row.at("history");
    let first = history(&mut client).await?;
    assert_history(
        &first,
        &accepted_request,
        accepted.order_id.ok_or("accepted-order-absent")?,
    )?;

    row.at("logout");
    logout(&mut client).await?;
    drop(client);

    row.at("fresh-admission");
    let mut fresh = row_connect_world(clients, 0xe3).await?;
    if fresh.join_snapshot().game_session_id == original_session {
        return Err("fresh-admission-reused-session".into());
    }
    let restored_balance = catalogue(&mut fresh).await?;
    if restored_balance.total_coins != balance_after_purchase.total_coins {
        return Err(format!(
            "relog-balance-expected-{}-observed-{}",
            balance_after_purchase.total_coins, restored_balance.total_coins
        )
        .into());
    }
    row.at("relog-history");
    let restored = history(&mut fresh).await?;
    assert_history(
        &restored,
        &accepted_request,
        accepted.order_id.ok_or("accepted-order-absent")?,
    )?;
    row.at("final-logout");
    logout(&mut fresh).await
}

fn request(
    seed: u8,
    offer: &wire::StoreOffer,
    revision: &str,
    price: u64,
) -> wire::StorePurchaseRequest {
    let mut id = [seed; 16];
    id[6] = 0x70 | (seed & 0x0f);
    id[8] = 0x80 | (seed & 0x3f);
    wire::StorePurchaseRequest {
        purchase_request_id: id,
        offer_key: offer.offer_key.clone(),
        catalog_revision: revision.into(),
        price,
        currency: offer.currency.clone(),
    }
}

fn assert_history(
    page: &wire::StoreHistoryPage,
    request: &wire::StorePurchaseRequest,
    order_id: [u8; 16],
) -> TestResult {
    if !page.entries.iter().any(|entry| {
        entry.order_id == order_id
            && entry.offer_key == request.offer_key
            && entry.catalog_revision == request.catalog_revision
            && entry.price == request.price
            && entry.currency == request.currency
            && entry.purchased_at_unix_ms > 0
    }) {
        return Err(format!(
            "accepted-purchase-missing-from-{}-history-rows",
            page.entries.len()
        )
        .into());
    }
    Ok(())
}

fn evidence_token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':' | '/')
            {
                character
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_is_one_report_token() {
        assert_eq!(
            evidence_token("Rejected { payload: [] }"),
            "Rejected---payload:-----"
        );
    }
}
