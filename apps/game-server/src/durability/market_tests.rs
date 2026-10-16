//! Proposed owner schema + real PostgreSQL reads/locks. Fixtures do not admit marketable
//! content, create current authority, or qualify the missing production item/money guards.
#![allow(clippy::expect_used)]
use super::*;
use crate::bestiary_postgres_harness as harness;
use crate::foundation::{CommandId, GameSessionId};
use harness::{CHARACTER, Harness, TestResult, WORLD, id};
use sqlx::Executor;

const PRIVATE_SCHEMA: &str = r#"
CREATE TABLE game_market_books(world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),definition_key TEXT NOT NULL,PRIMARY KEY(world_id,definition_key));
CREATE TABLE game_market_offers(
 offer_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(offer_id)),world_id UUID NOT NULL,definition_key TEXT NOT NULL,definition_revision TEXT NOT NULL,
 side SMALLINT NOT NULL CHECK(side IN(1,2)),account_id UUID NOT NULL REFERENCES game_character_account_guards,character_id UUID NOT NULL REFERENCES game_character_roots,
 piece_price NUMERIC(20,0) NOT NULL CHECK(piece_price BETWEEN 1 AND 999999999999),amount INT NOT NULL CHECK(amount BETWEEN 1 AND 64000),remaining INT NOT NULL CHECK(remaining BETWEEN 0 AND amount),
 anonymous BOOLEAN NOT NULL DEFAULT FALSE,created_at_us BIGINT NOT NULL CHECK(created_at_us>=0),expires_at_us BIGINT NOT NULL CHECK(expires_at_us=created_at_us+2592000000000),
 state SMALLINT NOT NULL CHECK(state BETWEEN 1 AND 5),ended_as SMALLINT CHECK(ended_as IN(2,3,4)),
 escrow_gold NUMERIC(20,0) NOT NULL CHECK(escrow_gold BETWEEN 0 AND 999999999999),held_credit_gold NUMERIC(20,0) NOT NULL CHECK(held_credit_gold BETWEEN 0 AND 18446744073709551615),matching_pending BOOLEAN NOT NULL DEFAULT FALSE,
 UNIQUE(offer_id,world_id),FOREIGN KEY(world_id,definition_key) REFERENCES game_market_books,
 CHECK(piece_price*amount<=999999999999),CHECK((state=5)=(ended_as IS NOT NULL)),
 CHECK(escrow_gold=CASE WHEN side=1 AND state=1 THEN piece_price*remaining ELSE 0 END),
 CHECK((state=5 AND held_credit_gold>0) OR(state<>5 AND held_credit_gold=0)),
 CHECK(state<>1 OR remaining>0),CHECK(state<>2 OR remaining=0),CHECK(ended_as<>2 OR remaining=0),CHECK(state=1 OR NOT matching_pending));
CREATE INDEX market_pending ON game_market_offers(world_id,definition_key,created_at_us,offer_id) WHERE state=1 AND matching_pending;
CREATE INDEX market_expiring ON game_market_offers(world_id,expires_at_us,offer_id) WHERE state=1;
CREATE INDEX market_sell_price ON game_market_offers(world_id,definition_key,piece_price ASC,created_at_us,offer_id) WHERE state=1 AND side=2;
CREATE INDEX market_buy_price ON game_market_offers(world_id,definition_key,piece_price DESC,created_at_us,offer_id) WHERE state=1 AND side=1;
CREATE TABLE game_item_market_escrow_locations(
 item_instance_id UUID PRIMARY KEY,world_id UUID NOT NULL,offer_id UUID NOT NULL,ordinal NUMERIC(20,0) NOT NULL CHECK(ordinal BETWEEN 1 AND 18446744073709551615),
 UNIQUE(offer_id,ordinal),FOREIGN KEY(item_instance_id,world_id) REFERENCES game_item_instances(item_instance_id,world_id) ON DELETE RESTRICT,
 FOREIGN KEY(offer_id,world_id) REFERENCES game_market_offers(offer_id,world_id) ON DELETE RESTRICT);
