//! The authenticated capacity probe must defer when PostgreSQL cannot give it a read lock.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::{DurabilityError, SqlxFailureClass};
use std::time::Duration;

#[test]
fn channel_capacity_defers_on_a_contended_relation() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime().map_err(fixture::debug)?.block_on(async {
                let harness = fixture::Harness::create(admin, "capacity_holder", false).await?;
                let result = probe_under_lock(&harness).await;
                harness.cleanup().await?;
                result
            }).map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "channel capacity test thread failed")??;
    Ok(())
}

async fn probe_under_lock(harness: &fixture::Harness) -> fixture::TestResult {
    let scope = fixture::scope()?;
    let character = fixture::id(fixture::CHARACTER);
    let world = *scope.world_id().as_bytes();
    let channel = *scope.channel_id().ok_or("fixture channel")?.as_bytes();
    let mut blocker = harness.pool.begin().await?;
    sqlx::query("LOCK TABLE game_channel_pvp_types IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await?;
    let result = tokio::time::timeout(
        Duration::from_millis(500),
        harness.root.read_channel_full(character, world, channel),
    )
    .await;
    blocker.rollback().await?;
    let result = result.map_err(|_| "capacity held the durability holder beyond 500ms")?;
    assert!(matches!(
        result,
        Err(DurabilityError::Database(SqlxFailureClass::RetryableStatement))
    ), "{result:?}");
    assert!(!harness.root.read_channel_full(character, world, channel).await?);
    Ok(())
}
