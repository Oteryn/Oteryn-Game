use sqlx::{Connection, Executor};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn m22_captured_ingredient_debit_rejects_partial_and_unknown_snapshots_postgres() -> TestResult {
    let Some(admin_url) = std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("M22_DEBIT POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    };
    let admin_url = admin_url.into_string().map_err(|_| "admin URL encoding")?;
    let prefix = admin_url
        .strip_suffix("/postgres")
        .ok_or("test admin database")?;
    if !prefix
        .rsplit_once('@')
        .is_some_and(|(_, host)| host.starts_with("127.0.0.1:"))
    {
        return Err("test database must use the local routed PostgreSQL".into());
    }
    let database = format!(
        "questpc3_m22_debit_{}_{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let database_url = format!("{prefix}/{database}");
    let started = Instant::now();
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut admin = sqlx::PgConnection::connect(&admin_url).await?;
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "CREATE DATABASE {database}"
                ))))
                .await?;
            let exercised: TestResult = async {
                let mut connection = sqlx::PgConnection::connect(&database_url).await?;
                let version: String = sqlx::query_scalar("SHOW server_version_num")
                    .fetch_one(&mut connection)
                    .await?;
                assert_eq!(
                    version,
                    oteryn_gameplay_durability::CANONICAL_POSTGRES_SERVER_VERSION_NUM
                );
                eprintln!("M22_DEBIT POSTGRES_CONNECTED version={version}");
                oteryn_gameplay_durability::GAME_MIGRATOR
                    .run(&mut connection)
                    .await?;
                let cases = [
                    (true, serde_json::json!({"quantity":7}), 7, true),
                    (true, serde_json::json!({"quantity":7}), 1, true),
                    (true, serde_json::json!({"quantity":7}), 2, false),
                    (true, serde_json::json!({"quantity":100}), 100, true),
                    (true, serde_json::json!({"quantity":101}), 101, false),
                    (true, serde_json::json!({"quantity":0}), 1, false),
                    (true, serde_json::json!({}), 7, false),
                    (true, serde_json::json!({"quantity":null}), 7, false),
                    (true, serde_json::json!({"quantity":"7"}), 7, false),
                    (true, serde_json::json!({"quantity":7}), 0, false),
                    (true, serde_json::json!({"quantity":7}), -1, false),
                ];
                for (present, before, debit, expected) in cases {
                    let actual: bool =
                        sqlx::query_scalar("SELECT game_craft_ingredient_debit_valid($1,$2,$3)")
                            .bind(present)
                            .bind(&before)
                            .bind(debit)
                            .fetch_one(&mut connection)
                            .await?;
                    assert_eq!(actual, expected, "before={before} debit={debit}");
                }
                let fixed_without_target: bool =
                    sqlx::query_scalar("SELECT game_craft_ingredient_debit_valid(false,NULL,1)")
                        .fetch_one(&mut connection)
                        .await?;
                assert!(fixed_without_target);
                for expression in [
                    "game_craft_ingredient_debit_valid(false,NULL,7)",
                    "game_craft_ingredient_debit_valid(true,NULL,1)",
                    "game_craft_ingredient_debit_valid(NULL,NULL,1)",
                    "game_craft_ingredient_debit_valid(false,NULL,NULL)",
                ] {
                    let actual: bool = connection
                        .fetch_one(sqlx::query(sqlx::AssertSqlSafe(format!(
                            "SELECT {expression}"
                        ))))
                        .await?
                        .try_get(0)?;
                    assert!(
                        !actual,
                        "unknown input must return false, not SQL NULL: {expression}"
                    );
                }
                connection.close().await?;
                Ok(())
            }
            .await;
            let cleanup = admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "DROP DATABASE {database} WITH (FORCE)"
                ))))
                .await;
            exercised?;
            cleanup?;
            Ok::<_, Box<dyn std::error::Error>>(())
        });
    eprintln!(
        "M22_DEBIT POSTGRES_CASE_COMPLETED duration_ms={} success={}",
        started.elapsed().as_millis(),
        result.is_ok()
    );
    result
}

use sqlx::Row;
