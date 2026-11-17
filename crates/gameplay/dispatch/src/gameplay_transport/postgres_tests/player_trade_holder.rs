//! A busy real holder must not become a permanent trade admission refusal.
use super::TestResult;
use crate::bestiary_postgres_harness::{self as fixture, Harness, debug};
use crate::gameplay_transport::player_trade::{CheckError, preflight::read_retry};
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn player_trade_postgres_busy_holder_read_recovers_without_changing_refusals() -> TestResult {
    let Some(admin) = fixture::configured_admin() else { return Ok(()); };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
        fixture::runtime().map_err(debug)?.block_on(async move {
            let h = Harness::create(admin, "trade_holder_read", false).await.map_err(debug)?;
            let result = exercise(&h).await.map_err(debug);
            h.cleanup().await.map_err(debug)?;
            result
        })
    })?.join().map_err(|_| "trade holder test thread failed")??;
    Ok(())
}

async fn exercise(h: &Harness) -> TestResult {
    let started = std::time::Instant::now();
    let mut held = h.root.try_acquire_ready()?;
    let attempts = AtomicUsize::new(0);
    let read = read_retry(&h.root, async || {
        attempts.fetch_add(1, Ordering::SeqCst);
        h.root.try_acquire_ready().map_err(CheckError::from)
    });
    let release = async {
        while attempts.load(Ordering::SeqCst) == 0 { tokio::task::yield_now().await; }
        held.return_to_pool_observed_until(std::time::Instant::now() + std::time::Duration::from_secs(1)).await
    };
    let (result, returned) = super::fixture::join_two(read, release).await;
    assert!(matches!(returned, sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle));
    assert!(result.is_ok(), "temporary holder contention must not reject trade: {:?}", result.as_ref().err());
    let mut ready = result.map_err(debug)?;
    assert_eq!(sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&mut *ready).await?, 1);
    assert!(attempts.load(Ordering::SeqCst) >= 2);
    let exhausted = read_retry(&h.root, async || {
        h.root.try_acquire_ready().map(drop).map_err(CheckError::from)
    }).await;
    assert_eq!(exhausted, Err(oteryn_protocol_oteryn::player_trade::ResultCode::Rejected));
    drop(ready);
    let refused = read_retry::<()>(&h.root, async || Err(CheckError::Refused(oteryn_protocol_oteryn::player_trade::ResultCode::Stale))).await;
    assert_eq!(refused, Err(oteryn_protocol_oteryn::player_trade::ResultCode::Stale));
    eprintln!("POSTGRES_CASE_COMPLETED trade_busy_holder_read duration={:?} attempts={}", started.elapsed(), attempts.load(Ordering::SeqCst));
    Ok(())
}
