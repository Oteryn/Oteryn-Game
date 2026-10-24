//! Ordinary properties do not install a Guild terminal participant.
use super::*;

#[test]
fn house_ownership_guild_participants_require_a_guild_owner() -> TestResult {
    run("house_guild_participants", |h| async move {
        // GUILD-BANK-1 is installed on the current schema, even with no Guild rows.
        let installed: bool = sqlx::query_scalar("SELECT game_guild_core_participants_installed()")
            .fetch_one(&h.pool)
            .await?;
        assert!(installed);
        // Reconstruct the existing core-only snapshot for the original House-only checks.
        // The bank's installed-schema dispatch E2E separately qualifies real payout/cleanup.
        crate::durability::guild_chat_fixture::prepare(&h.pool).await?;
        crate::durability::guild_chat_fixture::guard(&h.pool).await?;
        // Neither installation nor a populated ordinary property needs Guild disposition.
        let installed: bool = sqlx::query_scalar("SELECT game_guild_core_participants_installed()")
            .fetch_one(&h.pool)
            .await?;
        assert!(!installed);
        seed(&h, false).await?;
        let installed: bool = sqlx::query_scalar("SELECT game_guild_core_participants_installed()")
            .fetch_one(&h.pool)
            .await?;
        assert!(!installed);

        // Schema-presence inputs only, never substitutes for release/payout evidence.
        // Each genuine Guild owner must still block cleanup even with no rows.
        for statement in [
            "CREATE TABLE game_guild_bank_balances(guild_id UUID PRIMARY KEY)",
            "CREATE TABLE game_guildhall_properties(guild_id UUID PRIMARY KEY)",
            "CREATE TABLE game_house_guild_ownership(guild_id UUID PRIMARY KEY)",
        ] {
            let mut tx = h.pool.begin().await?;
            sqlx::query(sqlx::AssertSqlSafe(statement))
                .execute(&mut *tx)
                .await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            let installed: bool =
                sqlx::query_scalar("SELECT game_guild_core_participants_installed()")
                    .fetch_one(&mut *tx)
                    .await?;
            assert!(installed, "Guild owner went undetected: {statement}");
            tx.rollback().await?;
        }
        Ok(())
    })
}
