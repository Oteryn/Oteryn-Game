//! Fixed private export bytes, contiguous revisions, exact retry and stale fence.
use super::TestResult;
use super::fixture::{Harness, run};
#[test]
fn game_journal_outbox_fixed_bytes_ack_and_generation_fence() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "journal PG thread failed")?
        .map_err(Into::into)
}
fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create(admin, "journal_outbox", false).await?;
        let stream=br#"{"contract_version":1,"operation":"ReadGameJournalCursorV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":null,"character_revision":null,"from_projection_revision":null,"to_projection_revision":null,"projection_epoch":null,"family":"BANK","recovery_generation":"1"}"#.as_slice();
        let record=br#"{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":null,"character_revision":null,"from_projection_revision":null,"to_projection_revision":null,"projection_epoch":null,"family":"BANK","recovery_generation":"1","source_revision":"1","record_id":"019a0000-0000-7000-8000-000000000007","occurred_at":"1793836800","coverage_start_at":"1793836800","kind":"BANK_ENTRY","payload":{"operation_occurrence_id":"019a0000-0000-7000-8000-000000000008","event_id":"019a0000-0000-7000-8000-000000000009","entry_id":"019a0000-0000-7000-8000-00000000000a","previous_entry_id":null,"kind":"DEPOSIT","direction":"CREDIT","amount":"100","balance_before":"0","balance_after":"100"}}"#.as_slice();
        sqlx::query("INSERT INTO game_journal_streams(stream_key,source_authority,account_id,recovery_generation) VALUES($1,'OTERYN_GAME_JOURNAL','019a0000-0000-7000-8000-000000000001',1)").bind(stream).execute(&h.pool).await?;
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
            .replace("\"source_revision\":\"1\"", "\"source_revision\":\"3\"");
        assert!(
            sqlx::query("SELECT game_journal_enqueue($1,$2)")
                .bind(stream)
                .bind(gap.as_bytes())
                .execute(&h.pool)
                .await
                .is_err()
        );
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
        h.cleanup().await?;
        Ok(())
    })
}
