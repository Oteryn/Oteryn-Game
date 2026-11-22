use super::*;

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
        first.definition = definition(first.definition.key(), "r2")?;
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
            seed(
                h,
                &offer(
                    n,
                    Side::Buy,
                    if n <= 100 { 40 } else { 70 },
                    time + i64::from(n),
                ),
            )
            .await?;
        }
        let mut discovery = h.pool.begin().await?;
        let rows = lock_job_candidates(&mut discovery, crate::foundation::WorldId::decode(&id(WORLD))?).await?;
        assert_eq!(rows.len(), 100);
        assert_eq!(rows[0], MarketOfferId::new(id(1))?);
        assert_eq!(rows[99], MarketOfferId::new(id(100))?);
        assert!(
            lock_job_candidates(&mut discovery, crate::foundation::WorldId::decode(&id(111))?)
                .await?
                .is_empty()
        );
        discovery.rollback().await?;
        // A concurrent completed atomic writer may end a discovered row. Selection re-reads it;
        // this fixture does not claim to execute the omitted bank/Inbox operation.
        sqlx::query("UPDATE game_market_offers SET state=4,escrow_gold=0,ended_at_us=expires_at_us,end_sequence=nextval('game_market_end_sequence') WHERE offer_id=encode($1,'hex')::uuid").bind(id(1).as_slice()).execute(&h.pool).await?;
        let mut tx = h.pool.begin().await?;
        let key = offer(1, Side::Buy, 40, time).book();
        let mut book = lock_book(&mut tx, key).await?;
        assert!(book.plan_end(rows[0], EndReason::Expire).await.is_err());
        assert!(book.plan_end(rows[1], EndReason::Expire).await?.is_some());
        tx.rollback().await?;
        Ok(())
    })
}

#[test]
fn market_expiry_postgres_drains_unlocked_book_behind_locked_candidate_window() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_expiry_case(admin, "market_expiry_window", async |h| {
        let time = now(h).await? - LIFETIME_US - 10_000;
        let first = offer(1, Side::Buy, 40, time);
        for n in 1..=100 {
            seed(h, &offer(n, Side::Buy, 40, time + i64::from(n))).await?;
        }
        let mut later = offer(101, Side::Buy, 70, time + 101);
        later.definition = definition("oteryn:content.item.bowl", "r1")?;
        seed(h, &later).await?;

        let mut blocker = h.pool.begin().await?;
        let _book = lock_book(&mut blocker, first.book()).await?;
        assert!(advance_market(h, &h.root).await?, "the later book should make progress");
        assert_eq!(expiry_state(h, &later).await?, (4, 1_000, 1));
        assert_eq!(expiry_state(h, &first).await?, (1, 0, 0));
        assert!(!advance_market(h, &h.root).await?);
        blocker.rollback().await?;
        Ok(())
    })
}

#[test]
fn market_expiry_postgres_rechecks_offer_and_refunds_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_expiry_case(admin, "market_expiry_recheck", async |h| {
        let time = now(h).await?;
        let mut due = offer(60, Side::Buy, 40, time - LIFETIME_US - 1);
        due.matching_pending = true;
        let live = offer(61, Side::Buy, 70, time - 1_000);
        let taken = offer(62, Side::Buy, 80, time - LIFETIME_US - 2);
        seed(h, &due).await?;
        seed(h, &live).await?;
        seed(h, &taken).await?;

        let runtime_url = format!("{}?options=-c%20role%3Doteryn_game_runtime", h.database.url);
        let runtime = crate::durability::DurabilityRoot::connect_test_runtime(&runtime_url)?;
        let mut blocker = h.pool.begin().await?;
        let _book = lock_book(&mut blocker, due.book()).await?;
        assert!(!advance_market(h, &runtime).await?);
        assert_eq!(expiry_state(h, &due).await?, (1, 0, 0));

        // The lock holder ends the oldest candidate using the native atomic writer.
        sqlx::query(
            "SELECT game_market_write(encode($1,'hex')::uuid,NULL,NULL,NULL,NULL,NULL,
                game_market_identity(encode($2,'hex')::uuid,'expire'),4::smallint,
                encode($2,'hex')::uuid,NULL,NULL,NULL,NULL,NULL,NULL,NULL,sha256($2),NULL)",
        )
        .bind(taken.world.as_bytes().as_slice())
        .bind(taken.id.as_bytes().as_slice())
        .execute(&mut *blocker)
        .await?;
        blocker.commit().await?;
        assert_eq!(expiry_state(h, &taken).await?, (4, 1_000, 1));

        assert!(advance_market(h, &runtime).await?);
        assert_eq!(expiry_state(h, &due).await?, (4, 1_000, 1));
        assert!(!advance_market(h, &runtime).await?);
        assert_eq!(expiry_state(h, &due).await?, (4, 1_000, 1));
        assert_eq!(expiry_state(h, &taken).await?, (4, 1_000, 1));
        assert_eq!(expiry_state(h, &live).await?, (1, 0, 0));
        Ok(())
    })
}

fn run_expiry_case<F>(admin: String, tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult + Send + 'static,
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            harness::runtime()
                .and_then(|runtime| {
                    runtime.block_on(async {
                        let h = Harness::create(admin, tag, false).await?;
                        let result = body(&h).await;
                        h.cleanup().await?;
                        result
                    })
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Market expiry test thread failed")?
        .map_err(Into::into)
}

async fn advance_market(h: &Harness, root: &crate::durability::DurabilityRoot) -> TestResult<bool> {
    root.await_ready().await?;
    let seal = h.recovery.seal_current().map_err(harness::debug)?;
    let authority = root.open_character_authority(&seal).await.map_err(harness::debug)?;
    root.await_ready().await?;
    let world = crate::foundation::WorldId::decode(&id(WORLD))?;
    Ok(root.advance_market(&authority, &h.node, world).await?)
}

async fn expiry_state(h: &Harness, offer: &Offer) -> TestResult<(i16, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT o.state, COALESCE(b.balance, 0)::bigint,
            (SELECT count(*) FROM game_account_bank_entries e
             JOIN game_market_settlements s ON s.transaction_id=e.market_transaction_id
             WHERE s.offer_id=o.offer_id AND e.market_kind='MARKET_ESCROW_RETURN')
         FROM game_market_offers o LEFT JOIN game_account_bank_balances b USING(account_id,world_id)
         WHERE o.offer_id=encode($1,'hex')::uuid",
    )
    .bind(offer.id.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?)
}
