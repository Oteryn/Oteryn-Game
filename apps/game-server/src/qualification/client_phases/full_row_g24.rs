//! G24 market details and statistics (capability 63, command 6126).
//! 1. Admit the node-boot character; read the market catalogue page 0: every ware must satisfy
//!    the (empty) filter, and a name search built from the first ware must keep that ware.
//! 2. Query the statistics and the order book of that ware: the echoed definition must match.
//! 3. The price statistics must come from completed trades; a node-boot server with no trade
//!    history fails naming that precondition. The row never creates offers or trades.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::market::CAPABILITY_MARKET_V1;
use oteryn_protocol_oteryn::market_details as wire;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "market-catalogue-statistics-and-book-validated".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G24.full status={status} seconds={:.3} evidence={detail}",
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

async fn accepted(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let (status, payload) = send(client, capability, command, payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!(
            "step=command-{command} observed=status:{status:?} needed=accepted-reply"
        )
        .into());
    }
    Ok(payload)
}

async fn finish(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

fn search_term(name: &str) -> String {
    name.chars().take(4).collect::<String>().to_lowercase()
}

async fn ask(client: &mut DevClientSession, query: &wire::Query) -> TestResult<wire::Reply> {
    let payload = wire::encode_query(query)
        .map_err(|error| format!("step=query observed=encode-{error:?}"))?;
    let reply = accepted(
        client,
        CAPABILITY_MARKET_V1,
        wire::COMMAND_TYPE_MARKET_DETAILS,
        &payload,
    )
    .await?;
    let reply = wire::decode_reply(&reply)
        .map_err(|error| format!("step=query observed=decode-{error:?}"))?;
    Ok(reply)
}

async fn body(client: &mut DevClientSession, _clients: &SeamClients<'_>) -> TestResult {
    let filter = wire::Filter::default();
    let wire::Reply::Catalogue { items, .. } = ask(
        client,
        &wire::Query::Catalogue {
            filter: filter.clone(),
            page: 0,
        },
    )
    .await?
    else {
        return Err("step=catalogue observed=not-a-catalogue".into());
    };
    let Some(first) = items.first() else {
        return Err("step=catalogue observed=empty needed=market-catalogue".into());
    };
    if items.len() > wire::PAGE_SIZE || !items.iter().all(|ware| filter.matches(ware)) {
        return Err(format!("step=catalogue observed=items:{}", items.len()).into());
    }
    let search = wire::Filter {
        search: search_term(&first.name),
        ..wire::Filter::default()
    };
    let wire::Reply::Catalogue { items: found, .. } = ask(
        client,
        &wire::Query::Catalogue {
            filter: search.clone(),
            page: 0,
        },
    )
    .await?
    else {
        return Err("step=search observed=not-a-catalogue".into());
    };
    if !found.iter().any(|ware| ware.definition == first.definition)
        || !found.iter().all(|ware| search.matches(ware))
    {
        return Err(format!("step=search observed=items:{}", found.len()).into());
    }
    let definition = first.definition;
    let wire::Reply::Statistics(statistics) =
        ask(client, &wire::Query::Statistics { definition }).await?
    else {
        return Err("step=statistics observed=not-statistics".into());
    };
    if statistics.definition != definition {
        return Err(format!(
            "step=statistics observed=definition-{}",
            statistics.definition
        )
        .into());
    }
    let wire::Reply::Book {
        definition: echoed,
        rows,
        ..
    } = ask(
        client,
        &wire::Query::Book {
            definition,
            page: 0,
        },
    )
    .await?
    else {
        return Err("step=book observed=not-a-book".into());
    };
    if echoed != definition {
        return Err(format!("step=book observed=definition-{echoed}").into());
    }
    let trades = statistics.buy.trades + statistics.sell.trades;
    if trades == 0 {
        return Err(format!(
            "step=statistics observed=trades:0-book-rows:{} needed=completed-market-trade-history",
            rows.len()
        )
        .into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x8a).await?;
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
    fn search_term_is_a_short_lowercase_prefix() {
        assert_eq!(search_term("Magic Plate Armor"), "magi");
        assert_eq!(search_term("Ax"), "ax");
    }
}
