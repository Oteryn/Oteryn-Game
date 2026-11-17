//! Readiness publication against the actual durable runtime guard.
use super::*;

#[test]
fn readiness_publish_boot_database_errors_are_bounded()
-> crate::bestiary_postgres_harness::TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| readiness_retry_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "readiness retry scenario panicked")?
        .map_err(Into::into)
}

fn readiness_retry_scenario() -> crate::bestiary_postgres_harness::TestResult {
    use crate::bestiary_postgres_harness::{self as fixture, Harness};
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async {
        let h = Harness::create(admin, "readiness_retry_budget", true).await?;
        let mut config = NodeConfig::parse(super::super::config::tests::NODE.as_bytes())?;
        config.readiness.source_authority = "game-runtime-publisher".into();
        let readiness = Readiness {
            root: &h.root, proof: &h.node, scope: fixture::fence(1)?.runtime_scope,
            generation: 1, config: &config,
        };
        readiness.prepare(true, None).await.map_err(|error| format!("{error:?}"))?;
        sqlx::raw_sql("CREATE SEQUENCE readiness_failed_attempts; GRANT USAGE ON SEQUENCE readiness_failed_attempts TO PUBLIC; CREATE FUNCTION fail_readiness_publication() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('readiness_failed_attempts'); RAISE EXCEPTION 'injected readiness publication outage'; END $$; CREATE TRIGGER fail_readiness_publication BEFORE INSERT ON game_durability_admission_guard_history FOR EACH ROW EXECUTE FUNCTION fail_readiness_publication()")
            .execute(&h.pool).await?;
        let result = tokio::time::timeout(Duration::from_secs(8), readiness.publish(true, None)).await;
        let attempted: bool = sqlx::query_scalar("SELECT is_called FROM readiness_failed_attempts").fetch_one(&h.pool).await?;
        assert!(attempted, "the production publication reached the injected DB error: {result:?}");
        h.cleanup().await?;
        assert!(result.is_ok(), "readiness publication ignored the boot budget");
        assert!(matches!(result, Ok(Err(BootError::Readiness(_)))));
        Ok(())
    })
}
