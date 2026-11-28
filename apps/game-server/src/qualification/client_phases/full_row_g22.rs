//! G22: compare complete Store catalogue views, then spend disposable Platform
//! coins on an exercise sword and verify its delivery receipt and popularity increment.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::store::{self, browse as wire};
use std::collections::BTreeSet;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "store-browse-and-popularity-validated".to_owned()),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G22.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let joined = value.split_whitespace().collect::<Vec<_>>().join("-");
    joined.chars().take(400).collect()
}

fn gate(client: &DevClientSession, capability: u32) -> TestResult {
    if client.selected_capabilities().contains(&capability) {
        return Ok(());
    }
    Err(format!(
        "step=admit observed=capability-{capability}-not-selected needed=capability-{capability}-selected"
    )
    .into())
}

async fn send(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    gate(client, capability)?;
    let reply = client
        .session_mut()
        .gated_command(capability, command, payload)
        .await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    Ok((reply.status, reply.payload))
}

async fn finish(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

const SANDBOX_OFFER: &str = "store:exercise-sword";

async fn browse(client: &mut DevClientSession, sort: u8, cursor: String) -> TestResult<wire::Page> {
    let query = wire::Query {
        category: "featured".to_owned(),
        search: String::new(),
        sort,
        cursor,
    };
    let payload = query
        .encode()
        .map_err(|error| format!("step=browse observed=encode-{error:?}"))?;
    let (status, reply) =
        send(client, store::CAPABILITY_STORE_V1, wire::COMMAND_TYPE, &payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!(
            "step=browse observed=status:{status:?} needed=platform-store-catalogue-api-and-balance"
        )
        .into());
    }
    let page = wire::Page::decode(&reply)
        .map_err(|error| format!("step=browse observed=decode-{error:?}"))?;
    Ok(page)
}

fn same_offers<T: PartialEq>(left: &[T], right: &[T], key: impl Fn(&T) -> &str) -> bool {
    let mut left: Vec<_> = left.iter().collect();
    let mut right: Vec<_> = right.iter().collect();
    left.sort_by(|a, b| key(a).cmp(key(b)));
    right.sort_by(|a, b| key(a).cmp(key(b)));
    left == right
}

async fn catalogue(
    client: &mut DevClientSession,
    sort: u8,
    first: wire::Page,
) -> TestResult<Vec<wire::Product>> {
    let mut page = first;
    let mut offers = Vec::new();
    let mut keys = BTreeSet::new();
    loop {
        if page.products.is_empty() || page.categories.is_empty() {
            return Err(format!("step=browse observed=empty-page-for-sort-{sort}").into());
        }
        for product in page.products {
            if !keys.insert(product.offer.offer_key.clone()) {
                return Err(format!("step=browse observed=duplicate-offer-for-sort-{sort}").into());
            }
            offers.push(product);
        }
        if page.next.is_empty() {
            return Ok(offers);
        }
        page = browse(client, sort, page.next).await?;
    }
}

async fn popularity(connection: &mut sqlx::PgConnection, offer_key: &str) -> TestResult<i64> {
    let purchases = sqlx::query_scalar(
        "SELECT purchases FROM game_store_offer_popularity WHERE offer_key = $1",
    )
    .bind(offer_key)
    .fetch_optional(connection)
    .await?;
    Ok(purchases.unwrap_or(0))
}

async fn body(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    let first = browse(client, 0, String::new()).await?;
    let total = first.total;
    let offers = catalogue(client, 0, first).await?;
    for sort in 1..=4 {
        let first = browse(client, sort, String::new()).await?;
        let sorted = catalogue(client, sort, first).await?;
        if !same_offers(&offers, &sorted, |product| &product.offer.offer_key) {
            return Err(format!("step=sort observed=offer-set-differs-for-sort-{sort}").into());
        }
    }

    let product = offers
        .iter()
        .find(|product| product.offer.offer_key == SANDBOX_OFFER)
        .ok_or("step=purchase observed=sandbox-offer-absent")?;
    let price = product.offer.price;
    let expected_balance = total.checked_sub(price).ok_or(
        "step=purchase observed=insufficient-coins needed=funded-disposable-platform-wallet",
    )?;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let before = popularity(&mut connection, SANDBOX_OFFER).await?;
    let request = store::StorePurchaseRequest {
        purchase_request_id: crate::node::operator_files::uuid_v7()?,
        offer_key: product.offer.offer_key.clone(),
        catalog_revision: product.revision.clone(),
        price,
        currency: product.offer.currency.clone(),
    };
    let payload = store::encode_purchase_request(&request)
        .map_err(|error| format!("step=purchase observed=encode-{error:?}"))?;
    let (status, reply) = send(
        client,
        store::CAPABILITY_STORE_V1,
        store::COMMAND_TYPE_STORE_PURCHASE,
        &payload,
    )
    .await?;
    if status != CommandStatus::Accepted {
        return Err(format!("step=purchase observed=status:{status:?}").into());
    }
    let reply = store::decode_purchase_result(&reply)
        .map_err(|error| format!("step=purchase observed=decode-{error:?}"))?;
    let (store::StorePurchaseCode::Accepted, Some(order), Some(operation)) =
        (reply.code, reply.order_id, reply.operation_id)
    else {
        return Err(format!(
            "step=purchase observed=code:{:?} needed=sandbox-purchase",
            reply.code
        )
        .into());
    };

    let delivered: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM game_store_purchase_history h
            JOIN store_delivery_receipts r ON r.purchase_request_id = h.purchase_request_id
                AND r.account_id = h.account_id AND r.character_id = h.character_id
            WHERE h.purchase_request_id = encode($1, 'hex')::uuid
                AND h.order_id = encode($2, 'hex')::uuid
                AND r.operation_id = encode($3, 'hex')::uuid
                AND h.offer_key = $4 AND r.item_instance_id IS NOT NULL
        )",
    )
    .bind(request.purchase_request_id.as_slice())
    .bind(order.as_slice())
    .bind(operation.as_slice())
    .bind(SANDBOX_OFFER)
    .fetch_one(&mut connection)
    .await?;
    let after = popularity(&mut connection, SANDBOX_OFFER).await?;
    if !delivered || before.checked_add(1) != Some(after) {
        return Err(format!(
            "step=popularity observed=delivery:{delivered}-before:{before}-after:{after}"
        )
        .into());
    }
    let after_purchase = browse(client, 0, String::new()).await?;
    if after_purchase.total != expected_balance {
        return Err(format!(
            "step=balance observed={} expected={expected_balance}",
            after_purchase.total
        )
        .into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x86).await?;
    let outcome = verdict::observe(body(&mut client, clients)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    outcome?;
    closed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_is_one_bounded_token() {
        assert_eq!(
            compact("step=a  observed=b\nneeded=c"),
            "step=a-observed=b-needed=c"
        );
        assert!(!compact(&"x ".repeat(500)).contains(' '));
        assert!(compact(&"y".repeat(900)).len() <= 400);
    }

    #[test]
    fn offer_sets_compare_without_order() {
        let a = vec!["x".to_owned(), "y".to_owned()];
        let b = vec!["y".to_owned(), "x".to_owned()];
        assert!(same_offers(&a, &b, String::as_str));
        assert!(!same_offers(&a, &["x".to_owned()], String::as_str));
    }
}
