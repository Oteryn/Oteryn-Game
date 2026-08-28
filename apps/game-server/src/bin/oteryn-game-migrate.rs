//! Dedicated migration-only entry point for the game durability ledger.

// The migration binary deliberately invokes only the migration executor. The
// complete public durability boundary is compiled so it embeds the same ledger,
// while its runtime journal API remains unused in this one-purpose binary.
#[allow(dead_code, unused_imports)]
#[path = "../durability/mod.rs"]
mod durability;

use durability::MigrationExecutor;

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
