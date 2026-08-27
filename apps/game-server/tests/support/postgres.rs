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
        admin
            .execute(format!("CREATE DATABASE {database_name}"))
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
        let terminate = format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}' AND pid <> pg_backend_pid()",
            self.database_name
        );
        admin.execute(terminate.as_str()).await?;
        admin
            .execute(format!("DROP DATABASE IF EXISTS {}", self.database_name))
            .await?;
        Ok(())
    }
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
