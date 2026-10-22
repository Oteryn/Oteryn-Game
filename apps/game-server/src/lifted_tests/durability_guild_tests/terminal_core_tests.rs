//! Core cleanup proof boundaries; no Bank/Hall implementation is substituted.
use super::*;

#[test]
fn guild_core_terminal_transaction_rejects_foreign_transaction_and_unassigned_world() -> TestResult
{
    text_run("guild_core_context", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        assert_eq!(guild_job_start(h, 1).await?, GuildDeadlineOutcome::Started);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let request = GuildDisbandJobRequest {
            world_id: fence()?.runtime_scope.world_id(),
            guild_id: gid(1),
        };
        let mut tx = h.pool.begin().await?;
        let context =
            lock_guild_disband_transaction(&mut tx, &h.root, &authority, &h.node, &request)
                .await
                .map_err(debug)?
                .ok_or("pending context missing")?;
        context.assert_current(&mut tx).await.map_err(debug)?;
        let mut foreign = h.pool.begin().await?;
        assert!(matches!(
            context.assert_current(&mut foreign).await,
            Err(GuildError::NotCurrent)
        ));
        foreign.rollback().await?;
        tx.rollback().await?;
        let mut replacement = h.pool.begin().await?;
        assert!(matches!(
            context.assert_current(&mut replacement).await,
            Err(GuildError::NotCurrent)
        ));
        replacement.rollback().await?;
        let unassigned = fixture::register(&h.root, 3).await?;
        assert!(matches!(
            h.root
                .advance_guild_disband(&authority, &unassigned, request.clone())
                .await,
            Err(GuildError::NotCurrent)
        ));
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        assert!(matches!(
            other
                .advance_guild_disband(&authority, &h.node, request.clone())
                .await,
            Err(GuildError::NotCurrent)
        ));
        let mut wrong = request;
        wrong.world_id = crate::foundation::WorldId::decode(&id(91)).map_err(debug)?;
        assert!(matches!(
            h.root
                .advance_guild_disband(&authority, &h.node, wrong)
                .await,
            Err(GuildError::NotCurrent)
        ));
        assert_eq!(h.count("game_guild_disband_steps").await?, 0);
        assert_eq!(h.count("game_guild_members").await?, 1);
        Ok(())
    })
}

#[test]
fn guild_core_terminal_uncomposed_financial_owner_fails_closed_on_restart() -> TestResult {
    text_run("guild_core_held", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        assert_eq!(guild_job_start(h, 1).await?, GuildDeadlineOutcome::Started);
        // Negative schema-presence input only: this empty relation cannot attest payout or
        // release anything. Installing any financial owner requires its real composition.
        h.pool
            .execute("CREATE TABLE game_guild_bank_balances(guild_id UUID PRIMARY KEY)")
            .await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let request = GuildDisbandJobRequest {
            world_id: fence()?.runtime_scope.world_id(),
            guild_id: gid(1),
        };
        let restarted = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let authority = restarted
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        for _ in 0..2 {
            assert_eq!(
                restarted
                    .advance_guild_disband(&authority, &h.node, request.clone())
                    .await
                    .map_err(debug)?,
                GuildDisbandOutcome::WaitingForHouseDisposition
            );
        }
        assert_eq!(h.count("game_guild_members").await?, 1);
        assert_eq!(h.count("game_guild_ranks").await?, 4);
        assert_eq!(h.count("game_guild_disband_steps").await?, 0);
        let mut tx = h.pool.begin().await?;
        let error = sqlx::query(
            "SELECT game_guild_core_assert_released(encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
        )
        .bind(gid(1).as_bytes().as_slice())
        .bind(id(WORLD).as_slice())
        .execute(&mut *tx)
        .await
        .expect_err("uncomposed owner accepted");
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("0A000")
        );
        tx.rollback().await?;
        Ok(())
    })
}

#[test]
fn guild_core_terminal_two_roots_commit_one_cleanup_and_keep_start_evidence() -> TestResult {
    text_run("guild_core_race", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let first = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let second = other.open_character_authority(&seal).await.map_err(debug)?;
        let world = fence()?.runtime_scope.world_id();
        let (left, right) = fixture::join_two(
            oteryn_gameplay_social::guild::run_guild_world_pass(
                &h.root, &first, &h.node, world, &mut None,
            ),
            oteryn_gameplay_social::guild::run_guild_world_pass(
                &other, &second, &h.node, world, &mut None,
            ),
        )
        .await;
        left.map_err(debug)?;
        right.map_err(debug)?;
        assert_eq!(h.count("game_guild_job_checkpoints").await?, 1);
        assert_eq!(h.count("game_guild_job_event_outbox").await?, 1);
        assert_eq!(h.count("game_guild_disband_steps").await?, 1);
        assert_eq!(h.count("game_guild_disband_step_events").await?, 1);
        assert_eq!(h.count("game_guild_members").await?, 0);
        let state: i16 = sqlx::query_scalar("SELECT state FROM game_guilds")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(state, 4);
        Ok(())
    })
}
