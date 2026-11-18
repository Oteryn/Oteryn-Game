//! E06 — Market expiry/refunds, scheduling, and all marketable content.
//!
//! Player scenario derived from the E06 audit/matrix row and the market settlement owner:
//! 1. Walk the provisioned character to an authored depot locker, send MARKET wares pages, and
//!    require accepted replies whose rows are bounded and contain distinct definition references.
//! 2. Send a MARKET book query for every definition advertised by those pages and require the
//!    accepted, matching book reply; this exercises all marketable content served by this world.
//! 3. Send a book query for an unadvertised definition and require refusal code 14, proving the
//!    non-marketable-content path rather than treating an empty book as acceptance.
//! 4. Send a one-unit buy-offer command for an advertised definition and require its exact offer
//!    id in My Offers; a refusal reports the observed code and the missing player prerequisite.
//! 5. Send cancel for that offer, require it to disappear from My Offers and appear in History;
//!    this is the player-observable buy-escrow refund path owned by market settlement.
//! 6. Log out, obtain a fresh admission, walk back to the locker, and re-check My Offers and
//!    History so the ended offer and refund outcome are shown to be durable.
//! 7. Leave a second buy offer open and age its timestamps in the disposable database.
//!    Observe scheduled expiry before sending any market command, then check the refund and history.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::market::{self as wire, MarketReply, MarketRequest};
use std::collections::BTreeSet;

const OFFER_PRICE: i64 = 1;
const OFFER_FEE: i64 = 20;
const EXPIRED_STATE: i16 = 4;

async fn command(client: &mut DevClientSession, request: MarketRequest) -> TestResult<MarketReply> {
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
        return Err(format!("market command transport status={:?}", response.status).into());
    }
    wire::decode_reply(&response.payload)
        .map_err(|error| format!("market reply decode: {error:?}").into())
}

fn request(action: u8, definition: u32, offer: [u8; 16], page: u32) -> MarketRequest {
    MarketRequest {
        action,
        definition,
        offer,
        amount: u32::from(action == 5),
        price: u64::from(action == 5),
        anonymous: false,
        page,
    }
}

