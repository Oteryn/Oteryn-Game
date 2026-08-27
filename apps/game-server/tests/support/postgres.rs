use sqlx::postgres::PgConnectOptions;
use sqlx::{Connection, Executor, PgConnection};
use std::env;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};

static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub enum IsolatedPostgresError {
    MissingAdminUrl,
    UnsafeAdminUrl,
    Sqlx(sqlx::Error),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PostgresE2eAvailability {
    NotConfigured,
    Configured,
}

impl Display for IsolatedPostgresError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingAdminUrl => {
                "OTERYN_TEST_POSTGRES_ADMIN_URL is required for PostgreSQL E2E"
            }
            Self::UnsafeAdminUrl => "PostgreSQL E2E requires the pinned local test-admin URL",
            Self::Sqlx(error) => {
                return write!(formatter, "PostgreSQL E2E operation failed: {error}");
            }
        })
    }
}

impl std::error::Error for IsolatedPostgresError {}

impl From<sqlx::Error> for IsolatedPostgresError {
    fn from(error: sqlx::Error) -> Self {
        Self::Sqlx(error)
    }
}

#[derive(Debug)]
pub struct IsolatedPostgres {
    admin_url: String,
    database_name: String,
}

impl IsolatedPostgres {
    pub async fn create(test_name: &str) -> Result<Self, IsolatedPostgresError> {
        let admin_url = env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")
            .map_err(|_error| IsolatedPostgresError::MissingAdminUrl)?;
        validate_admin_url(&admin_url)?;

        let database_name = database_name(test_name)?;
        let mut admin = PgConnection::connect(&admin_url).await?;
        // `database_name` is generated below from ASCII alphanumerics/underscores only.
        let create_database = format!("CREATE DATABASE {database_name}");
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(create_database)))
            .await?;

        Ok(Self {
            admin_url,
            database_name,
        })
    }

    pub fn database_url(&self) -> Result<String, IsolatedPostgresError> {
        let prefix = self
            .admin_url
            .strip_suffix("/postgres")
            .ok_or(IsolatedPostgresError::UnsafeAdminUrl)?;
        Ok(format!("{prefix}/{}", self.database_name))
    }

    pub async fn cleanup(self) -> Result<(), IsolatedPostgresError> {
        let mut admin = PgConnection::connect(&self.admin_url).await?;
        sqlx::query(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
             WHERE datname = $1 AND pid <> pg_backend_pid()",
        )
        .bind(&self.database_name)
        .execute(&mut admin)
        .await?;
        // The test name is normalized by `database_name`, so this identifier is not user SQL.
        let drop_database = format!("DROP DATABASE IF EXISTS {}", self.database_name);
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(drop_database)))
            .await?;
        Ok(())
    }
}

pub fn postgres_e2e_availability() -> Result<PostgresE2eAvailability, IsolatedPostgresError> {
    match env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") {
        Ok(admin_url) => classify_e2e_admin_url(Some(&admin_url)),
        Err(env::VarError::NotPresent) => classify_e2e_admin_url(None),
        Err(env::VarError::NotUnicode(_value)) => Err(IsolatedPostgresError::UnsafeAdminUrl),
    }
}

pub fn classify_e2e_admin_url(
    value: Option<&str>,
) -> Result<PostgresE2eAvailability, IsolatedPostgresError> {
    let Some(value) = value else {
        return Ok(PostgresE2eAvailability::NotConfigured);
    };
    validate_admin_url(value)?;
    Ok(PostgresE2eAvailability::Configured)
}

pub fn validate_admin_url(value: &str) -> Result<(), IsolatedPostgresError> {
    if value.contains(['?', '#', '\n', '\r'])
        || env::var_os("PGOPTIONS").is_some()
        || env::var_os("PGHOST").is_some()
        || env::var_os("PGSERVICE").is_some()
        || env::var_os("PGSERVICEFILE").is_some()
    {
        return Err(IsolatedPostgresError::UnsafeAdminUrl);
    }

    let options = PgConnectOptions::from_str(value)
        .map_err(|_error| IsolatedPostgresError::UnsafeAdminUrl)?;
    if !matches!(options.get_host(), "127.0.0.1" | "localhost")
        || options.get_port() != 5432
        || options.get_socket().is_some()
        || options.get_username() != "oteryn_test_admin"
        || options.get_database() != Some("postgres")
        || !value.starts_with("postgresql://oteryn_test_admin:")
        || !value.ends_with("/postgres")
    {
        return Err(IsolatedPostgresError::UnsafeAdminUrl);
    }

    Ok(())
}

