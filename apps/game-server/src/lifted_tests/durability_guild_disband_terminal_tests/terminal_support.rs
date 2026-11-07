//! Reuse the existing Guild snapshot and native Harness; an uncomposed owner only refuses.
use super::*;
pub(super) fn job() -> TestResult<GuildDisbandJobRequest> {
    Ok(GuildDisbandJobRequest {
        world_id: fence()?.runtime_scope.world_id(),
        guild_id: gid(1),
    })
}
pub(super) fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    text_run(tag, async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        assert_eq!(guild_job_start(h, 1).await?, GuildDeadlineOutcome::Started);
        // Negative schema-presence input: no fake release/payout implementation or witness.
        h.pool
            .execute("CREATE TABLE game_guild_bank_balances(guild_id UUID PRIMARY KEY)")
            .await?;
        body(h).await
    })
}
pub(super) async fn snapshot(h: &Harness) -> TestResult<Vec<String>> {
    let mut values = Vec::new();
    for table in [
        "game_guilds",
        "game_guild_members",
        "game_guild_invitations",
        "game_guild_ranks",
        "game_account_guild_leadership",
        "game_guild_disband_steps",
        "game_guild_disband_step_events",
        "game_guild_operations",
        "game_guild_event_outbox",
    ] {
        values.push(sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT coalesce(jsonb_agg(x ORDER BY x::text),'[]'::jsonb)::text FROM (SELECT to_jsonb(t) AS x FROM {table} t) snapshots"))).fetch_one(&h.pool).await?);
    }
    Ok(values)
}