async fn at_market(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        near_locker(world, at)
    })
    .ok_or("E06 step 1: no reachable authored depot locker")?;
    walk(client, world, &path).await
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0xe6, 1).await?;
    at_market(&mut client, world).await?;

    let mut definitions = Vec::new();
    let mut seen = BTreeSet::new();
    for page in 0..=100_000 {
        let reply = command(&mut client, request(1, 0, [0; 16], page)).await?;
        if reply.code != 1 || reply.action != 1 || reply.rows.len() > 32 {
            return Err(format!("E06 step 1: wares page={page} observed={reply:?}").into());
        }
        let count = reply.rows.len();
        for row in reply.rows {
            if row.definition == 0 || !seen.insert(row.definition) {
                return Err(format!("E06 step 1: duplicate/zero ware observed={row:?}").into());
            }
            definitions.push(row.definition);
        }
        if count < 32 {
            break;
        }
    }
    if definitions.is_empty() {
        return Err("E06 step 1: accepted wares query advertised no marketable content".into());
    }
    evidence(&format!(
        "E06_CHECK step=1 status=PASS definitions={}",
        definitions.len()
    ));

    for &definition in &definitions {
        let reply = command(&mut client, request(2, definition, [0; 16], 0)).await?;
        if reply.code != 1
            || reply.action != 2
            || reply.rows.iter().any(|r| r.definition != definition)
        {
            return Err(format!("E06 step 2: definition={definition} observed={reply:?}").into());
        }
    }
    evidence("E06_CHECK step=2 status=PASS all-advertised-books-accepted");

    let invalid = (1..=u32::MAX)
        .rev()
        .find(|definition| !seen.contains(definition))
        .ok_or("E06 step 3: no unadvertised definition reference")?;
    let refusal = command(&mut client, request(2, invalid, [0; 16], 0)).await?;
    if refusal.code != 14 {
        return Err(format!("E06 step 3: expected code=14 observed={refusal:?}").into());
    }
    evidence("E06_CHECK step=3 status=PASS observed-code=14");

    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let before = bank_gold(&mut db, clients).await?;
    let created = command(&mut client, request(5, definitions[0], [0; 16], 0)).await?;
    if created.code != 1 {
        return Err(format!("E06 step 4: create observed-code={}; need premium-current and at least fee+escrow bank gold", created.code).into());
    }
    let offer = created.offer;
    let own = command(&mut client, request(3, 0, [0; 16], 0)).await?;
    if own.code != 1
        || !own
            .rows
            .iter()
            .any(|row| row.offer == offer && row.amount == 1 && row.price == 1)
    {
        return Err(format!("E06 step 4: created offer missing observed={own:?}").into());
    }
    evidence("E06_CHECK step=4 status=PASS offer-visible");

    let bank_with_escrow = bank_gold(&mut db, clients).await?;
    if before.checked_sub(OFFER_FEE + OFFER_PRICE) != Some(bank_with_escrow)
        || offer_state(&mut db, clients, offer).await? != (1, OFFER_PRICE, 0)
    {
        return Err("E06 step 4: expected fee debit and exact buy escrow funding".into());
    }
    let expected_refund = bank_with_escrow + OFFER_PRICE;
    let cancelled = command(&mut client, request(7, 0, offer, 0)).await?;
    if cancelled.code != 1 || cancelled.action != 7 {
        return Err(format!("E06 step 5: cancel observed={cancelled:?}").into());
    }
    let own = command(&mut client, request(3, 0, [0; 16], 0)).await?;
    let history = command(&mut client, request(4, 0, [0; 16], 0)).await?;
    if !settled_offer(
        &own,
        &history,
        offer,
        bank_gold(&mut db, clients).await?,
        expected_refund,
        offer_state(&mut db, clients, offer).await?,
        3,
    ) {
        return Err(format!(
            "E06 step 5: refund/history delta missing own={own:?} history={history:?}"
        )
        .into());
    }
    evidence("E06_CHECK step=5 status=PASS cancelled-refund-history");

    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("E06 step 6: logout refused".into());
    }
    let mut client = connect_character(clients, 0xe7, 1).await?;
    at_market(&mut client, world).await?;
    let own = command(&mut client, request(3, 0, [0; 16], 0)).await?;
    let history = command(&mut client, request(4, 0, [0; 16], 0)).await?;
    if !settled_offer(
        &own,
        &history,
        offer,
        bank_gold(&mut db, clients).await?,
        expected_refund,
        offer_state(&mut db, clients, offer).await?,
        3,
    ) {
        return Err(format!(
            "E06 step 6: fresh-admission state mismatch own={own:?} history={history:?}"
        )
        .into());
    }
    evidence("E06_CHECK step=6 status=PASS durable-cancelled-refund");
    expire_offer(
        &mut client,
        &mut db,
        clients,
        definitions[0],
        expected_refund,
    )
    .await?;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("E06 step 6: final logout refused".into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
            .await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "e2e_all scenario=E06.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E06.full status=FAIL seconds={:.3} evidence={}",
            started.elapsed().as_secs_f64(),
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence(&format!(
            "e2e_all scenario=E06.full status=FAIL seconds={:.3} evidence=step-deadline-timeout",
            started.elapsed().as_secs_f64()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_shapes_match_market_actions() {
        assert!(wire::encode_request(request(1, 0, [0; 16], 4)).is_ok());
        assert!(wire::encode_request(request(2, 7, [0; 16], 0)).is_ok());
        assert!(wire::encode_request(request(5, 7, [0; 16], 0)).is_ok());
        let mut offer = [0; 16];
        offer[6] = 0x70;
        offer[8] = 0x80;
        assert!(wire::encode_request(request(7, 0, offer, 0)).is_ok());
    }
}

async fn bank_gold(db: &mut sqlx::PgConnection, clients: &SeamClients<'_>) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=encode($2,'hex')::uuid),0)::bigint",
    ).bind(&clients.accounts[1]).bind(clients.world.as_slice()).fetch_one(db).await?)
}

