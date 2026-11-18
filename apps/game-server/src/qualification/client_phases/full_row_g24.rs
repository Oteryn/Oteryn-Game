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

fn market_evidence_matches(
    statistics: &wire::Statistics,
    expected_statistics: &wire::Statistics,
    book: &[oteryn_protocol_oteryn::market::MarketRow],
    expected_book: &[oteryn_protocol_oteryn::market::MarketRow],
) -> bool {
    statistics.trades > 0 && statistics == expected_statistics && book == expected_book
}

async fn trade_statistics(
    db: &mut sqlx::PgConnection,
    world: &[u8; 16],
    key: &str,
    observed: &wire::Statistics,
) -> TestResult<wire::Statistics> {
    type SummaryRow = (Option<i16>, i64, String, Option<String>, Option<String>, Option<String>);
    let rows: Vec<SummaryRow> = sqlx::query_as(
        "WITH trades AS (
            SELECT DISTINCT ON (h.transaction_id) h.transaction_id,h.price,h.amount,o.side
            FROM game_market_settlements s JOIN game_market_history h USING(transaction_id)
            JOIN game_market_offers o ON o.offer_id=s.offer_id AND o.world_id=s.world_id
            WHERE s.world_id=encode($1,'hex')::uuid AND o.definition_key=$2 AND s.kind IN(2,5)
              AND s.occurred_at_us BETWEEN $3 AND $4
            ORDER BY h.transaction_id,h.character_id
         ) SELECT side,count(*),coalesce(sum(amount),0)::text,min(price)::text,max(price)::text,floor(avg(price))::text
           FROM trades GROUP BY GROUPING SETS ((side),())",
    ).bind(world.as_slice()).bind(key).bind(observed.since_us).bind(observed.as_of_us)
        .fetch_all(&mut *db).await?;
    let mut result = wire::Statistics {
        definition: observed.definition, since_us: observed.since_us, as_of_us: observed.as_of_us,
        ..Default::default()
    };
    for (side, trades, units, minimum, maximum, average) in rows {
        let summary = wire::PriceSummary {
            trades: u64::try_from(trades)?, units: units.parse()?,
            minimum: minimum.map(|value| value.parse()).transpose()?,
            maximum: maximum.map(|value| value.parse()).transpose()?,
            average: average.map(|value| value.parse()).transpose()?,
        };
        match side {
            Some(1) => result.buy = summary,
            Some(2) => result.sell = summary,
            None => {
                result.trades = summary.trades;
                result.units = summary.units;
                result.minimum = summary.minimum;
                result.maximum = summary.maximum;
                result.average = summary.average;
            }
            _ => return Err("step=statistics observed=invalid-settlement-side".into()),
        }
    }
    Ok(result)
}

async fn first_book_page(
    db: &mut sqlx::PgConnection,
    world: &[u8; 16],
    key: &str,
    revision: &str,
    definition: u32,
    as_of: i64,
) -> TestResult<(Vec<oteryn_protocol_oteryn::market::MarketRow>, bool)> {
    type BookRow = (Vec<u8>, i32, String, bool, String);
    let mut book = Vec::new();
    let mut has_next = false;
    for side in [1i16, 2] {
        let rows: Vec<BookRow> = sqlx::query_as(
            "SELECT uuid_send(offer_id),remaining,piece_price::text,anonymous,definition_revision
             FROM game_market_offers
             WHERE world_id=encode($1,'hex')::uuid AND definition_key=$2 AND side=$3 AND state=1
               AND created_at_us<=$4 AND expires_at_us>$4
             ORDER BY CASE WHEN side=1 THEN piece_price END DESC,
                      CASE WHEN side=2 THEN piece_price END ASC,created_at_us,offer_id LIMIT 33",
        ).bind(world.as_slice()).bind(key).bind(side).bind(as_of).fetch_all(&mut *db).await?;
        has_next |= rows.len() > 32;
        for (id, amount, price, anonymous, stored_revision) in rows.into_iter().take(32) {
            if stored_revision == revision {
                book.push(oteryn_protocol_oteryn::market::MarketRow {
                    offer: id.try_into().map_err(|_| "step=book observed=invalid-offer-id")?,
                    definition, side: u8::try_from(side)?, amount: u32::try_from(amount)?,
                    price: price.parse()?, state: 1, anonymous,
                });
            }
        }
    }
    Ok((book, has_next))
}

