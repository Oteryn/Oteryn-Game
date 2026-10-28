//! LOYALTY3 step 1: Platform's absolute Premium-day projection is account-wide and monotonic.
use super::fixture::{Harness, debug, id, run};
use super::TestResult;
use sqlx::Executor;

#[test]
fn loyalty_platform_premium_day_writer_is_idempotent_and_account_wide() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "loyalty_writer", true, false).await?;
        let account = id(40);

        let mut control = h.pool.acquire().await?;
        control.execute("SET ROLE oteryn_game_control").await?;
        let earned: i64 = sqlx::query_scalar(
            "SELECT game_account_loyalty_project_premium_days(encode($1,'hex')::uuid,$2,$3)",
        )
        .bind(account.as_slice())
        .bind(72_i64)
        .bind(1_i64)
        .fetch_one(&mut *control)
        .await?;
        assert_eq!(earned, 360);
        // A retry of the same Platform fact is harmless.
        let retry: i64 = sqlx::query_scalar(
            "SELECT game_account_loyalty_project_premium_days(encode($1,'hex')::uuid,$2,$3)",
        )
        .bind(account.as_slice())
        .bind(72_i64)
        .bind(1_i64)
        .fetch_one(&mut *control)
        .await?;
        assert_eq!(retry, 360);
        // A later absolute account total adds only the newly spent day, irrespective of Character.
        let advanced: i64 = sqlx::query_scalar(
            "SELECT game_account_loyalty_project_premium_days(encode($1,'hex')::uuid,$2,$3)",
        )
        .bind(account.as_slice())
        .bind(73_i64)
        .bind(2_i64)
        .fetch_one(&mut *control)
        .await?;
        assert_eq!(advanced, 365);
        let conflict = sqlx::query_scalar::<_, i64>(
            "SELECT game_account_loyalty_project_premium_days(encode($1,'hex')::uuid,$2,$3)",
        )
        .bind(account.as_slice())
        .bind(74_i64)
        .bind(2_i64)
        .fetch_one(&mut *control)
        .await;
        assert!(conflict.is_err());
        drop(control);

        // The production runtime credential can consume the projection but cannot forge it.
        let mut runtime = h.pool.acquire().await?;
        runtime.execute("SET ROLE oteryn_game_runtime").await?;
        let stored: (i64, i64) = sqlx::query_as(
            "SELECT loyalty_points,platform_revision FROM game_account_loyalty_projections WHERE account_id=encode($1,'hex')::uuid",
        )
        .bind(account.as_slice())
        .fetch_one(&mut *runtime)
        .await?;
        assert_eq!(stored, (365, 2));
        let forged = sqlx::query(
            "SELECT game_account_loyalty_project_premium_days(encode($1,'hex')::uuid,100,3)",
        )
        .bind(account.as_slice())
        .execute(&mut *runtime)
        .await;
        assert!(forged.is_err());
        drop(runtime);

        eprintln!("LOYALTY3 Platform writer, retry, monotonic total and runtime privilege verified");
        h.cleanup().await.map_err(debug)
    })
}