CREATE TABLE game_market_operations(
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),occurrence BYTEA NOT NULL CHECK(octet_length(occurrence) BETWEEN 26 AND 27),
 transaction_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),kind SMALLINT NOT NULL CHECK(kind BETWEEN 1 AND 6),actor UUID REFERENCES game_character_roots,
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),outcome BYTEA NOT NULL CHECK(octet_length(outcome) BETWEEN 1 AND 512),
 PRIMARY KEY(occurrence),CHECK((kind<=3)=(actor IS NOT NULL)));
CREATE FUNCTION market_fixture_immutable() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'immutable Market occurrence' USING ERRCODE='23514'; END;$$;
CREATE TRIGGER market_fixture_immutable BEFORE UPDATE OR DELETE ON game_market_operations FOR EACH ROW EXECUTE FUNCTION market_fixture_immutable();
REVOKE ALL ON game_market_books,game_market_offers,game_item_market_escrow_locations,game_market_operations FROM PUBLIC;
"#;

fn world() -> WorldId {
    WorldId::from_bytes(id(WORLD)).expect("world")
}
fn offer(seed: u8, side: Side, account: u8, created: i64) -> Offer {
    Offer {
        id: MarketOfferId::new(id(seed)).expect("offer"),
        world: world(),
        definition: TypedDefinitionRef::new(
            DefinitionFamily::Item,
            ProductionKey::new("oteryn:content.item.vase").expect("key"),
            DefinitionRevisionRef::new("r1").expect("revision"),
        ),
        side,
        account: AccountId::from_bytes(id(account)).expect("account"),
        character: CharacterId::from_bytes(id(account + 1)).expect("character"),
        piece_price: 100,
        amount: 10,
        remaining: 10,
        anonymous: false,
        created_at_us: created,
        expires_at_us: created + LIFETIME_US,
        state: OfferState::Open,
        escrow_gold: if side == Side::Buy { 1000 } else { 0 },
        held_credit_gold: 0,
        matching_pending: false,
    }
}
fn price(o: &mut Offer, p: u64) {
    o.piece_price = p;
    o.escrow_gold = if o.side == Side::Buy && o.state == OfferState::Open {
        u64::from(o.remaining) * p
    } else {
        0
    };
}
fn run<F>(tag: &str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    harness::runtime()?.block_on(async move {
        let h = Harness::create(admin, tag, false).await?;
        h.pool.execute(PRIVATE_SCHEMA).await?;
        let result = body(&h).await;
        h.cleanup().await?;
        result
    })
}
async fn now(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&h.pool)
            .await?,
    )
}
async fn seed(h: &Harness, o: &Offer) -> TestResult {
    // Administrative storage fixtures only. No live session/root capability is inferred.
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING").bind(o.account.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,profile_revision,ruleset_revision,content_revision,starter_template_revision,$4 FROM game_character_roots WHERE character_id=encode($5,'hex')::uuid ON CONFLICT(character_id) DO NOTHING")
        .bind(o.character.as_bytes().as_slice()).bind(o.account.as_bytes().as_slice()).bind(o.world.as_bytes().as_slice()).bind(format!("Market Fixture {}",char::from(b'A'+o.account.as_bytes()[0]/10))).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
    sqlx::query(
        "INSERT INTO game_market_books VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT DO NOTHING",
    )
    .bind(o.world.as_bytes().as_slice())
    .bind(o.definition.key().as_str())
    .execute(&h.pool)
    .await?;
    let (state, ended) = match o.state {
        OfferState::Open => (1_i16, None),
        OfferState::Ended(e) => (end_code(e), None),
        OfferState::CreditHeld(e) => (5, Some(end_code(e))),
    };
    sqlx::query("INSERT INTO game_market_offers VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8::text::numeric,$9,$10,$11,$12,$13,$14,$15,$16::text::numeric,$17::text::numeric,$18)")
        .bind(o.id.as_bytes().as_slice()).bind(o.world.as_bytes().as_slice()).bind(o.definition.key().as_str()).bind(o.definition.revision().as_str()).bind(if o.side==Side::Buy {1_i16}else{2}).bind(o.account.as_bytes().as_slice()).bind(o.character.as_bytes().as_slice()).bind(o.piece_price.to_string()).bind(i32::try_from(o.amount)?).bind(i32::try_from(o.remaining)?).bind(o.anonymous).bind(o.created_at_us).bind(o.expires_at_us).bind(state).bind(ended).bind(o.escrow_gold.to_string()).bind(o.held_credit_gold.to_string()).bind(o.matching_pending).execute(&h.pool).await?;
    Ok(())
}
fn end_code(e: EndState) -> i16 {
    match e {
        EndState::Filled => 2,
        EndState::Cancelled => 3,
        EndState::Expired => 4,
    }
}

