use super::TestResult;
use sqlx::Executor;
const MIGRATION: &str =
    include_str!("../../../../../../apps/game-server/migrations/0132_guild_core.sql");
// Retained source-qualified fixtures use the exact installed SQL, never a copied schema.
use std::sync::LazyLock;
pub static GUILD_TABLES_PROPOSAL: LazyLock<&str> = LazyLock::new(|| {
    MIGRATION
        .split("-- GUILD TABLES BEGIN")
        .nth(1)
        .unwrap()
        .split("-- GUILD GUARDS BEGIN")
        .next()
        .unwrap()
});
pub static GUILD_GUARDS_PROPOSAL: LazyLock<&str> = LazyLock::new(|| {
    MIGRATION
        .split("-- GUILD GUARDS BEGIN")
        .nth(1)
        .unwrap()
        .split("-- GUILD GRANTS BEGIN")
        .next()
        .unwrap()
});
pub async fn prepare(pool: &sqlx::PgPool) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(pool)
        .await?;
    assert_eq!(version, crate::CANONICAL_POSTGRES_SERVER_VERSION_NUM);
    eprintln!("GUILD-NATIVE-PG server_version_num={version}");
    // Only this isolated historical snapshot fixture is rebuilt; new real-path tests below
    // use the installed migration without this setup. No production guards are relaxed.
    pool.execute("DROP TABLE game_guild_disband_step_events,game_guild_disband_steps,game_guild_job_event_outbox,game_guild_job_checkpoints,game_guild_event_outbox,game_guild_operations,game_account_guild_leadership,game_guild_invitations,game_guild_members,game_guild_ranks,game_guilds CASCADE").await?;
    pool.execute("DO $$ DECLARE f RECORD;BEGIN FOR f IN SELECT p.oid::regprocedure::text AS signature FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname=current_schema() AND p.proname LIKE 'game_guild_%' LOOP EXECUTE format('DROP FUNCTION IF EXISTS %s CASCADE',f.signature);END LOOP;END $$").await?;
    pool.execute(*GUILD_TABLES_PROPOSAL).await?;
    Ok(())
}
pub async fn guard(pool: &sqlx::PgPool) -> TestResult {
    pool.execute(*GUILD_GUARDS_PROPOSAL).await?;
    Ok(())
}
