//! LOYALTY: Platform's absolute Premium-day projection is 1:1, revisioned and control-only.
use super::TestResult;
use super::fixture::{Harness, debug, id, run};
use sqlx::Executor;

const PROJECT: &str =
    "SELECT game_account_loyalty_project_premium_days(encode($1,'hex')::uuid,$2,$3)";

fn sqlstate(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|e| e.code())
        .map(|c| c.into_owned())
}

#[test]
fn loyalty_premium_days_project_one_point_per_day_idempotently_for_control_only() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "loyalty_premium_days", true, false).await?;
        let account: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;

        let mut control = h.pool.acquire().await?;
        control.execute("SET ROLE oteryn_game_control").await?;
        let points: i64 = sqlx::query_scalar(PROJECT)
            .bind(account.as_slice())
            .bind(72_i64)
            .bind(1_i64)
            .fetch_one(&mut *control)
            .await?;
        assert_eq!(points, 72);

        let stored: (i64, i64) = sqlx::query_as(
            "SELECT loyalty_points, platform_revision FROM game_account_loyalty_projections WHERE account_id=encode($1,'hex')::uuid",
        )
        .bind(account.as_slice())
        .fetch_one(&mut *control)
        .await?;
        assert_eq!(stored, (72, 1));

        let retry: i64 = sqlx::query_scalar(PROJECT)
            .bind(account.as_slice())
            .bind(72_i64)
            .bind(1_i64)
            .fetch_one(&mut *control)
            .await?;
        assert_eq!(retry, 72);

        let advanced: i64 = sqlx::query_scalar(PROJECT)
            .bind(account.as_slice())
            .bind(73_i64)
            .bind(2_i64)
            .fetch_one(&mut *control)
            .await?;
        assert_eq!(advanced, 73);

        let conflict = sqlx::query_scalar::<_, i64>(PROJECT)
            .bind(account.as_slice())
            .bind(74_i64)
            .bind(2_i64)
            .fetch_one(&mut *control)
            .await
            .expect_err("same revision with different points must conflict");
        assert_eq!(sqlstate(&conflict).as_deref(), Some("23000"));

        let stale = sqlx::query_scalar::<_, i64>(PROJECT)
            .bind(account.as_slice())
            .bind(80_i64)
            .bind(1_i64)
            .fetch_one(&mut *control)
            .await
            .expect_err("stale revision must be rejected");
        assert_eq!(sqlstate(&stale).as_deref(), Some("40001"));

        let corrected: i64 = sqlx::query_scalar(PROJECT)
            .bind(account.as_slice())
            .bind(70_i64)
            .bind(3_i64)
            .fetch_one(&mut *control)
            .await?;
        assert_eq!(corrected, 70);

        let stored: (i64, i64) = sqlx::query_as(
            "SELECT loyalty_points, platform_revision FROM game_account_loyalty_projections WHERE account_id=encode($1,'hex')::uuid",
        )
        .bind(account.as_slice())
        .fetch_one(&mut *control)
        .await?;
        assert_eq!(stored, (70, 3));

        let negative = sqlx::query_scalar::<_, i64>(PROJECT)
            .bind(account.as_slice())
            .bind(-1_i64)
            .bind(4_i64)
            .fetch_one(&mut *control)
            .await
            .expect_err("negative spent Premium-day total must be rejected");
        assert_eq!(sqlstate(&negative).as_deref(), Some("22003"));

        let overflow = sqlx::query_scalar::<_, i64>(PROJECT)
            .bind(account.as_slice())
            .bind(2_147_483_648_i64)
            .bind(4_i64)
            .fetch_one(&mut *control)
            .await
            .expect_err("spent Premium-day total above the projection range must be rejected");
        assert_eq!(sqlstate(&overflow).as_deref(), Some("22003"));

        let stored: (i64, i64) = sqlx::query_as(
            "SELECT loyalty_points, platform_revision FROM game_account_loyalty_projections WHERE account_id=encode($1,'hex')::uuid",
        )
        .bind(account.as_slice())
        .fetch_one(&mut *control)
        .await?;
        assert_eq!(stored, (70, 3));
        control.detach();

        let mut runtime = h.pool.acquire().await?;
        runtime.execute("SET ROLE oteryn_game_runtime").await?;
        let denied = sqlx::query_scalar::<_, i64>(PROJECT)
            .bind(account.as_slice())
            .bind(100_i64)
            .bind(9_i64)
            .fetch_one(&mut *runtime)
            .await
            .expect_err("runtime role must not execute the projection writer");
        assert_eq!(sqlstate(&denied).as_deref(), Some("42501"));
        runtime.detach();
        h.cleanup().await.map_err(debug)
    })
}
