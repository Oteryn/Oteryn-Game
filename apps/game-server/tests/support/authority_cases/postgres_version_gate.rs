use crate::bestiary_postgres_harness::{TestResult, configured_admin, runtime};
use sqlx::{Connection, PgConnection};
use std::panic::{AssertUnwindSafe, catch_unwind};

async fn cluster_catalog(admin: &mut PgConnection) -> TestResult<(Vec<String>, Vec<String>)> {
    Ok(sqlx::query_as(
        "SELECT ARRAY(SELECT datname::text FROM pg_database ORDER BY datname), \
                ARRAY(SELECT rolname::text FROM pg_roles ORDER BY rolname)",
    )
    .fetch_one(admin)
    .await?)
}

// Run this case on its own against a disposable PostgreSQL server whose version differs
// from the canonical test server.
#[test]
fn authority_postgres_rejects_wrong_version_before_cluster_changes() -> TestResult {
    let Some(admin_url) = configured_admin() else {
        return Ok(());
    };
    let runtime = runtime()?;
    let (mut admin, version) = runtime.block_on(async {
        let mut admin = PgConnection::connect(&admin_url).await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&mut admin)
            .await?;
        Ok::<_, sqlx::Error>((admin, version))
    })?;
    if version == oteryn_gameplay_durability::CANONICAL_POSTGRES_SERVER_VERSION_NUM {
        return Ok(());
    }

    let before = runtime.block_on(cluster_catalog(&mut admin))?;
    let refusal = catch_unwind(AssertUnwindSafe(|| {
        runtime.block_on(crate::Database::create(admin_url, "version_gate"))
    }));
    let after = runtime.block_on(cluster_catalog(&mut admin))?;
    assert_eq!(before, after, "version mismatch changed databases or roles");
    assert!(
        matches!(refusal, Err(panic) if panic.downcast_ref::<String>().is_some_and(|message| {
            message.contains("canonical target requires PostgreSQL 18.6")
        })),
        "fixture did not reject the server version"
    );
    Ok(())
}