fn database_name(test_name: &str) -> Result<String, IsolatedPostgresError> {
    let normalized: String = test_name
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() {
                char::from(byte.to_ascii_lowercase())
            } else {
                '_'
            }
        })
        .collect();
    if normalized.is_empty() || normalized.len() > 40 {
        return Err(IsolatedPostgresError::UnsafeAdminUrl);
    }

    let ordinal = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let process = std::process::id();
    Ok(format!("oteryn_game_test_{process}_{ordinal}_{normalized}"))
}

#[cfg(test)]
mod durability_contract_tests {
    use super::{
        IsolatedPostgres, IsolatedPostgresError, PostgresE2eAvailability,
        postgres_e2e_availability,
    };
    use crate::durability::{
        AdmissionReconnectJournal, DurabilityError, MigrationExecutor, SchemaCompatibility,
    };
    use sqlx::{Connection, Executor, PgConnection};
    use std::error::Error;
    use std::future::Future;
    use std::time::Duration;

    type TestResult = Result<(), Box<dyn Error>>;

    fn run_postgres_test<F>(future: F) -> TestResult
    where
        F: Future<Output = TestResult>,
    {
        if postgres_e2e_availability()? == PostgresE2eAvailability::NotConfigured {
            return Ok(());
        }
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(future)
    }

    async fn migrated_database(
        test_name: &str,
    ) -> Result<(IsolatedPostgres, String, MigrationExecutor), Box<dyn Error>> {
        let database = IsolatedPostgres::create(test_name).await?;
        let database_url = database.database_url()?;
        let executor = MigrationExecutor::connect_migration(&database_url).await?;
        executor.apply_embedded_ledger().await?;
        Ok((database, database_url, executor))
    }

    #[test]
    fn runtime_startup_rejects_missing_ledger_without_creating_ddl() -> TestResult {
        run_postgres_test(async {
            let database = IsolatedPostgres::create("runtime_no_ddl").await?;
            let database_url = database.database_url()?;

            let startup = AdmissionReconnectJournal::connect_runtime(&database_url).await;
            assert!(matches!(
                startup,
                Err(DurabilityError::SchemaIncompatible(
                    SchemaCompatibility::MissingMigrationLedger
                ))
            ));

            let mut connection = PgConnection::connect(&database_url).await?;
            let migration_table: Option<String> =
                sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations')::text")
                    .fetch_one(&mut connection)
                    .await?;
            assert!(
                migration_table.is_none(),
                "runtime startup must not create the SQLx migration ledger"
            );
            connection.close().await?;
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn checksum_mismatch_is_runtime_incompatible() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, executor) = migrated_database("checksum_mismatch").await?;
            let mut connection = PgConnection::connect(&database_url).await?;
            sqlx::query("UPDATE _sqlx_migrations SET checksum = decode('00', 'hex')")
                .execute(&mut connection)
                .await?;
            connection.close().await?;

            assert_eq!(executor.inspect().await?, SchemaCompatibility::Incompatible);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn dirty_migration_is_runtime_incompatible() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, executor) = migrated_database("dirty_migration").await?;
            let mut connection = PgConnection::connect(&database_url).await?;
            sqlx::query("UPDATE _sqlx_migrations SET success = false")
                .execute(&mut connection)
                .await?;
            connection.close().await?;

            assert_eq!(executor.inspect().await?, SchemaCompatibility::Incompatible);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn behind_migration_ledger_is_runtime_incompatible() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, executor) = migrated_database("behind_ledger").await?;
            let mut connection = PgConnection::connect(&database_url).await?;
            sqlx::query("DELETE FROM _sqlx_migrations")
                .execute(&mut connection)
                .await?;
            connection.close().await?;