#[test]
fn market_own_open_offers_are_scoped_ordered_and_include_expiry_backlog() -> TestResult {
    run("marketown", async |h| {
        let mut older = offer(60, Side::Buy, 40, 100);
        older.anonymous = true;
        let newer = offer(61, Side::Sell, 40, 200);
        let tied = offer(62, Side::Buy, 40, 200);
        let mut ended = offer(63, Side::Sell, 40, 300);
        ended.state = OfferState::Ended(EndState::Cancelled);
        let mut held = offer(64, Side::Buy, 40, 300);
        held.state = OfferState::CreditHeld(EndState::Cancelled);
        held.escrow_gold = 0;
        held.held_credit_gold = 1000;
        let other_character = offer(65, Side::Sell, 70, 400);
        for o in [&older, &newer, &tied, &ended, &held, &other_character] {
            seed(h, o).await?;
        }
        assert!(now(h).await? > tied.expires_at_us);
        let mut connection = h.pool.acquire().await?;
        assert_eq!(
            open_character_offers(&mut connection, world(), older.character).await?,
            vec![tied, newer, older.clone()]
        );
        assert!(
            open_character_offers(
                &mut connection,
                WorldId::from_bytes(id(90))?,
                older.character,
            )
            .await?
            .is_empty()
        );
        assert_eq!(
            open_character_offers(&mut connection, world(), other_character.character).await?,
            vec![other_character]
        );
        Ok(())
    })
}

#[test]
fn market_own_open_offers_reject_overflow_instead_of_truncating() -> TestResult {
    run("marketownbound", async |h| {
        for seed_id in 100..200 {
            seed(h, &offer(seed_id, Side::Sell, 40, 100)).await?;
        }
        let character = CharacterId::from_bytes(id(41))?;
        {
            let mut connection = h.pool.acquire().await?;
            assert_eq!(
                open_character_offers(&mut connection, world(), character)
                    .await?
                    .len(),
                100
            );
        }
        seed(h, &offer(200, Side::Sell, 40, 100)).await?;
        let mut connection = h.pool.acquire().await?;
        assert!(matches!(
            open_character_offers(&mut connection, world(), character).await,
            Err(DurabilityError::InvalidStoredState)
        ));
        Ok(())
    })
}

