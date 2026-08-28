//! PostgreSQL-backed, journal-only reconnect durability boundary.
//!
//! This module deliberately owns persistence/classification only. Foundation
//! constructs and revalidates reconnect authority; the runtime must submit the
//! resulting request asynchronously and consume its completion as new input.

mod admission_journal;
mod db;
mod schema;

pub use admission_journal::AdmissionReconnectJournal;
pub use schema::{MigrationExecutor, SchemaCompatibility};

use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum DurabilityError {
    Database(sqlx::Error),
    Migration(sqlx::migrate::MigrateError),
    SchemaIncompatible(SchemaCompatibility),
    InvalidStoredState,
}

impl Display for DurabilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => {
                write!(formatter, "PostgreSQL durability operation failed: {error}")
            }
            Self::Migration(error) => write!(formatter, "game migration operation failed: {error}"),
            Self::SchemaIncompatible(state) => {
                write!(
                    formatter,
                    "game durability schema is not runtime-compatible: {state:?}"
                )
            }
            Self::InvalidStoredState => {
                formatter.write_str("durability journal contains invalid state")
            }
        }
    }
}

impl std::error::Error for DurabilityError {}

impl From<sqlx::Error> for DurabilityError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<sqlx::migrate::MigrateError> for DurabilityError {
    fn from(error: sqlx::migrate::MigrateError) -> Self {
        Self::Migration(error)
    }
}
