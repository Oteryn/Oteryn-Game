//! G26: house sale escrow and durable ownership through the admitted wire.
use super::*;
use oteryn_dev_client::DevClientSession;
use sqlx::Connection;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G26.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

async fn character_name(clients: &SeamClients<'_>, index: usize) -> TestResult<String> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let name: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id = encode($1, 'hex')::uuid",
    )
    .bind(clients.characters[index].as_slice())
    .fetch_one(&mut database)
    .await?;
    database.close().await?;
    Ok(name)
}

use oteryn_protocol_oteryn::house_sale as sale;

const HOUSE: &str = "oteryn:content.house.sunset_homes_flat_01";
const PRICE: u64 = 200;
const RENT: u64 = 100_000; // Authored rent for Sunset Homes Flat 01.

async fn prepare_owned_house(seller: &SeamClients<'_>, buyer: &SeamClients<'_>) -> TestResult {
    let mut database = sqlx::PgConnection::connect(seller.url).await?;
    let mut tx = database.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let world = seller.world.as_slice();
    let result = sqlx::query(
        "WITH property AS (
            UPDATE game_house_properties p
            SET state=3, revision=p.revision+1, owner_character_id=r.character_id,
                owner_account_id=r.account_id, acquisition_receipt_ref=game_character_uuid_v7(),
                paid_until=statement_timestamp()+interval '30 days'
            FROM game_character_roots r
            WHERE p.world_id=encode($1,'hex')::uuid AND p.house_key=$2 AND p.rent_gold=$4
              AND p.state=1 AND p.owner_character_id IS NULL AND p.owner_account_id IS NULL
              AND r.world_id=p.world_id AND r.character_id=encode($3,'hex')::uuid
            RETURNING p.*
        ), slot AS (
            INSERT INTO game_account_housing_slots(account_id, world_id, house_key)
            SELECT owner_account_id, world_id, house_key FROM property
        )
        UPDATE game_house_ownership o
        SET owner_character_id=p.owner_character_id, ownership_revision=p.ownership_generation,
            acquisition_receipt_ref=p.acquisition_receipt_ref
        FROM property p WHERE o.world_id=p.world_id AND o.house_key=p.house_key
          AND o.owner_character_id IS NULL",
    )
    .bind(world)
    .bind(HOUSE)
    .bind(seller.characters[1].as_slice())
    .bind(i64::try_from(RENT)?)
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() != 1 {
        return Err("step=fixture observed=missing-vacant-authored-house-or-seller".into());
    }
    sqlx::query(include_str!("g26_bank_fixture.sql"))
        .bind(&buyer.accounts[0])
        .bind(world)
        .bind(buyer.channel.as_slice())
        .bind(buyer.characters[0].as_slice())
        .bind(i64::try_from(PRICE + RENT)?)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

fn action(kind: u8, revision: u64) -> sale::HouseSaleRequest {
    sale::HouseSaleRequest {
        action: kind,
        house: HOUSE.into(),
        expected_revision: revision,
        due_at_unix_ms: 0,
        recipient: String::new(),
        price: 0,
        after_house: String::new(),
    }
}

async fn send(
    client: &mut DevClientSession,
    request: &sale::HouseSaleRequest,
    expected: CommandStatus,
) -> TestResult<Option<sale::HouseSaleReply>> {
    let payload = sale::encode_request(request)
        .map_err(|error| format!("step=encode action={} observed={error:?}", request.action))?;
    let response = client
        .session_mut()
        .gated_command(sale::CAPABILITY, sale::COMMAND, &payload)
        .await?;
    if response.status != expected {
        return Err(format!(
            "step=sale-wire action={} observed={:?} needed={expected:?}",
            request.action, response.status
        )
        .into());
    }
    if expected == CommandStatus::Rejected {
        return Ok(None);
    }
    Ok(Some(sale::decode_reply(&response.payload).map_err(
        |error| format!("step=decode action={} observed={error:?}", request.action),
    )?))
}

async fn accepted(
    client: &mut DevClientSession,
    request: &sale::HouseSaleRequest,
) -> TestResult<sale::HouseSaleReply> {
    send(client, request, CommandStatus::Accepted)
        .await?
        .ok_or_else(|| "missing accepted reply".into())
}

