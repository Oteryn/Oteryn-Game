//! E10 client replay capture: real Store catalogue, entitlement purchase, stale-quote refusal and
//! purchase-history payloads. Client owners: `apply_result` in
//! `crates/client-ui/src/features/0154-store/protocol.rs` and `apply_result` in
//! `crates/client-ui/src/features/0285-store-purchase-history/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::store as wire;
use std::fmt::Write as _;

const CATEGORY: &str = "consumables.exercise-weapons";

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("E10_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
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
        return Err(format!("command {kind} status={:?}", reply.status).into());
    }
    Ok(reply.payload)
}

fn request(seed: u8, offer: &wire::StoreOffer, revision: &str, price: u64) -> wire::StorePurchaseRequest {
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x3e).await?;
    let catalogue_payload = command(
        &mut client,
        wire::CAPABILITY_STORE_V1,
        wire::COMMAND_TYPE_STORE_CATALOGUE_PAGE,
        &wire::encode_catalogue_request(&wire::StoreCatalogueRequest {
            category: CATEGORY.into(),
            page_cursor: String::new(),
        })
        .map_err(|error| format!("catalogue encode: {error:?}"))?,
    )
    .await?;
    let page = wire::decode_catalogue_page(&catalogue_payload)
        .map_err(|error| format!("catalogue decode: {error:?}"))?;
    let (index, offer) = page
        .offers
        .iter()
        .enumerate()
        .filter(|(_, offer)| offer.price <= page.total_coins)
        .min_by_key(|(_, offer)| offer.price)
        .ok_or_else(|| format!("no affordable offer, balance={}", page.total_coins))?;

    let accepted_request = request(0x3e, offer, &page.catalog_revision, offer.price);
    let accepted_payload = command(
        &mut client,
        wire::CAPABILITY_STORE_V1,
        wire::COMMAND_TYPE_STORE_PURCHASE,
        &wire::encode_purchase_request(&accepted_request)
            .map_err(|error| format!("purchase encode: {error:?}"))?,
    )
    .await?;
    let accepted = wire::decode_purchase_result(&accepted_payload)
        .map_err(|error| format!("purchase decode: {error:?}"))?;
    let (Some(order), Some(operation)) = (accepted.order_id, accepted.operation_id) else {
        return Err(format!("purchase not accepted: {accepted:?}").into());
    };
    if accepted.code != wire::StorePurchaseCode::Accepted {
        return Err(format!("purchase code {:?}", accepted.code).into());
    }

    let stale_request = request(
        0x3d,
        offer,
        &page.catalog_revision,
        offer.price.checked_add(1).ok_or("price overflow")?,
    );
    let stale_payload = command(
        &mut client,
        wire::CAPABILITY_STORE_V1,
        wire::COMMAND_TYPE_STORE_PURCHASE,
        &wire::encode_purchase_request(&stale_request)
            .map_err(|error| format!("stale encode: {error:?}"))?,
    )
    .await?;
    let stale = wire::decode_purchase_result(&stale_payload)
        .map_err(|error| format!("stale decode: {error:?}"))?;
    if stale.code != wire::StorePurchaseCode::StaleQuote
        || stale.order_id.is_some()
        || stale.operation_id.is_some()
    {
        return Err(format!("stale quote reply {stale:?}").into());
    }

    let history_payload = command(
        &mut client,
        wire::CAPABILITY_STORE_PURCHASE_HISTORY_V1,
        wire::COMMAND_TYPE_STORE_PURCHASE_HISTORY_PAGE,
        &wire::encode_history_request(&wire::StoreHistoryRequest {
            page_cursor: String::new(),
        })
        .map_err(|error| format!("history encode: {error:?}"))?,
    )
    .await?;
    let history = wire::decode_history_page(&history_payload)
        .map_err(|error| format!("history decode: {error:?}"))?;
    let entry = history
        .entries
        .iter()
        .find(|entry| entry.order_id == order)
        .ok_or_else(|| format!("order missing from {} history rows", history.entries.len()))?;
    if entry.offer_key != offer.offer_key || entry.price != offer.price {
        return Err(format!("history row {entry:?} differs from purchased offer").into());
    }

    let mut expected = format!("category={CATEGORY}\nbalance={}\n", page.total_coins);
    for row in &page.offers {
        let _ = writeln!(expected, "offer={}|{}|{}|{}", row.offer_key, row.title, row.price, row.currency);
    }
    let _ = writeln!(expected, "purchased_index={index}");
    let _ = writeln!(expected, "purchase={:?}|{}|{}", accepted.code, hex(&order), hex(&operation));
    let _ = writeln!(expected, "stale={:?}", stale.code);
    let _ = writeln!(
        expected,
        "history={}|{}|{}|{}|{}",
        hex(&entry.order_id),
        entry.offer_key,
        entry.price,
        entry.currency,
        entry.purchased_at_unix_ms
    );
    capture("e10-catalogue.bin", &catalogue_payload);
    capture("e10-purchase.bin", &accepted_payload);
    capture("e10-stale.bin", &stale_payload);
    capture("e10-history.bin", &history_payload);
    capture("e10-expected.txt", expected.as_bytes());
    let _ = client.session_mut().logout().await;
    Ok(format!("balance={} price={} order={}", page.total_coins, offer.price, hex(&order)))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=E10.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E10.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=E10.client status=FAIL evidence=timeout"),
    }
}
