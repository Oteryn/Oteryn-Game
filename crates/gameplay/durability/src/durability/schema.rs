use crate::durability::{DurabilityError, db};
use sqlx::postgres::PgConnection;
use sqlx::{Executor, PgPool, Postgres, Row};

pub use oteryn_gameplay_durability::GAME_MIGRATOR;

const MIGRATION_LEDGER_INSPECTION_SQL: &str = "WITH expected(version, checksum_len) AS (\
     SELECT * FROM unnest($1::BIGINT[], $2::BIGINT[])\
 ) \
 SELECT m.version, octet_length(m.checksum)::BIGINT AS checksum_len, \
        CASE WHEN e.checksum_len IS NOT NULL \
                   AND octet_length(m.checksum)::BIGINT = e.checksum_len \
             THEN m.checksum ELSE NULL END AS bounded_checksum, \
        m.success \
 FROM _sqlx_migrations AS m \
 LEFT JOIN expected AS e ON e.version = m.version \
 ORDER BY m.version ASC \
 LIMIT $3";

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

    /// This is the only migration path in this module allowed to execute DDL.
    /// Runtime compatibility inspection remains read-only; the URL connector below is test-only.
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

#[cfg(any(test, feature = "test-support"))]
pub async fn connect_runtime(database_url: &str) -> Result<PgPool, DurabilityError> {
    let pool = db::connect(database_url, 4).await?;
    let compatibility = inspect(&pool).await?;
    if compatibility != SchemaCompatibility::Compatible {
        return Err(DurabilityError::SchemaIncompatible(compatibility));
    }
    Ok(pool)
}

async fn inspect(pool: &PgPool) -> Result<SchemaCompatibility, DurabilityError> {
    inspect_executor(pool).await
}

pub async fn inspect_connection(
    connection: &mut PgConnection,
) -> Result<SchemaCompatibility, DurabilityError> {
    inspect_executor(connection).await
}

async fn inspect_executor<'e, E>(executor: E) -> Result<SchemaCompatibility, DurabilityError>
where
    E: Executor<'e, Database = Postgres>,
{
    let expected: Vec<_> = GAME_MIGRATOR.iter().collect();
    let expected_versions: Vec<i64> = expected.iter().map(|migration| migration.version).collect();
    let expected_checksum_lengths: Vec<i64> = expected
        .iter()
        .map(|migration| i64::try_from(migration.checksum.len()))
        .collect::<Result<_, _>>()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let row_limit = i64::try_from(expected.len())
        .ok()
        .and_then(|count| count.checked_add(1))
        .ok_or(DurabilityError::InvalidStoredState)?;

    // The row transfer is bounded by embedded migration count N + one overflow
    // sentinel. A checksum is transferred only when its server-side byte length
    // exactly matches the embedded checksum for the same version.
    let rows = match sqlx::query(MIGRATION_LEDGER_INSPECTION_SQL)
        .bind(&expected_versions)
        .bind(&expected_checksum_lengths)
        .bind(row_limit)
        .fetch_all(executor)
        .await
    {
        Ok(rows) => rows,
        Err(error) if is_missing_table(&error) => {
            return Ok(SchemaCompatibility::MissingMigrationLedger);
        }
        Err(error) => return Err(DurabilityError::from(error)),
    };

    if rows.len() != expected.len() {
        return Ok(SchemaCompatibility::Incompatible);
    }

    for (row, (migration, expected_checksum_len)) in rows
        .iter()
        .zip(expected.iter().zip(expected_checksum_lengths.iter()))
    {
        let version: i64 = row.try_get("version")?;
        let checksum_len: i64 = row.try_get("checksum_len")?;
        let checksum: Option<Vec<u8>> = row.try_get("bounded_checksum")?;
        let success: bool = row.try_get("success")?;
        if !success
            || version != migration.version
            || checksum_len != *expected_checksum_len
            || checksum.as_deref() != Some(migration.checksum.as_ref())
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

#[cfg(test)]
#[path = "schema_contract_tests.rs"]
mod contract_tests;

#[cfg(test)]
#[path = "schema_terminal_replacement_postgres_red_tests.rs"]
mod terminal_replacement_postgres_red_tests;
