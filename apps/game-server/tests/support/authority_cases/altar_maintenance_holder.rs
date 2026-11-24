//! A contended altar scan must return the single holder to admission.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::spell_owner_commit::SpellLane;
use crate::durability::{DurabilityError, DurabilityRoot, SqlxFailureClass};
use crate::foundation::ScopeOwnershipGeneration;
use std::time::Duration;

#[test]
fn scheduled_altar_scan_yields_on_database_lock_contention() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || exercise(admin, false).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "scheduled altar scan test thread failed")??;
    Ok(())
}

#[test]
fn empty_scheduled_altar_scan_skips_the_admission_relation_fence() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || exercise(admin, true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "empty altar scan test thread failed")??;
    Ok(())
}

fn exercise(admin: String, lock_admission: bool) -> fixture::TestResult {
    fixture::runtime()?.block_on(async {
        let harness = fixture::Harness::create(admin, "altar_holder", false).await?;
        let result = contention(&harness, lock_admission).await;
        harness.cleanup().await?;
        result
    })
}

async fn contention(harness: &fixture::Harness, lock_admission: bool) -> fixture::TestResult {
    let root = &harness.root;
    let seal = harness.recovery.seal_current().map_err(fixture::debug)?;
    let authority = root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let scope = fixture::scope()?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?;
    let permit = SpellLane::for_scope(scope)
        .acquire()
        .await
        .map_err(|_| "unexpected parked spell operation")?;
    let mut blocker = harness.pool.begin().await?;
    let lock = if lock_admission {
        "LOCK TABLE game_durability_admission_account_guards IN ACCESS EXCLUSIVE MODE"
    } else {
        "LOCK TABLE game_elemental_boss_latches IN ACCESS EXCLUSIVE MODE"
    };
    sqlx::query(lock)
        .execute(&mut *blocker)
        .await?;

    let outcome = tokio::time::timeout(
        Duration::from_millis(500),
        DurabilityRoot::with_background_holder(root.read_pending_altar_bosses(
            &permit,
            &authority,
            &harness.node,
            scope,
            generation,
            0,
            [1; 32],
            "map-1",
        )),
    )
    .await;
    blocker.rollback().await?;
    let outcome = outcome.map_err(|_| "scheduled altar scan held the shared holder for 500ms")?;
    if lock_admission {
        assert!(outcome.map_err(fixture::debug)?.is_empty());
    } else if !matches!(
        outcome,
        Err(DurabilityError::Database(
            SqlxFailureClass::RetryableStatement
        ))
    ) {
        return Err(format!("expected PostgreSQL's bounded lock refusal, got {outcome:?}").into());
    }
    let _ = root
        .reconcile_character_bootstrap(&authority, fixture::id(49))
        .await
        .map_err(fixture::debug)?;
    Ok(())
}
