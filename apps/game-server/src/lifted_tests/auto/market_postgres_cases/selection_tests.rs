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
        sqlx::query("UPDATE game_market_offers SET state=4,escrow_gold=0,ended_at_us=expires_at_us,end_sequence=nextval('game_market_end_sequence') WHERE offer_id=encode($1,'hex')::uuid").bind(id(1).as_slice()).execute(&h.pool).await?;
        let mut tx = h.pool.begin().await?;
        let mut book = lock_book(&mut tx, rows[0].0.clone()).await?;
        assert!(book.plan_end(rows[0].1, EndReason::Expire).await.is_err());
        assert!(book.plan_end(rows[1].1, EndReason::Expire).await?.is_some());
        tx.rollback().await?;
        Ok(())
    })
}
