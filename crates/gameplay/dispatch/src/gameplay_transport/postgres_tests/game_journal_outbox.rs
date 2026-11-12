//! Fixed private export bytes, contiguous revisions, exact retry and stale fence.
use super::TestResult;
use super::fixture::{Harness, run};
#[test]
fn game_journal_outbox_fixed_bytes_ack_and_generation_fence() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::journal_watchdog::run("journal-outbox", scenario)
}
fn scenario(stage: super::journal_watchdog::Stage) -> TestResult {
    run(async move |admin| {
        let h = Harness::create(admin, "journal_outbox", false).await?;
        stage.set("enqueue/ACK/generation fences");
        let stream=br#"{"contract_version":1,"operation":"ReadGameJournalCursorV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":null,"character_revision":null,"from_projection_revision":null,"to_projection_revision":null,"projection_epoch":null,"family":"BANK","recovery_generation":"1"}"#.as_slice();
        let record=br#"{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":null,"character_revision":null,"from_projection_revision":null,"to_projection_revision":null,"projection_epoch":null,"family":"BANK","recovery_generation":"1","source_revision":"1","record_id":"019a0000-0000-7000-8000-000000000007","occurred_at":"1793836800","coverage_start_at":"1793836800","kind":"BANK_ENTRY","payload":{"operation_occurrence_id":"019a0000-0000-7000-8000-000000000008","event_id":"019a0000-0000-7000-8000-000000000009","entry_id":"019a0000-0000-7000-8000-00000000000a","previous_entry_id":null,"kind":"DEPOSIT","direction":"CREDIT","amount":"100","balance_before":"0","balance_after":"100"}}"#.as_slice();
        sqlx::query("INSERT INTO game_journal_streams(stream_key,source_authority,account_id,recovery_generation) VALUES($1,'OTERYN_GAME_JOURNAL','019a0000-0000-7000-8000-000000000001',1)").bind(stream).execute(&h.pool).await?;
        // Fix this storage scenario's source time once; the codec goldens keep
        // their literal timestamps, independent of the retention clock.
        let now: String =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint::text")
                .fetch_one(&h.pool)
                .await?;
        let fixed = std::str::from_utf8(record)?.replace("1793836800", &now);
        let record = fixed.as_bytes();
        let insert = || {
            sqlx::query_scalar::<_, bool>("SELECT game_journal_enqueue($1,$2)")
                .bind(stream)
                .bind(record)
        };
        let mut runtime = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *runtime)
            .await?;
        assert!(insert().fetch_one(&mut *runtime).await?);
        runtime.commit().await?;
        assert!(!insert().fetch_one(&h.pool).await?);
        let (bytes,digest_ok):(Vec<u8>,bool)=sqlx::query_as("SELECT export_bytes,export_sha256=sha256(export_bytes) FROM game_journal_outbox WHERE stream_key=$1 AND source_revision=1").bind(stream).fetch_one(&h.pool).await?;
        assert_eq!(bytes, record);
        assert!(digest_ok);
        let changed =
            std::str::from_utf8(record)?.replace("\"amount\":\"100\"", "\"amount\":\"101\"");
        assert!(
            sqlx::query("SELECT game_journal_enqueue($1,$2)")
                .bind(stream)
                .bind(changed.as_bytes())
                .execute(&h.pool)
                .await
                .is_err()
        );
        let gap = std::str::from_utf8(record)?
            .replace("\"source_revision\":\"1\"", "\"source_revision\":\"3\"")
            .replace(
                "019a0000-0000-7000-8000-000000000007",
                "019a0000-0000-7000-8000-000000000017",
            );
        let Err(error) = sqlx::query("SELECT game_journal_enqueue($1,$2)")
            .bind(stream)
            .bind(gap.as_bytes())
            .execute(&h.pool)
            .await
        else {
            return Err("revision gap must fail".into());
        };
        let database = error.as_database_error().ok_or("database rejection")?;
        assert_eq!(database.code().as_deref(), Some("23514"));
        assert!(database.message().contains("noncontiguous revision"));
        let cursor:(String,String,i64)=sqlx::query_as("SELECT next_revision::text,acknowledged_revision::text,(SELECT count(*) FROM game_journal_outbox WHERE stream_key=$1) FROM game_journal_streams WHERE stream_key=$1").bind(stream).fetch_one(&h.pool).await?;
        assert_eq!(cursor, ("2".into(), "0".into(), 1));
        assert!(
            sqlx::query_scalar::<_, bool>(
                "SELECT game_journal_ack($1,1,'019a0000-0000-7000-8000-000000000007')"
            )
            .bind(stream)
            .fetch_one(&h.pool)
            .await?
        );
        assert!(
            !sqlx::query_scalar::<_, bool>(
                "SELECT game_journal_ack($1,1,'019a0000-0000-7000-8000-000000000007')"
            )
            .bind(stream)
            .fetch_one(&h.pool)
            .await?
        );
        let next = std::str::from_utf8(stream)?.replace(
            "\"recovery_generation\":\"1\"",
            "\"recovery_generation\":\"2\"",
        );
        sqlx::query("INSERT INTO game_journal_streams(stream_key,source_authority,account_id,recovery_generation) VALUES($1,'OTERYN_GAME_JOURNAL','019a0000-0000-7000-8000-000000000001',2)").bind(next.as_bytes()).execute(&h.pool).await?;
        assert!(insert().fetch_one(&h.pool).await.is_err());
        assert!(
            sqlx::query("SELECT game_journal_ack($1,1,'019a0000-0000-7000-8000-000000000007')")
                .bind(stream)
                .execute(&h.pool)
                .await
                .is_err()
        );
        stage.set("unresolved expiry and privacy fences");
        sqlx::query("UPDATE game_journal_export_settings SET game_journal_items_export_enabled=true,source_authority='OTERYN_GAME_JOURNAL' WHERE singleton").execute(&h.pool).await?;
        let second = std::str::from_utf8(record)?.replace(
            "\"recovery_generation\":\"1\"",
            "\"recovery_generation\":\"2\"",
        );
        sqlx::query("SELECT game_journal_enqueue($1,$2)")
            .bind(next.as_bytes())
            .bind(second.as_bytes())
            .execute(&h.pool)
            .await?;
        sqlx::query("UPDATE game_journal_outbox SET retain_until=clock_timestamp()-interval '1 second' WHERE stream_key=$1").bind(next.as_bytes()).execute(&h.pool).await?;
        let mut runtime = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *runtime)
            .await?;
        sqlx::query("SELECT game_journal_purge()")
            .execute(&mut *runtime)
            .await?;
        runtime.commit().await?;
        let evidence:(bool,bool,bool,String)=sqlx::query_as("SELECT o.export_bytes IS NULL,o.acknowledged_at IS NULL,o.export_sha256=sha256($2),s.retained_floor::text FROM game_journal_outbox o JOIN game_journal_streams s USING(stream_key) WHERE o.stream_key=$1").bind(next.as_bytes()).bind(second.as_bytes()).fetch_one(&h.pool).await?;
        assert_eq!(evidence, (true, true, true, "1".into()));
        assert!(
            sqlx::query("SELECT game_journal_enqueue($1,$2)")
                .bind(next.as_bytes())
                .bind(second.as_bytes())
                .execute(&h.pool)
                .await
                .is_err()
        );
        stage.set("cleanup");
        h.cleanup().await?;
        Ok(())
    })
}
