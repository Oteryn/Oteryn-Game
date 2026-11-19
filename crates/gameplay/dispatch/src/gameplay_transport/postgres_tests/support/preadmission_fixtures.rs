use super::TestResult;

// The existing verifier host-clock hook, the fake Platform's timestamps, and
// the commit query all use one fixed instant in this disposable test database.
pub(super) struct FrozenAdmissionClock(Option<i64>);

impl FrozenAdmissionClock {
    pub(super) async fn install(h: &super::fixture::Harness, now: i64) -> TestResult<Self> {
        sqlx::query("CREATE SCHEMA preadm_clock")
            .execute(&h.pool)
            .await?;
        sqlx::query("CREATE TABLE preadm_clock.instant(singleton BOOLEAN PRIMARY KEY CHECK(singleton), value BIGINT NOT NULL)").execute(&h.pool).await?;
        sqlx::query("INSERT INTO preadm_clock.instant VALUES(true,$1)")
            .bind(now)
            .execute(&h.pool)
            .await?;
        sqlx::query("CREATE FUNCTION preadm_clock.clock_timestamp() RETURNS timestamptz LANGUAGE sql STABLE AS 'SELECT pg_catalog.to_timestamp(value::double precision) FROM preadm_clock.instant WHERE singleton'").execute(&h.pool).await?;
        sqlx::query("CREATE FUNCTION preadm_clock.statement_timestamp() RETURNS timestamptz LANGUAGE sql STABLE AS 'SELECT pg_catalog.to_timestamp(value::double precision) FROM preadm_clock.instant WHERE singleton'").execute(&h.pool).await?;
        sqlx::query("GRANT USAGE ON SCHEMA preadm_clock TO oteryn_game_runtime")
            .execute(&h.pool)
            .await?;
        sqlx::query("GRANT SELECT ON preadm_clock.instant TO oteryn_game_runtime")
            .execute(&h.pool)
            .await?;
        let mut connection = h.root.try_acquire_ready()?;
        sqlx::query("SET search_path=preadm_clock,pg_catalog,public")
            .execute(&mut *connection)
            .await?;
        let returned = connection
            .return_to_pool_observed_until(
                std::time::Instant::now() + std::time::Duration::from_secs(5),
            )
            .await;
        assert!(matches!(
            returned,
            sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle
        ));
        Ok(Self(super::HOST_NOW.replace(Some(now))))
    }
}

impl Drop for FrozenAdmissionClock {
    fn drop(&mut self) {
        super::HOST_NOW.set(self.0);
    }
}