#[test]
fn market_checked_total_and_fee_limits() {
    assert_eq!(offer_total(1, MAX_TOTAL), Ok(MAX_TOTAL));
    for (amount, price) in [
        (0, 1),
        (64001, 1),
        (1, 0),
        (1, u64::MAX),
        (64000, MAX_TOTAL),
    ] {
        assert!(offer_total(amount, price).is_err());
    }
    assert_eq!(placement_fee(1, 1), Ok(20));
    assert_eq!(placement_fee(1, 1049), Ok(20));
    assert_eq!(placement_fee(1, 1050), Ok(21));
    assert_eq!(placement_fee(1, MAX_TOTAL), Ok(1_000_000));
}
#[test]
fn market_state_custody_guards_fail_closed() {
    let good = offer(60, Side::Buy, 40, 100);
    assert_eq!(good.validate(), Ok(()));
    let mut bad = good.clone();
    bad.escrow_gold -= 1;
    assert!(bad.validate().is_err());
    let mut bad = good.clone();
    bad.state = OfferState::CreditHeld(EndState::Cancelled);
    bad.held_credit_gold = 1;
    assert!(bad.validate().is_err());
    let mut held = good.clone();
    held.state = OfferState::CreditHeld(EndState::Cancelled);
    held.held_credit_gold = 1000;
    held.escrow_gold = 0;
    assert!(held.validate().is_ok());
    held.matching_pending = true;
    assert!(held.validate().is_err());
    let mut bad = good.clone();
    bad.expires_at_us -= 1;
    assert!(bad.validate().is_err());
    let mut bad = good;
    bad.remaining = 0;
    bad.escrow_gold = 0;
    assert!(bad.validate().is_err());
    bad.state = OfferState::Ended(EndState::Filled);
    assert!(bad.validate().is_ok());
}
#[test]
fn market_match_maker_price_and_exact_refund_without_compatibility_claim() {
    let mut buy = offer(60, Side::Buy, 40, 200);
    let sell = offer(61, Side::Sell, 70, 100);
    price(&mut buy, 120);
    let terms = match_terms(&buy, &sell, 4, 300).expect("terms");
    assert_eq!(
        (
            terms.maker,
            terms.sale_gold,
            terms.buy_escrow_debit,
            terms.buyer_refund_gold
        ),
        (sell.id, 400, 480, 80)
    );
    buy.created_at_us = 50;
    buy.expires_at_us = 50 + LIFETIME_US;
    assert_eq!(
        match_terms(&buy, &sell, 4, 300)
            .expect("buy maker")
            .buyer_refund_gold,
        0
    );
    let mut other = sell.clone();
    other.account = buy.account;
    assert_eq!(
        match_terms(&buy, &other, 1, 300),
        Err(PlanError::SameAccount)
    );
    let mut other = sell.clone();
    price(&mut other, 121);
    assert_eq!(
        match_terms(&buy, &other, 1, 300),
        Err(PlanError::NotCrossed)
    );
    for amount in [0, 11, u32::MAX] {
        assert!(match_terms(&buy, &sell, amount, 300).is_err());
    }
    assert!(match_terms(&buy, &sell, 1, buy.expires_at_us).is_err());
}
#[test]
fn market_cancel_expiry_and_held_credit_are_noncommitting_exact_requirements() {
    let buy = offer(60, Side::Buy, 40, 100);
    assert_eq!(
        end_plan(&buy, EndReason::Expire, 200),
        Err(PlanError::NotExpired)
    );
    let plan = end_plan(&buy, EndReason::Expire, buy.expires_at_us).expect("expire");
    assert_eq!(
        (plan.return_gold, plan.return_item_units, plan.ended_as),
        (1000, 0, EndState::Expired)
    );
    let sell = offer(61, Side::Sell, 70, 100);
    assert_eq!(
        end_plan(&sell, EndReason::Cancel, 200)
            .expect("cancel")
            .return_item_units,
        10
    );
    let mut held = buy;
    held.state = OfferState::CreditHeld(EndState::Cancelled);
    held.held_credit_gold = 1100;
    held.escrow_gold = 0;
    assert_eq!(
        held_credit_release(&held, HARD_CREDIT_CEILING - 1100),
        Ok(Some(1100))
    );
    assert_eq!(
        held_credit_release(&held, HARD_CREDIT_CEILING - 1099),
        Ok(None)
    );
    assert!(end_plan(&held, EndReason::Cancel, 200).is_err());
}

