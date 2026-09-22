//! Dedicated migration-only entry point for the game durability ledger.

// Use the canonical library so private adapter seals and the ledger share one
// crate identity; this entry point still performs migration-only execution.
use oteryn_game_server::durability::MigrationExecutor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("OTERYN_GAME_MIGRATION_DATABASE_URL").map_err(|_error| {
        std::io::Error::other("OTERYN_GAME_MIGRATION_DATABASE_URL is required for migrations")
    })?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let executor = MigrationExecutor::connect_migration(&database_url).await?;
            executor.apply_embedded_ledger().await
        })?;
    Ok(())
}