async fn offer_state(
    db: &mut sqlx::PgConnection,
    clients: &SeamClients<'_>,
    offer: [u8; 16],
) -> TestResult<(i16, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT state,escrow_gold::bigint,held_credit_gold::bigint FROM game_market_offers WHERE offer_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid",
    ).bind(offer.as_slice()).bind(clients.world.as_slice()).bind(clients.characters[1].as_slice()).fetch_one(db).await?)
}

async fn expire_offer(
    client: &mut DevClientSession,
    db: &mut sqlx::PgConnection,
    clients: &SeamClients<'_>,
    definition: u32,
    before: i64,
) -> TestResult {
    if E2eMode::from_env()? == E2eMode::Journey {
        evidence("e2e_game_gap scenario=E06.full cause=authored-market-expiry-exceeds-row-budget");
        return Err(verdict::MissingServerRoute(concat!("source:", file!(), ":", line!())).into());
    }
    let created = command(client, request(5, definition, [0; 16], 0)).await?;
    let offer = created.offer;
    if created.code != 1
        || created.action != 5
        || offer_state(db, clients, offer).await? != (1, OFFER_PRICE, 0)
        || before.checked_sub(OFFER_FEE + OFFER_PRICE) != Some(bank_gold(db, clients).await?)
    {
        return Err(format!(
            "E06 step 7: expiry requires a funded open buy offer; observed={created:?}"
        )
        .into());
    }
    age_fixture_offer(db, &offer, &clients.world, &clients.characters[1]).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(75);
    verdict::Harness::global().bounded_wait(deadline, "market-expiry");
    let scheduled = loop {
        let (state, _, _) = offer_state(db, clients, offer).await?;
        if state != 1 || tokio::time::Instant::now() >= deadline {
            break state == EXPIRED_STATE;
        }
        client.service_liveness(Duration::from_millis(250)).await?;
        client.take_events();
    };
    // History is itself a market command and can trigger demand-driven settlement.
    let history = command(client, request(4, 0, [0; 16], 0)).await?;
    let own = command(client, request(3, 0, [0; 16], 0)).await?;
    let settled = settled_offer(
        &own,
        &history,
        offer,
        bank_gold(db, clients).await?,
        before - OFFER_FEE,
        offer_state(db, clients, offer).await?,
        EXPIRED_STATE,
    );
    expiry_verdict(scheduled, settled)
}

fn settled_offer(
    own: &MarketReply,
    history: &MarketReply,
    offer: [u8; 16],
    bank: i64,
    expected: i64,
    state: (i16, i64, i64),
    ended_state: i16,
) -> bool {
    own.code == 1
        && own.action == 3
        && history.code == 1
        && history.action == 4
        && state == (ended_state, 0, 0)
        && bank == expected
        && !own.rows.iter().any(|row| row.offer == offer)
        && history.rows.iter().any(|row| {
            row.offer == offer
                && row.state == 2
                && row.price == 1
                && row.amount == 1
                && row.side == 1
        })
}

fn expiry_verdict(scheduled: bool, settled: bool) -> TestResult {
    match (scheduled, settled) {
        (true, true) => Ok(()),
        (false, true) => Err("E06 step 7: scheduled expiry unobserved before the market command; subsequent settlement refunded the escrow".into()),
        (_, false) => Err("E06 step 7: expected expired state, refund and successful own-offers/history replies".into()),
    }
}

#[test]
fn expiry_verdict_requires_observed_settlement_and_schedule() {
    assert!(expiry_verdict(true, true).is_ok());
    assert!(expiry_verdict(false, true).is_err());
    assert!(expiry_verdict(false, false).is_err());
}