            assert_eq!(executor.inspect().await?, SchemaCompatibility::Incompatible);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn ahead_migration_ledger_is_runtime_incompatible() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, executor) = migrated_database("ahead_ledger").await?;
            let mut connection = PgConnection::connect(&database_url).await?;
            sqlx::query(
                "INSERT INTO _sqlx_migrations \
                 (version, description, success, checksum, execution_time) \
                 SELECT version + 1000000, 'synthetic ahead migration', true, checksum, 0 \
                 FROM _sqlx_migrations ORDER BY version DESC LIMIT 1",
            )
            .execute(&mut connection)
            .await?;
            connection.close().await?;

            assert_eq!(executor.inspect().await?, SchemaCompatibility::Incompatible);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn migration_lock_interruption_releases_before_any_ddl_and_allows_retry() -> TestResult {
        run_postgres_test(async {
            let database = IsolatedPostgres::create("migration_lock_interrupt").await?;
            let database_url = database.database_url()?;
            let migration_lock_id = sqlx_migration_lock_id(&database.database_name);

            let mut lock_holder = PgConnection::connect(&database_url).await?;
            sqlx::query("SELECT pg_advisory_lock($1)")
                .bind(migration_lock_id)
                .execute(&mut lock_holder)
                .await?;

            let executor = MigrationExecutor::connect_migration(&database_url).await?;
            let mut migration = Box::pin(executor.apply_embedded_ledger());
            assert!(
                tokio::time::timeout(Duration::from_millis(250), migration.as_mut())
                    .await
                    .is_err(),
                "migration must wait while SQLx's database advisory lock is held"
            );

            let mut observer = PgConnection::connect(&database_url).await?;
            let migration_table: Option<String> =
                sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations')::text")
                    .fetch_one(&mut observer)
                    .await?;
            assert!(
                migration_table.is_none(),
                "SQLx acquires the migration lock before creating the migration ledger"
            );
            observer.close().await?;

            lock_holder.close().await?;
            tokio::time::timeout(Duration::from_secs(5), migration.as_mut()).await??;
            drop(migration);
            assert_eq!(executor.inspect().await?, SchemaCompatibility::Compatible);

            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn isolated_database_outage_fails_closed_and_runtime_recovers() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, executor) = migrated_database("database_outage").await?;
            drop(executor);

            let mut admin = PgConnection::connect(&database.admin_url).await?;
            set_accepting_connections(&mut admin, &database.database_name, false).await?;
            sqlx::query(
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
                 WHERE datname = $1 AND pid <> pg_backend_pid()",
            )
            .bind(&database.database_name)
            .execute(&mut admin)
            .await?;

            let outage = AdmissionReconnectJournal::connect_runtime(&database_url).await;
            assert!(matches!(outage, Err(DurabilityError::Database(_))));

            set_accepting_connections(&mut admin, &database.database_name, true).await?;
            let recovered = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            drop(recovered);
            admin.close().await?;
            database.cleanup().await?;
            Ok(())
        })
    }

    async fn set_accepting_connections(
        admin: &mut PgConnection,
        database_name: &str,
        accepting: bool,
    ) -> Result<(), IsolatedPostgresError> {
        let value = if accepting { "true" } else { "false" };
        let statement = format!("ALTER DATABASE {database_name} WITH ALLOW_CONNECTIONS {value}");
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(statement)))
            .await?;
        Ok(())
    }

    // Mirrors sqlx-postgres 0.9.0 migrate::generate_lock_id without adding a new crate dependency.
    fn sqlx_migration_lock_id(database_name: &str) -> i64 {
        0x3d32_ad9e_i64 * i64::from(crc32_iso_hdlc(database_name.as_bytes()))
    }

    fn crc32_iso_hdlc(bytes: &[u8]) -> u32 {
        let mut checksum = u32::MAX;
        for &byte in bytes {
            checksum ^= u32::from(byte);
            for _ in 0..8 {
                let low_bit_mask = 0_u32.wrapping_sub(checksum & 1);
                checksum = (checksum >> 1) ^ (0xedb8_8320 & low_bit_mask);
            }
        }
        !checksum
    }
}