fn statistics_timestamp_is_current(stamp: i64, sent: i64, read: i64) -> bool {
    (sent..=read).contains(&stamp)
}

async fn database_time(database: &mut sqlx::PgConnection) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp())*1000000)::bigint")
        .fetch_one(database).await?)
}

async fn body(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
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
    use sqlx::Connection;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let statistics_sent = database_time(&mut database).await?;
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
    let statistics_read = database_time(&mut database).await?;
    let book_sent = database_time(&mut database).await?;
    let wire::Reply::Book {
        definition: echoed,
        page,
        rows,
        statistics: book_statistics,
        has_next,
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
    if echoed != definition || page != 0 {
        return Err(format!("step=book observed=definition:{echoed},page:{page};needed={definition},0").into());
    }
    let book_read = database_time(&mut database).await?;
    let index = provisioning::item_index()?;
    let (key, revision) = index.definition_at(std::num::NonZeroU32::new(definition).ok_or("step=catalogue observed=zero-definition")?)
        .ok_or("step=catalogue observed=unknown-item-definition")?;
    if !statistics_timestamp_is_current(statistics.as_of_us, statistics_sent, statistics_read)
        || !statistics_timestamp_is_current(book_statistics.as_of_us, book_sent, book_read)
    {
        return Err("step=statistics observed=noncurrent-statistics-window".into());
    }
    let expected = trade_statistics(&mut database, &clients.world, key, &statistics).await?;
    let expected_book_stats = trade_statistics(&mut database, &clients.world, key, &book_statistics).await?;
    let (expected_book, expected_next) = first_book_page(&mut database, &clients.world, key, revision, definition, book_statistics.as_of_us).await?;
    if has_next != expected_next || book_statistics != expected_book_stats
        || !market_evidence_matches(&statistics, &expected, &rows, &expected_book)
    {
        return Err(format!(
            "step=market-proof observed=trades:{},book-rows:{},next:{has_next};needed=exact-durable-statistics-and-book:{},{},{expected_next}",
            statistics.trades, rows.len(), expected.trades, expected_book.len()
        ).into());
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
    fn market_rejects_nonzero_fabricated_statistics_and_book_rows() {
        use oteryn_protocol_oteryn::market::MarketRow;
        let actual = wire::Statistics {
            definition: 7, trades: 1, buy: wire::PriceSummary {
                trades: 1, units: 1, minimum: Some(50), maximum: Some(50), average: Some(50),
            }, ..Default::default()
        };
        let book = [MarketRow { offer: [3; 16], definition: 7, side: 1, amount: 1, price: 50,
            state: 1, anonymous: false }];
        let mut fabricated = actual.clone();
        fabricated.buy.average = Some(40);
        assert!(!market_evidence_matches(&fabricated, &actual, &book, &book));
        assert!(!market_evidence_matches(&actual, &actual, &book, &[]));
        assert!(market_evidence_matches(&actual, &actual, &book, &book));
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
    fn search_term_is_a_short_lowercase_prefix() {
        assert_eq!(search_term("Magic Plate Armor"), "magi");
        assert_eq!(search_term("Ax"), "ax");
    }
}

#[cfg(test)]
mod delta_tests {
    use super::*;

    #[test]
    fn statistics_cutoff_cannot_predate_the_request() {
        assert!(!statistics_timestamp_is_current(30_000_000, 45_000_000, 46_000_000));
        assert!(statistics_timestamp_is_current(45_500_000, 45_000_000, 46_000_000));
    }
}