#[test]
fn market_native_offer_read_is_scoped_and_never_commits_a_return() -> TestResult {
    run("marketread", async |h| {
        let mut o = offer(60, Side::Buy, 40, now(h).await? - 1000);
        o.anonymous = true;
        seed(h, &o).await?;
        let mut tx = h.pool.begin().await?;
        let mut book = lock_book(&mut tx, o.book()).await?;
        assert_eq!(book.offer(o.id).await?, Some(o.clone()));
        assert!(!book.accepts_busy().await?);
        let p = book
            .plan_end(o.id, EndReason::Cancel)
            .await?
            .ok_or("plan")?;
        assert_eq!(p.return_gold, 1000);
        tx.commit().await?;
        let mut tx = h.pool.begin().await?;
        let mut book = lock_book(&mut tx, o.book()).await?;
        assert_eq!(book.offer(o.id).await?, Some(o.clone()));
        tx.rollback().await?;
        let mut foreign = o.book();
        foreign.world = WorldId::from_bytes(id(99))?;
        let mut tx = h.pool.begin().await?;
        let mut book = lock_book(&mut tx, foreign).await?;
        assert!(book.offer(o.id).await?.is_none());
        tx.rollback().await?;
        let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(revision, "1");
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_bank_entries")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(n, 0);
        Ok(())
    })
}
#[test]
fn market_native_pending_matching_is_oldest_best_price_account_filtered() -> TestResult {
    run("marketmatch", async |h| {
        let time = now(h).await? - 10000;
        let mut pending = offer(60, Side::Buy, 40, time + 500);
        price(&mut pending, 120);
        pending.matching_pending = true;
        seed(h, &pending).await?;
        let mut later = offer(66, Side::Buy, 80, time + 600);
        later.matching_pending = true;
        seed(h, &later).await?;
        let mut own = offer(61, Side::Sell, 40, time);
        price(&mut own, 1);
        seed(h, &own).await?;
        let mut first = offer(62, Side::Sell, 70, time + 100);
        price(&mut first, 80);
        first.definition = TypedDefinitionRef::new(
            DefinitionFamily::Item,
            first.definition.key().clone(),
            DefinitionRevisionRef::new("r2")?,
        );
        seed(h, &first).await?;
        let mut tie = offer(63, Side::Sell, 80, time + 200);
        price(&mut tie, 80);
        seed(h, &tie).await?;
        let mut worse = offer(64, Side::Sell, 80, time);
        price(&mut worse, 90);
        seed(h, &worse).await?;
        let mut tx = h.pool.begin().await?;
        let mut book = lock_book(&mut tx, pending.book()).await?;
        assert!(book.accepts_busy().await?);
        let pair = book.next_match().await?.ok_or("pair")?;
        assert_eq!(pair.pending.id, pending.id);
        let counter = pair.counter.ok_or("counter")?;
        assert_eq!(counter, first);
        assert_ne!(
            counter.definition.revision(),
            pair.pending.definition.revision()
        );
        let terms =
            match_terms(&pair.pending, &counter, 10, now(h).await?).expect("economic terms only");
        assert_eq!(terms.buyer_refund_gold, 400);
        tx.rollback().await?;
        Ok(())
    })
}
#[test]
fn market_native_job_book_skip_locked_and_transaction_release() -> TestResult {
    run("marketlocks", async |h| {
        let o = offer(60, Side::Sell, 40, now(h).await? - 1000);
        seed(h, &o).await?;
        let mut first = h.pool.begin().await?;
        let _book = lock_book(&mut first, o.book()).await?;
        let mut second = h.pool.begin().await?;
        assert!(try_lock_job_book(&mut second, o.book()).await?.is_none());
        second.rollback().await?;
        first.rollback().await?;
        let mut second = h.pool.begin().await?;
        assert!(try_lock_job_book(&mut second, o.book()).await?.is_some());
        second.rollback().await?;
        Ok(())
    })
}
#[test]
fn market_native_expiry_candidates_bound_order_world_and_recheck() -> TestResult {
    run("marketexpiry", async |h| {
        let time = now(h).await? - LIFETIME_US - 10000;
        for n in 1..=101_u8 {
            seed(h, &offer(n, Side::Buy, 40, time + i64::from(n))).await?;
        }
        let mut connection = h.pool.acquire().await?;
        let rows = expired_candidates(&mut connection, world()).await?;
        assert_eq!(rows.len(), 100);
        assert_eq!(rows[0].1, MarketOfferId::new(id(1))?);
        assert_eq!(rows[99].1, MarketOfferId::new(id(100))?);
        assert!(
            expired_candidates(&mut connection, WorldId::from_bytes(id(111))?)
                .await?
                .is_empty()
        );
        drop(connection);
        // A concurrent completed atomic writer may end a discovered row. Selection re-reads it;
        // this fixture does not claim to execute the omitted bank/Inbox operation.
        sqlx::query("UPDATE game_market_offers SET state=4,escrow_gold=0 WHERE offer_id=encode($1,'hex')::uuid").bind(id(1).as_slice()).execute(&h.pool).await?;
        let mut tx = h.pool.begin().await?;
        let mut book = lock_book(&mut tx, rows[0].0.clone()).await?;
        assert!(book.plan_end(rows[0].1, EndReason::Expire).await.is_err());
        assert!(book.plan_end(rows[1].1, EndReason::Expire).await?.is_some());
        tx.rollback().await?;
        Ok(())
    })
}
#[test]
fn market_native_operation_history_binding_immutability_and_occurrence_namespace() -> TestResult {
    run("marketreceipt", async |h| {
        let occurrence = Occurrence::Command(CommandRef::new(
            GameSessionId::decode(&id(50))?,
            CommandId::new(1)?,
        ));
        let actor = CharacterId::from_bytes(id(CHARACTER))?;
        sqlx::query("INSERT INTO game_market_operations VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,3,encode($4,'hex')::uuid,$5,$6)").bind(id(WORLD).as_slice()).bind(occurrence.key()?).bind(id(60).as_slice()).bind(actor.as_bytes().as_slice()).bind([7_u8;32].as_slice()).bind([1_u8,2,3].as_slice()).execute(&h.pool).await?;
        let mut c = h.pool.acquire().await?;
        let first = read_operation(
            &mut c,
            world(),
            occurrence,
            OperationKind::Cancel,
            Some(actor),
            &[7; 32],
        )
        .await
        .map_err(harness::debug)?
        .ok_or("receipt")?;
        assert_eq!(first.transaction_id, id(60));
        assert_eq!(first.outcome, vec![1, 2, 3]);
        assert_eq!(
            read_operation(
                &mut c,
                world(),
                occurrence,
                OperationKind::Cancel,
                Some(actor),
                &[7; 32]
            )
            .await
            .map_err(harness::debug)?,
            Some(first)
        );
        assert!(matches!(
            read_operation(
                &mut c,
                world(),
                occurrence,
                OperationKind::Cancel,
                Some(actor),
                &[8; 32]
            )
            .await,
            Err(OperationReadError::ConflictingBinding)
        ));
        assert!(matches!(
            read_operation(
                &mut c,
                world(),
                occurrence,
                OperationKind::Accept,
                Some(actor),
                &[7; 32]
            )
            .await,
            Err(OperationReadError::ConflictingBinding)
        ));
        assert!(matches!(
            read_operation(
                &mut c,
                WorldId::from_bytes(id(99))?,
                occurrence,
                OperationKind::Cancel,
                Some(actor),
                &[7; 32]
            )
            .await,
            Err(OperationReadError::ConflictingBinding)
        ));
        assert!(matches!(
            read_operation(
                &mut c,
                world(),
                occurrence,
                OperationKind::Cancel,
                Some(CharacterId::from_bytes(id(99))?),
                &[7; 32]
            )
            .await,
            Err(OperationReadError::ConflictingBinding)
        ));
        let job = Occurrence::Job {
            offer: MarketOfferId::new(id(60))?,
            step: JobStep::Expire,
            number: 1,
        };
        assert_ne!(job.key()?, occurrence.key()?);
        assert!(
            Occurrence::Job {
                offer: MarketOfferId::new(id(60))?,
                step: JobStep::Expire,
                number: 0
            }
            .key()
            .is_err()
        );
        let error = sqlx::query("UPDATE game_market_operations SET outcome=$1")
            .bind([4_u8].as_slice())
            .execute(&mut *c)
            .await
            .expect_err("immutable");
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        Ok(())
    })
}

