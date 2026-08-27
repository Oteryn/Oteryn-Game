use crate::durability::{DurabilityError, db};
use sqlx::{PgPool, Row};

pub(crate) static GAME_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaCompatibility {
    Compatible,
    MissingMigrationLedger,
    Incompatible,
}

pub struct MigrationExecutor {
    pool: PgPool,
}

impl MigrationExecutor {
    pub async fn connect_migration(database_url: &str) -> Result<Self, DurabilityError> {
        Ok(Self {
            pool: db::connect(database_url, 1).await?,
        })
    }

    /// This is the only runtime path in this module allowed to execute DDL.
    /// Normal game-server startup must use `inspect` through `connect_runtime`.
    pub async fn apply_embedded_ledger(&self) -> Result<(), DurabilityError> {
        GAME_MIGRATOR
            .run(&self.pool)
            .await
            .map_err(DurabilityError::from)
    }

    pub async fn inspect(&self) -> Result<SchemaCompatibility, DurabilityError> {
        inspect(&self.pool).await
    }
}

pub(crate) async fn connect_runtime(database_url: &str) -> Result<PgPool, DurabilityError> {
    let pool = db::connect(database_url, 4).await?;
    let compatibility = inspect(&pool).await?;
    if compatibility != SchemaCompatibility::Compatible {
        return Err(DurabilityError::SchemaIncompatible(compatibility));
    }
    Ok(pool)
}

async fn inspect(pool: &PgPool) -> Result<SchemaCompatibility, DurabilityError> {
    let rows = match sqlx::query(
        "SELECT version, checksum, success FROM _sqlx_migrations ORDER BY version ASC",
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) if is_missing_table(&error) => {
            return Ok(SchemaCompatibility::MissingMigrationLedger);
        }
        Err(error) => return Err(DurabilityError::from(error)),
    };

    let expected: Vec<_> = GAME_MIGRATOR.iter().collect();
    if rows.len() != expected.len() {
        return Ok(SchemaCompatibility::Incompatible);
    }

    for (row, migration) in rows.iter().zip(expected) {
        let version: i64 = row.try_get("version")?;
        let checksum: Vec<u8> = row.try_get("checksum")?;
        let success: bool = row.try_get("success")?;
        if !success
            || version != migration.version
            || checksum.as_slice() != migration.checksum.as_ref()
        {
            return Ok(SchemaCompatibility::Incompatible);
        }
    }

    Ok(SchemaCompatibility::Compatible)
}

fn is_missing_table(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|database| database.code())
        .is_some_and(|code| code == "42P01")
}
