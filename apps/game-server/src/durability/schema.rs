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

#[cfg(test)]
mod contract_tests {
    const MIGRATION: &str = include_str!("../../migrations/0001_admission_reconnect_journal.sql");

    fn session_schema() -> Option<&'static str> {
        MIGRATION
            .split_once("CREATE TABLE game_durability_transport_ref_reservations")
            .map(|(session, _rest)| session)
    }

    #[test]
    fn reconnect_session_schema_binds_actor_and_runtime_scope_identity() {
        let session = session_schema();
        assert!(
            session.is_some(),
            "session table must precede transport reservations"
        );
        let Some(session) = session else {
            return;
        };
        for required in [
            "account_id UUID NOT NULL",
            "character_id UUID NOT NULL",
            "world_id UUID NOT NULL",
            "runtime_scope_kind SMALLINT NOT NULL",
            "runtime_scope_world_id UUID NOT NULL",
            "runtime_scope_channel_id UUID NULL",
            "runtime_scope_instance_id UUID NULL",
        ] {
            assert!(
                session.contains(required),
                "reconnect session schema must retain {required}"
            );
        }
    }

    #[test]
    fn reconnect_authority_fences_preserve_the_full_unsigned_range() {
        for required in [
            "predecessor_generation NUMERIC(20, 0) NOT NULL",
            "character_lease_generation NUMERIC(20, 0) NOT NULL",
            "scope_ownership_generation NUMERIC(20, 0) NOT NULL",
            "current_generation NUMERIC(20, 0) NOT NULL",
        ] {
            assert!(
                MIGRATION.contains(required),
                "reconnect session schema must retain full u64 range for {required}"
            );
        }
        assert_eq!(
            MIGRATION
                .matches("control_loss_epoch NUMERIC(20, 0) NOT NULL")
                .count(),
            2,
            "both session and attempt ControlLossEpoch mirrors must retain the full u64 range"
        );
    }
}
