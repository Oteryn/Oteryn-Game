//! G22 store browse and offer popularity (capability 154, command 6118).
//! Browse the live catalogue and verify that sort changes preserve the Platform balance
//! and offer set. Compare the default order with Game's persisted purchase counts.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::store::browse as wire;
use sqlx::Connection;

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

const CAPABILITY: u32 = 154;

fn same_offers(left: &[String], right: &[String]) -> bool {
    let (mut left, mut right) = (left.to_vec(), right.to_vec());
    left.sort();
    right.sort();
    left == right
}

async fn browse_page(
    client: &mut DevClientSession,
    sort: u8,
    cursor: String,
) -> TestResult<wire::Page> {
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

async fn browse(client: &mut DevClientSession, sort: u8) -> TestResult<wire::Page> {
    let mut catalogue = browse_page(client, sort, String::new()).await?;
    let mut cursors = std::collections::BTreeSet::new();
    while !catalogue.next.is_empty() {
        let cursor = std::mem::take(&mut catalogue.next);
        if !cursors.insert(cursor.clone()) || cursors.len() > 64 {
            return Err(format!("step=browse observed=invalid-pagination-for-sort-{sort}").into());
        }
        let page = browse_page(client, sort, cursor).await?;
        if (
            page.total,
            page.transferable,
            page.transferable_known,
            &page.categories,
        ) != (
            catalogue.total,
            catalogue.transferable,
            catalogue.transferable_known,
            &catalogue.categories,
        ) || page.products.is_empty()
        {
            return Err(format!("step=browse observed=inconsistent-page-for-sort-{sort}").into());
        }
        catalogue.products.extend(page.products);
        catalogue.next = page.next;
    }
    Ok(catalogue)
}

async fn body(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    let base = browse(client, 0).await?;
    if base.products.is_empty() || base.categories.is_empty() {
        return Err(format!(
            "step=browse observed=products:{}-categories:{} needed=store-catalogue-offers",
            base.products.len(),
            base.categories.len()
        )
        .into());
    }
    let offers: Vec<String> = base
        .products
        .iter()
        .map(|p| p.offer.offer_key.clone())
        .collect();
    if base.transferable_known || base.transferable != 0 {
        return Err("step=balance observed=invalid-transferable-balance".into());
    }
    for sort in 1..=4 {
        let page = browse(client, sort).await?;
        let other: Vec<String> = page
            .products
            .iter()
            .map(|p| p.offer.offer_key.clone())
            .collect();
        if (page.total, page.transferable, page.transferable_known)
            != (base.total, base.transferable, base.transferable_known)
        {
            return Err(format!("step=balance observed=balance-differs-for-sort-{sort}").into());
        }
        if !same_offers(&offers, &other) {
            return Err(format!("step=sort observed=offer-set-differs-for-sort-{sort}").into());
        }
    }
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let expected: Vec<String> = sqlx::query_scalar(
        "SELECT keys.offer_key FROM unnest($1::text[]) AS keys(offer_key) \
         LEFT JOIN game_store_offer_popularity p USING (offer_key) \
         ORDER BY COALESCE(p.purchases, 0) DESC, keys.offer_key COLLATE \"C\"",
    )
    .bind(&offers)
    .fetch_all(&mut connection)
    .await?;
    if offers != expected {
        return Err("step=popularity observed=offer-order-differs-from-purchase-counts".into());
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
        assert!(same_offers(&a, &b));
        assert!(!same_offers(&a, &["x".to_owned()]));
    }
}