fn only_house(page: &sale::HouseSaleReply) -> TestResult<&sale::HouseSaleRow> {
    match page.rows.as_slice() {
        [row] if row.house == HOUSE && !page.has_more => Ok(row),
        _ => Err(format!(
            "step=catalogue observed=houses:{} needed={HOUSE}",
            page.rows.len()
        )
        .into()),
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    use super::full_row_e06::bank_balance;

    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_Q_G26_ACCOUNT").await?;
    let buyer_scope = dedicated.clients(clients);
    let seller_name = character_name(clients, 1).await?;
    let buyer_name = character_name(&buyer_scope, 0).await?;
    let mut buyer = connect_character(&buyer_scope, 0xd6, 0).await?;
    let original_buyer_session = buyer.join_snapshot().game_session_id;
    logout(&mut buyer).await?;
    drop(buyer);

    prepare_owned_house(clients, &buyer_scope).await?;
    let seller_gold = bank_balance(clients, 1).await?;
    let buyer_gold = bank_balance(&buyer_scope, 0).await?;
    if buyer_gold != PRICE + RENT {
        return Err(format!("step=fixture observed=buyer-gold:{buyer_gold}").into());
    }
    let mut seller = connect_world(clients, 0xd6).await?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let is_owner: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM game_durability_reconnect_sessions s
            JOIN game_house_properties p ON p.world_id=s.world_id AND p.owner_character_id=s.character_id
            WHERE s.game_session_id=encode($1,'hex')::uuid AND p.house_key=$2
              AND p.owner_account_id=s.account_id
        )",
    )
    .bind(seller.join_snapshot().game_session_id.as_bytes().as_slice())
    .bind(HOUSE)
    .fetch_one(&mut database)
    .await?;
    if !is_owner {
        return Err("step=initial-ownership observed=admitted-seller-is-not-owner".into());
    }
    evidence("row=G26.full fixture=authored-house owner=admitted-seller");
    let mut buyer = connect_character(&buyer_scope, 0xd7, 0).await?;
    if buyer.join_snapshot().game_session_id == original_buyer_session {
        return Err("step=buyer-reconnect observed=original-session".into());
    }
    character_prep::grant_premium(&buyer).await?;
    evidence("row=G26.full fixture=premium buyer=reconnected");
    for client in [&seller, &buyer] {
        if !client.selected_capabilities().contains(&sale::CAPABILITY) {
            return Err(format!(
                "step=admit observed=capability-{}-not-selected",
                sale::CAPABILITY
            )
            .into());
        }
    }

    let request = sale::HouseSaleRequest::catalogue(String::new());
    let buyer_before = accepted(&mut buyer, &request).await?;
    if buyer_before.has_more || !buyer_before.rows.is_empty() {
        return Err("step=initial-buyer observed=visible-house-before-offer".into());
    }
    let original = accepted(&mut seller, &request).await?;
    let row = only_house(&original)?;
    if row.owner != seller_name || row.transfer_state != 0 || row.rent != RENT {
        return Err(format!("step=initial-ownership observed={row:?}").into());
    }
    let mut offer = action(3, row.revision);
    offer.due_at_unix_ms = original.now_unix_ms + 172_800_000;
    offer.recipient = buyer_name.clone();
    offer.price = PRICE;
    let offered = accepted(&mut seller, &offer).await?;
    let incoming = accepted(&mut buyer, &request).await?;
    let row = only_house(&incoming)?;
    if row.owner != seller_name
        || row.recipient != buyer_name
        || row.transfer_state != 1
        || row.revision != offer.expected_revision + 1
        || row.due_at_unix_ms != offer.due_at_unix_ms
        || row.price != PRICE
        || row.rent != RENT
        || row.allowed != ((1 << 5) | (1 << 6))
        || only_house(&offered)?.revision != row.revision
        || bank_balance(&buyer_scope, 0).await? != buyer_gold
        || bank_balance(clients, 1).await? != seller_gold
    {
        return Err(format!("step=offer observed={row:?}").into());
    }

    let mut unauthorized = offer.clone();
    unauthorized.expected_revision = row.revision;
    unauthorized.recipient = seller_name.clone();
    send(&mut buyer, &unauthorized, CommandStatus::Rejected).await?;
    let after = accepted(&mut buyer, &request).await?;
    if after.rows != incoming.rows
        || accepted(&mut seller, &request).await?.rows != offered.rows
        || bank_balance(&buyer_scope, 0).await? != buyer_gold
        || bank_balance(clients, 1).await? != seller_gold
    {
        return Err("step=offer-not-owner observed=catalogue-or-balance-changed".into());
    }
    let accept = action(5, row.revision);
    send(&mut seller, &accept, CommandStatus::Rejected).await?;
    if accepted(&mut seller, &request).await?.rows != offered.rows
        || accepted(&mut buyer, &request).await?.rows != incoming.rows
        || bank_balance(&buyer_scope, 0).await? != buyer_gold
        || bank_balance(clients, 1).await? != seller_gold
    {
        return Err("step=accept-not-recipient observed=catalogue-or-balance-changed".into());
    }
    let escrowed = accepted(&mut buyer, &accept).await?;
    let escrow = only_house(&escrowed)?;
    if escrow.transfer_state != 2
        || escrow.owner != seller_name
        || escrow.recipient != buyer_name
        || escrow.revision != row.revision + 1
        || escrow.price != PRICE
        || escrow.rent != RENT
        || bank_balance(&buyer_scope, 0).await? != buyer_gold - PRICE - RENT
        || bank_balance(clients, 1).await? != seller_gold
    {
        return Err(format!("step=escrow observed={escrow:?}").into());
    }

    let original_calendar: String = sqlx::query_scalar(
        "SELECT pg_get_functiondef('game_house_calendar_milliseconds(timestamptz)'::regprocedure)",
    )
    .fetch_one(&mut database)
    .await?;
    let (offer_id, disposition_id, due, advance): (String, String, i64, i64) = sqlx::query_as(
        "SELECT o.offer_id::text, d.operation_id::text, d.due_at,
                    d.due_at - floor(extract(epoch FROM clock_timestamp())*1000)::bigint + 1
             FROM game_house_sale_offers o
             JOIN game_house_dispositions d ON d.operation_id=o.disposition
             JOIN game_house_sale_holds h ON h.offer_id=o.offer_id
             WHERE o.world_id=encode($1,'hex')::uuid AND o.house_key=$2 AND o.state=2
               AND d.state=1 AND d.released_at IS NULL",
    )
    .bind(clients.world.as_slice())
    .bind(HOUSE)
    .fetch_one(&mut database)
    .await?;
    if due != escrow.due_at_unix_ms || advance <= 0 {
        return Err(format!("step=settlement-setup observed=due:{due},advance:{advance}").into());
    }
    // Use the persisted server deadline to advance the existing qualification calendar.
    let advance_calendar = format!(
        "CREATE OR REPLACE FUNCTION game_house_calendar_milliseconds(t TIMESTAMPTZ)
         RETURNS BIGINT LANGUAGE sql IMMUTABLE STRICT
         AS $$ SELECT floor(extract(epoch FROM t)*1000)::bigint + {advance} $$"
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(advance_calendar))
        .execute(&mut database)
        .await?;
    let settled: TestResult<sale::HouseSaleRow> = async {
        let now: i64 = sqlx::query_scalar(
            "SELECT game_house_calendar_milliseconds(clock_timestamp())",
        )
        .fetch_one(&mut database)
        .await?;
        if now <= due {
            return Err(format!("step=settlement-setup observed=server-now:{now},due:{due}").into());
        }
        evidence(&format!(
            "row=G26.full escrow_id={offer_id} disposition_id={disposition_id} due_ms={due} server_now_ms={now}"
        ));
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            let page = accepted(&mut buyer, &request).await?;
            let current = only_house(&page)?;
            if current.owner == buyer_name {
                if current.transfer_state != 0 || !current.recipient.is_empty() {
                    return Err(format!("step=settlement observed={current:?}").into());
                }
                return Ok(current.clone());
            }
            if current.owner != seller_name
                || current.transfer_state != 2
                || tokio::time::Instant::now() >= deadline
            {
                return Err(format!("step=settlement observed={current:?}").into());
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
    .await;
    sqlx::raw_sql(sqlx::AssertSqlSafe(original_calendar))
        .execute(&mut database)
        .await?;
    let settled = settled?;

    let buyer_session = buyer.join_snapshot().game_session_id;
    let seller_session = seller.join_snapshot().game_session_id;
    logout(&mut buyer).await?;
    logout(&mut seller).await?;
    drop((buyer, seller));
    let mut buyer = connect_character(&buyer_scope, 0xd8, 0).await?;
    let mut seller = connect_world(clients, 0xd9).await?;
    let durable = accepted(&mut buyer, &request).await?;
    let row = only_house(&durable)?;
    if buyer.join_snapshot().game_session_id == buyer_session
        || seller.join_snapshot().game_session_id == seller_session
        || row != &settled
        || row.owner != buyer_name
        || row.transfer_state != 0
        || !row.recipient.is_empty()
        || !accepted(&mut seller, &request).await?.rows.is_empty()
        || bank_balance(&buyer_scope, 0).await? != buyer_gold - PRICE - RENT
        || bank_balance(clients, 1).await? != seller_gold + PRICE
    {
        return Err(format!("step=relog observed={row:?}").into());
    }
    logout(&mut buyer).await?;
    logout(&mut seller).await?;
    Ok("owner-and-recipient-authority;exact-escrow;settled-owner-and-balances-after-relog".into())
}
