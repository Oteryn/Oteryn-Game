// PRIV-GUARD-1: every function a CHECK constraint calls must be executable by
// `oteryn_game_runtime`. PostgreSQL evaluates a CHECK's functions as the
// inserting/updating role, so a migration that revokes EXECUTE from PUBLIC on
// such a function without granting it to the runtime role makes every runtime
// write fail with 42501 (0016, fixed by 0018). Other tests run as the table
// owner and never see it, so this walks the catalog instead of a list.

use sqlx::{Connection, Executor};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const RUNTIME_ROLE: &str = "oteryn_game_runtime";

/// One row per (CHECK constraint, function it depends on) on a user table the
/// runtime role can INSERT or UPDATE (table or any column level).
const CHECK_FUNCTIONS: &str = "\
SELECT c.conrelid::regclass::text AS table_name, \
       c.conname::text AS constraint_name, \
       p.oid::regprocedure::text AS function_name, \
       has_function_privilege($1, p.oid, 'EXECUTE') AS runtime_can_execute \
  FROM pg_constraint c \
  JOIN pg_class t ON t.oid = c.conrelid \
  JOIN pg_namespace n ON n.oid = t.relnamespace \
  JOIN pg_depend d ON d.classid = 'pg_constraint'::regclass \
                  AND d.objid = c.oid AND d.refclassid = 'pg_proc'::regclass \
  JOIN pg_proc p ON p.oid = d.refobjid \
 WHERE c.contype = 'c' \
   AND n.nspname NOT IN ('pg_catalog', 'information_schema') \
   AND n.nspname NOT LIKE 'pg_toast%' \
   AND (has_table_privilege($1, t.oid, 'INSERT') \
        OR has_table_privilege($1, t.oid, 'UPDATE') \
        OR has_any_column_privilege($1, t.oid, 'INSERT') \
        OR has_any_column_privilege($1, t.oid, 'UPDATE')) \
 ORDER BY 1, 2, 3";

type Row = (String, String, String, bool);

struct Database {
    admin_url: String,
    name: String,
    url: String,
}

impl Database {
    async fn create(admin_url: String) -> TestResult<Self> {
        if !admin_url.starts_with("postgresql://oteryn_test_admin:")
            || !admin_url.ends_with("@127.0.0.1:5432/postgres")
        {
            return Err("unsafe PostgreSQL test admin URL".into());
        }
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = format!("cf_privileges_{suffix}");
        let mut admin = sqlx::PgConnection::connect(&admin_url).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE DATABASE {name}"
            ))))
            .await?;
        admin.close().await?;
        let prefix = admin_url
            .strip_suffix("/postgres")
            .ok_or("invalid admin URL")?;
        let url = format!("{prefix}/{name}");
        let mut connection = sqlx::PgConnection::connect(&url).await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&mut connection)
            .await?;
        assert_eq!(version, "170006", "canonical target is PostgreSQL 17.6");
        sqlx::migrate!("./migrations").run(&mut connection).await?;
        connection.close().await?;
        Ok(Self {
            admin_url,
            name,
            url,
        })
    }

    async fn cleanup(self) -> TestResult {
        let mut admin = sqlx::PgConnection::connect(&self.admin_url).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE {} WITH (FORCE)",
                self.name
            ))))
            .await?;
        admin.close().await?;
        Ok(())
    }
}

fn offenders(rows: &[Row]) -> Vec<String> {
    rows.iter()
        .filter(|row| !row.3)
        .map(|row| format!("{} CHECK {} calls {}", row.0, row.1, row.2))
        .collect()
}

#[test]
fn runtime_role_executes_every_check_function() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured");
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let database = Database::create(admin).await?;
            let outcome = async {
                let mut connection = sqlx::PgConnection::connect(&database.url).await?;
                let rows: Vec<Row> = sqlx::query_as(CHECK_FUNCTIONS)
                    .bind(RUNTIME_ROLE)
                    .fetch_all(&mut connection)
                    .await?;

                // Positive control: the walk sees the known 0016 dependency, so
                // an empty or broken query cannot pass vacuously.
                let known = rows.iter().filter(|row| {
                    row.0 == "game_character_death_receipts"
                        && row.2 == "game_character_is_blessing_set(text[])"
                });
                assert!(
                    known.count() >= 2,
                    "catalog walk must find the blessings_before/blessings_after CHECKs \
                     that call game_character_is_blessing_set; rows: {rows:?}"
                );

                // Mutation control: the same query flags a revoked grant. The
                // transaction is rolled back, so nothing persists.
                let mut tx = connection.begin().await?;
                tx.execute(sqlx::query(
                    "REVOKE EXECUTE ON FUNCTION game_character_is_blessing_set(text[]) \
                       FROM oteryn_game_runtime",
                ))
                .await?;
                let revoked: Vec<Row> = sqlx::query_as(CHECK_FUNCTIONS)
                    .bind(RUNTIME_ROLE)
                    .fetch_all(&mut *tx)
                    .await?;
                tx.rollback().await?;
                assert!(
                    offenders(&revoked)
                        .iter()
                        .any(|line| line.contains("game_character_is_blessing_set")),
                    "a revoked grant must be reported"
                );

                let bad = offenders(&rows);
                assert!(
                    bad.is_empty(),
                    "{RUNTIME_ROLE} cannot EXECUTE functions used by CHECK constraints \
                     (INSERT/UPDATE fails with 42501); add a GRANT EXECUTE in a migration:\n{}",
                    bad.join("\n")
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            outcome
        })
}
