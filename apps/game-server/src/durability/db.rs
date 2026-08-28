use crate::durability::DurabilityError;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn connect(database_url: &str, max_connections: u32) -> Result<PgPool, DurabilityError> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(CONNECT_TIMEOUT)
        .connect(database_url)
        .await
        .map_err(DurabilityError::from)
}
