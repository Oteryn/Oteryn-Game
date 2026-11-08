//! Yalahar mechanism component: exact UID, source threshold and read-only progress.
use super::*;

#[test]
fn questpc_yalahar_mechanism_read_only_threshold_and_later_progress_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::super::kill_reward_e2e::scenario_thread::run("yalahar-mechanism-use", || {
        let started = std::time::Instant::now();
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
            let url = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
            let pool = sqlx::PgPool::connect(&url).await?;
            let version: String = sqlx::query_scalar("SHOW server_version").fetch_one(&pool).await?;
            println!("POSTGRES_CASE_STARTED questpc-yalahar-mechanism server_version={version} connection_duration={:?}", started.elapsed());
            pool.close().await;
            Ok::<(), Box<dyn std::error::Error>>(())
        })?;
        let mut gate = callback("in_service_of_yalahar/mechanism-3091")?;
        assert_eq!(gate.selector.unique, Some(3091));
        assert_eq!(gate.storage[0].minimum, 21);
        assert!(gate.is_read_only());
        // Component scene only. The separate native-map test proves this exact
        // source object's original UID; production selectors stay UID-only.
        gate.selector.position = Some(position((32743, 31164, -5)));
        for stage in [None, Some(20), Some(21), Some(72)] {
            exercise(gate.clone(), 8618, 1, stage)?;
        }
        Ok(())
    })
}
