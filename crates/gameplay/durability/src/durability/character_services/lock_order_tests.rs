use super::*;

#[test]
fn service_waits_for_admission_before_holding_portfolio() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    portfolio_lock_order(false, false, false)
}

#[test]
fn fresh_admission_waits_for_relations_before_holding_portfolio() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    portfolio_lock_order(true, false, false)
}

#[test]
fn busy_portfolio_does_not_hold_admission_relations() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    portfolio_lock_order(true, true, false)
}

#[test]
fn concurrent_portfolio_insert_does_not_hold_admission_relations() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    portfolio_lock_order(false, true, true)
}

fn portfolio_lock_order(fresh_admission: bool, portfolio_blocked: bool, inserting: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
            use sqlx::Connection;
            let Ok(url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()); };
            let mut admin = sqlx::PgConnection::connect(&url).await?;
            // Identifiers contain only a fixed prefix and decimal process/time values; tables below are literals.
            let schema = format!("audit_coin_lock_{}_{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos());
            sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}"))).execute(&mut admin).await?;
            sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}"))).execute(&mut admin).await?;
            for table in [
                "game_durability_admission_account_guards", "game_durability_admission_character_guards",
                "game_durability_admission_guard_history", "game_durability_admission_lifecycle_receipts",
                "game_durability_admission_runtime_guards", "game_durability_admission_signing_trust_guards",
                "game_durability_control_loss_continuity", "game_durability_executor_custody",
                "game_durability_fresh_admission_receipts", "game_durability_reconnect_attempts",
                "game_durability_reconnect_pending_commands", "game_durability_reconnect_sessions",
                "game_durability_recovery_grant_consumptions", "game_durability_session_replacements",
                "game_durability_session_use_ledgers", "game_durability_session_use_memberships",
                "game_durability_transport_ref_reservations",
            ] {
                sqlx::query(sqlx::AssertSqlSafe(format!("CREATE TABLE {table}(id integer)"))).execute(&mut admin).await?;
            }
            sqlx::raw_sql("CREATE TABLE game_character_account_guards(account_id uuid PRIMARY KEY); INSERT INTO game_character_account_guards VALUES('00000000-0000-7000-8000-000000000001')").execute(&mut admin).await?;
            let mut worker = sqlx::PgConnection::connect(&url).await?;
            sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}"))).execute(&mut worker).await?;
            let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&mut worker).await?;
            let mut blocker = sqlx::PgConnection::connect(&url).await?;
            sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}"))).execute(&mut blocker).await?;
            let mut blocker = blocker.begin().await?;
            let account = if inserting {
                "00000000-0000-7000-8000-000000000003"
            } else {
                "00000000-0000-7000-8000-000000000001"
            };
            if inserting {
                sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES($1::uuid)")
                    .bind(account).execute(&mut *blocker).await?;
            } else if portfolio_blocked {
                sqlx::query("SELECT account_id FROM game_character_account_guards FOR UPDATE")
                    .fetch_one(&mut *blocker).await?;
            } else {
                super::super::db::lock_admission_relations(&mut blocker).await?;
            }
            let mut pending = tokio::spawn(async move {
                let mut tx = worker.begin().await?;
                if fresh_admission {
                    super::super::fresh_admission::lock_admission_portfolio(&mut tx, account).await?;
                } else {
                    portfolio(&mut tx, account).await?;
                }
                tx.commit().await?;
                Ok::<_, DurabilityError>(())
            });
            let probe = tokio::time::timeout(std::time::Duration::from_secs(10), async {
                if portfolio_blocked {
                    // Confirm the worker has reached the portfolio before probing from another account.
                    let query_pattern = if inserting {
                        "INSERT INTO game_character_account_guards%"
                    } else {
                        "%game_character_account_guards%"
                    };
                    loop {
                        let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND query LIKE $2)")
                            .bind(pid).bind(query_pattern).fetch_one(&mut admin).await?;
                        if active { break; }
                        tokio::task::yield_now().await;
                    }
                    let mut other = admin.begin().await?;
                    sqlx::query("SET LOCAL lock_timeout='500ms'").execute(&mut *other).await?;
                    let probe = super::super::fresh_admission::lock_admission_portfolio(
                        &mut other, "00000000-0000-7000-8000-000000000002",
                    ).await;
                    other.rollback().await?;
                    assert!(!pending.is_finished());
                    assert!(probe.is_ok(), "unrelated admission waited behind the busy portfolio: {probe:?}");
                } else {
                    loop {
                        let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE pid=$1 AND NOT granted AND locktype='relation')")
                            .bind(pid).fetch_one(&mut admin).await?;
                        if waiting { break; }
                        tokio::task::yield_now().await;
                    }
                    let probe = sqlx::query("SELECT account_id FROM game_character_account_guards FOR UPDATE NOWAIT").fetch_one(&mut *blocker).await;
                    assert!(probe.is_ok(), "service held portfolio while waiting for admission: {probe:?}");
                }
                if inserting {
                    blocker.commit().await?;
                } else {
                    blocker.rollback().await?;
                }
                (&mut pending).await??;
                sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE"))).execute(&mut admin).await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            }).await;
            if probe.is_err() {
                pending.abort();
            }
            probe?
        })
    })?.join().map_err(|_| "test thread panicked")?
}