async fn seed_escrow(h: &Harness, o: &Offer, count: i32) -> TestResult {
    // Historical admin item fixtures, following native Inbox tests. Disabled native MINT/
    // placement guards are confined to fixture loading; no such bypass exists in the component.
    // This does NOT qualify Market custody proofs, admitted content or conservation guards.
    let mut tx = h.pool.begin().await?;
    tx.execute("SET LOCAL session_replication_role=replica")
        .await?;
    sqlx::query("WITH items AS(INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$2,$3,1,1,game_character_uuid_v7() FROM generate_series(1,$4) RETURNING item_instance_id,world_id) INSERT INTO game_item_market_escrow_locations SELECT item_instance_id,world_id,encode($5,'hex')::uuid,row_number() OVER(ORDER BY item_instance_id) FROM items")
        .bind(o.world.as_bytes().as_slice()).bind(o.definition.key().as_str()).bind(o.definition.revision().as_str()).bind(count).bind(o.id.as_bytes().as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
#[test]
fn market_native_escrow_reads_real_items_bounded_and_offer_bound() -> TestResult {
    run("marketescrow", async |h| {
        let mut o = offer(60, Side::Sell, 40, now(h).await? - 1000);
        o.amount = 100;
        o.remaining = 100;
        seed(h, &o).await?;
        seed_escrow(h, &o, 100).await?;
        let mut tx = h.pool.begin().await?;
        let mut book = lock_book(&mut tx, o.book()).await?;
        let rows = book.escrow(&o).await?;
        assert_eq!(rows.len(), 100);
        assert_eq!(rows.iter().map(|r| u64::from(r.quantity)).sum::<u64>(), 100);
        assert!(rows.windows(2).all(|w| w[0].item < w[1].item));
        assert!(rows.iter().all(|r| r.ordinal > 0));
        let mut substituted = o.clone();
        substituted.remaining = 99;
        assert!(book.escrow(&substituted).await.is_err());
        tx.rollback().await?;
        sqlx::query("UPDATE game_market_offers SET amount=101,remaining=101")
            .execute(&h.pool)
            .await?;
        let extra = offer(61, Side::Sell, 40, now(h).await? - 1000);
        seed(h, &extra).await?;
        seed_escrow(h, &extra, 1).await?;
        sqlx::query("UPDATE game_item_market_escrow_locations SET offer_id=encode($1,'hex')::uuid,ordinal=101 WHERE offer_id=encode($2,'hex')::uuid").bind(o.id.as_bytes().as_slice()).bind(extra.id.as_bytes().as_slice()).execute(&h.pool).await?;
        let mut tx = h.pool.begin().await?;
        let mut book = lock_book(&mut tx, o.book()).await?;
        let actual = book.offer(o.id).await?.ok_or("offer")?;
        assert!(book.escrow(&actual).await.is_err());
        tx.rollback().await?;
        Ok(())
    })
}
#[test]
fn market_native_schema_rejects_invalid_gold_custody_and_ended_matching() -> TestResult {
    run("marketguards", async |h| {
        let o = offer(60, Side::Buy, 40, now(h).await? - 1000);
        seed(h, &o).await?;
        for query in [
            "UPDATE game_market_offers SET escrow_gold=999",
            "UPDATE game_market_offers SET state=5,held_credit_gold=1000,ended_as=3",
            "UPDATE game_market_offers SET state=2,escrow_gold=0",
            "UPDATE game_market_offers SET state=3,escrow_gold=0,matching_pending=true",
        ] {
            let mut tx = h.pool.begin().await?;
            let result = sqlx::query(query).execute(&mut *tx).await;
            assert_eq!(
                result
                    .expect_err("guard")
                    .as_database_error()
                    .and_then(|e| e.code())
                    .as_deref(),
                Some("23514")
            );
            tx.rollback().await?;
        }
        Ok(())
    })
}
