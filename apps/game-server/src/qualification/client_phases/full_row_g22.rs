//! G22 store browse and offer popularity (capability 154, command 6118).
//! 1. Admit the node-boot character and browse the featured category (no payment is made).
//!    The server answers only from the Platform store catalogue and balance projection; the
//!    pinned node-boot Platform exposes no store catalogue API, so the browse is refused and the
//!    row fails naming that precondition.
//! 2. Read every page for each sort and compare the ordered offers, including ties.
//! 3. Offer popularity is only populated by purchase receipts, which need a Platform payment
//!    sandbox; the row never makes a real payment and fails naming that precondition.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::store::browse as wire;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
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

const CAPABILITY: u32 = 154;

fn ordered_sort(sort: u8, base: &[wire::Product], actual: &[wire::Product]) -> bool {
    let mut expected: Vec<_> = base.iter().collect();
    expected.sort_by(|left, right| {
        let left = &left.offer;
        let right = &right.offer;
        let primary = match sort {
            1 => left.title.to_lowercase().cmp(&right.title.to_lowercase()),
            2 => right.title.to_lowercase().cmp(&left.title.to_lowercase()),
            3 => left.price.cmp(&right.price),
            4 => right.price.cmp(&left.price),
            _ => std::cmp::Ordering::Equal,
        };
        primary.then_with(|| left.offer_key.cmp(&right.offer_key))
    });
    expected.into_iter().eq(actual)
}

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
    let (status, reply) = send(client, CAPABILITY, wire::COMMAND_TYPE, &payload).await?;
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

async fn all_offers(client: &mut DevClientSession, sort: u8) -> TestResult<Vec<wire::Product>> {
    let mut cursor = String::new();
    let mut cursors = std::collections::BTreeSet::new();
    let mut keys = std::collections::BTreeSet::new();
    let mut offers = Vec::new();
    loop {
        let page = browse(client, sort, cursor).await?;
        if page.categories.is_empty() || page.products.is_empty() {
            return Err(format!("step=browse observed=empty-category-or-products-for-sort-{sort}").into());
        }
        for product in page.products {
            if !keys.insert(product.offer.offer_key.clone()) {
                return Err(format!("step=browse observed=duplicate-offer:{}", product.offer.offer_key).into());
            }
            offers.push(product);
        }
        if page.next.is_empty() {
            return Ok(offers);
        }
        if !cursors.insert(page.next.clone()) {
            return Err(format!("step=browse observed=repeated-cursor-for-sort-{sort}").into());
        }
        cursor = page.next;
    }
}

async fn body(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    let base = all_offers(client, 0).await?;
    for sort in 1..=4 {
        let offers = all_offers(client, sort).await?;
        if !ordered_sort(sort, &base, &offers) {
            return Err(format!("step=sort observed=incorrect-offer-order-or-content-for-sort-{sort}").into());
        }
    }
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let rows: i64 = sqlx::query_scalar("SELECT count(*)::bigint FROM game_store_offer_popularity")
        .fetch_one(&mut connection)
        .await?;
    Err(format!(
        "step=popularity observed=rows:{rows} needed=platform-payment-sandbox-purchase-receipts"
    )
    .into())
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
    fn store_sort_checks_offer_order_and_identity() {
        let product = |key: &str, price| wire::Product {
            category: "featured".into(), revision: "common".into(),
            offer: oteryn_protocol_oteryn::store::StoreOffer {
                offer_key: key.into(), title: key.into(), price, currency: "COINS".into(),
            },
            artwork: wire::Artwork::default(),
        };
        let base = [product("a", 3), product("b", 2), product("c", 1)];
        let expected = [base[2].clone(), base[1].clone(), base[0].clone()];
        assert!(!ordered_sort(3, &base, &base));
        assert!(!ordered_sort(3, &base, &[base[2].clone(), base[2].clone(), base[0].clone()]));
        assert!(ordered_sort(3, &base, &expected));
    }

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
        let product = |key: &str| wire::Product {
            category: "featured".into(), revision: "catalogue".into(),
            offer: oteryn_protocol_oteryn::store::StoreOffer {
                offer_key: key.into(), title: key.into(), price: 10, currency: "COINS".into(),
            },
            artwork: wire::Artwork::default(),
        };
        let a = [product("x"), product("y")];
        let b = [product("y"), product("x")];
        assert!(ordered_sort(1, &b, &a));
        assert!(!ordered_sort(1, &a, &a[..1]));
    }
}
