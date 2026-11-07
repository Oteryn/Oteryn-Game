//! The existing command-event fault injector, shared with cleanup-event rollback tests.
use super::{Harness, TestResult};
use sqlx::Executor;

pub(super) async fn install(h: &Harness, cleanup: bool) -> TestResult {
    h.pool.execute("CREATE FUNCTION guild_test_failure() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'fixture failure' USING ERRCODE='23514';END$$").await?;
    let table = if cleanup {
        "game_guild_disband_step_events"
    } else {
        "game_guild_event_outbox"
    };
    h.pool.execute(sqlx::AssertSqlSafe(format!("CREATE CONSTRAINT TRIGGER guild_test_failure AFTER INSERT ON {table} DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION guild_test_failure()"))).await?;
    Ok(())
}
pub(super) async fn remove(h: &Harness, cleanup: bool) -> TestResult {
    let table = if cleanup {
        "game_guild_disband_step_events"
    } else {
        "game_guild_event_outbox"
    };
    h.pool
        .execute(sqlx::AssertSqlSafe(format!(
            "DROP TRIGGER guild_test_failure ON {table};DROP FUNCTION guild_test_failure()"
        )))
        .await?;
    Ok(())
}

// Moved from the original Resign payload-binding test; both hashes change before retention.
pub(super) async fn install_payload(h: &Harness) -> TestResult {
    h.pool.execute("CREATE TABLE guild_resign_test_payload(payload BYTEA NOT NULL);CREATE FUNCTION guild_resign_test_payload_op() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN SELECT sha256(payload) INTO NEW.audit_sha256 FROM guild_resign_test_payload;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_resign_test_payload_op BEFORE INSERT ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION guild_resign_test_payload_op();CREATE FUNCTION guild_resign_test_payload_event() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN SELECT payload INTO NEW.payload FROM guild_resign_test_payload;NEW.payload_sha256=sha256(NEW.payload);RETURN NEW;END$$;CREATE TRIGGER aaa_guild_resign_test_payload_event BEFORE INSERT ON game_guild_event_outbox FOR EACH ROW EXECUTE FUNCTION guild_resign_test_payload_event();").await?;
    Ok(())
}
pub(super) async fn set_payload(h: &Harness, payload: &serde_json::Value) -> TestResult {
    sqlx::query("DELETE FROM guild_resign_test_payload")
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO guild_resign_test_payload VALUES($1)")
        .bind(serde_json::to_vec(payload)?)
        .execute(&h.pool)
        .await?;
    Ok(())
}
pub(super) async fn remove_payload(h: &Harness) -> TestResult {
    h.pool.execute("DROP TRIGGER aaa_guild_resign_test_payload_op ON game_guild_operations;DROP TRIGGER aaa_guild_resign_test_payload_event ON game_guild_event_outbox;DROP FUNCTION guild_resign_test_payload_op();DROP FUNCTION guild_resign_test_payload_event();DROP TABLE guild_resign_test_payload;").await?;
    Ok(())
}