#[test]
fn cancellation_checks_refund_and_successful_queries() {
    let offer = [1; 16];
    let mut own = MarketReply {
        action: 3,
        code: 1,
        offer: [0; 16],
        rows: vec![],
    };
    let history = MarketReply {
        action: 4,
        code: 1,
        offer: [0; 16],
        rows: vec![wire::MarketRow {
            offer,
            definition: 1,
            side: 1,
            amount: 1,
            price: 1,
            state: 2,
            anonymous: false,
        }],
    };
    assert!(settled_offer(&own, &history, offer, 100, 100, (3, 0, 0), 3));
    assert!(!settled_offer(&own, &history, offer, 99, 100, (3, 0, 0), 3));
    assert!(!settled_offer(
        &own,
        &history,
        offer,
        100,
        100,
        (5, 0, 1),
        3
    ));
    own.code = 9;
    assert!(!settled_offer(
        &own,
        &history,
        offer,
        100,
        100,
        (3, 0, 0),
        3
    ));
}

async fn age_fixture_offer(
    db: &mut sqlx::PgConnection, offer: &[u8; 16], world: &[u8; 16], character: &[u8; 16],
) -> TestResult {
    // As in the dispatch expiration fixture, override the immutable timestamps
    // in a local transaction on the disposable database and leave settlement to the scheduler.
    let mut tx = db.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    let aged = sqlx::query(
        "UPDATE game_market_offers SET created_at_us=1,expires_at_us=2592000000001 WHERE offer_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid AND state=1",
    ).bind(offer.as_slice()).bind(world.as_slice()).bind(character.as_slice())
        .execute(&mut *tx).await?;
    if aged.rows_affected() != 1 {
        return Err("E06 step 7: expiry fixture lost its open offer".into());
    }
    tx.commit().await?;
    Ok(())
}

#[test]
fn round2_ages_an_open_offer_with_the_real_guard_installed() -> TestResult {
    let Some(url) = crate::bestiary_postgres_harness::configured_admin() else { return Ok(()) };
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let mut db = sqlx::PgConnection::connect(&url).await?;
        db.execute("CREATE TEMP TABLE game_market_offers (
            offer_id uuid, world_id uuid, definition_key text, definition_revision text,
            side int, account_id uuid, character_id uuid, piece_price int, amount int,
            anonymous bool, created_at_us bigint, expires_at_us bigint, state int
        )").await?;
        db.execute("INSERT INTO game_market_offers VALUES (
            '00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000002','item','r1',
            1,'00000000-0000-0000-0000-000000000004','00000000-0000-0000-0000-000000000003',1,1,false,2,2592000000002,1
        )").await?;
        let migration = include_str!("../../../migrations/0155_market_offer_book.sql");
        let (_, guard) = migration.split_once("CREATE FUNCTION game_market_offer_guard").ok_or("market guard absent")?;
        let (guard, _) = guard.split_once("CREATE FUNCTION game_market_operation_immutable").ok_or("market guard boundary absent")?;
        let guard = format!("CREATE FUNCTION game_market_offer_guard{guard}").replace("game_market_offer_guard()", "pg_temp.game_market_offer_guard()");
        sqlx::raw_sql(sqlx::AssertSqlSafe(guard)).execute(&mut db).await?;
        age_fixture_offer(&mut db, &1_u128.to_be_bytes(), &2_u128.to_be_bytes(), &3_u128.to_be_bytes()).await?;
        let times: (i64,i64) = sqlx::query_as("SELECT created_at_us,expires_at_us FROM game_market_offers").fetch_one(&mut db).await?;
        assert_eq!(times,(1,2592000000001));
        let role: String = sqlx::query_scalar("SHOW session_replication_role").fetch_one(&mut db).await?;
        assert_eq!(role,"origin");
        Ok(())
    })
}
