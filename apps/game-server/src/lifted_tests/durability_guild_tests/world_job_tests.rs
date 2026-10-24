//! Installed migration and production World scheduler; no reconstructed Guild schema/guards.
use super::*;
use oteryn_gameplay_social::guild::run_guild_world_pass;

#[test]
fn guild_world_job_installed_migration_real_commands_and_restart_cleanup() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = Harness::create(admin, "guild_world_installed").await?;
        let outcome = async {
            let premium = pre_delivery(&h.root);
            assert_eq!(
                result(found(&h, found_request(1, 1, "Guild Origin"), fence()?, &premium).await?)
                    .result,
                GuildResult::Ok
            );
            let mut successor = None;
            for target in 91..=94 {
                historical_character(&h, target, id(WORLD)).await?;
                let target_fence = historical_session(&h, target).await?;
                if target == 91 {
                    successor = Some(target_fence);
                }
                assert_eq!(
                    result(
                        apply(
                            &h,
                            request(target, 1, GuildCommand::Invite(character(target))),
                            fence()?
                        )
                        .await?
                    )
                    .result,
                    GuildResult::Ok
                );
                let mut accept = request(target + 100, 1, GuildCommand::Accept);
                accept.command = CommandRef::new(target_fence.game_session_id, CommandId::new(1)?);
                assert_eq!(
                    result(apply(&h, accept, target_fence).await?).result,
                    GuildResult::Ok
                );
                let revision = revision(&h, 1).await?.parse()?;
                assert_eq!(
                    result(
                        rank_apply(
                            &h,
                            rank_request(target + 200, target, revision, 2),
                            &premium
                        )
                        .await?
                    )
                    .result,
                    GuildResult::Ok
                );
            }
            let state: i16 = sqlx::query_scalar("SELECT state FROM game_guilds")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(state, 2);
            let revision = revision(&h, 1).await?.parse()?;
            assert_eq!(
                result(resign_apply(&h, resign_request(400, 91, revision), &premium).await?).result,
                GuildResult::Ok
            );
            let successor = successor.ok_or("missing successor session")?;
            let mut common = request(401, 1, GuildCommand::Disband);
            common.command = CommandRef::new(successor.game_session_id, CommandId::new(2)?);
            let disband = GuildDisbandRequest::new(common, "Guild Origin").map_err(debug)?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            assert_eq!(
                result(
                    h.root
                        .commit_guild_disband(&authority, &h.node, successor, disband)
                        .await
                        .map_err(debug)?
                )
                .result,
                GuildResult::Ok
            );
            let world = fence()?.runtime_scope.world_id();
            let mut cursor = None;
            run_guild_world_pass(&h.root, &authority, &h.node, world, &mut cursor)
                .await
                .map_err(debug)?;
            // GUILD-BANK-1: with the bank installed the first committed step only pays the
            // Guild balance to the leader; membership cleanup starts on the next tick.
            assert_eq!(h.count("game_guild_members").await?, 5);
            assert_eq!(h.count("game_guild_disband_steps").await?, 0);
            // The fair cursor now sits on this Guild: the next tick finds nothing after it and
            // wraps, and the tick after that starts membership cleanup.
            run_guild_world_pass(&h.root, &authority, &h.node, world, &mut cursor)
                .await
                .map_err(debug)?;
            assert!(cursor.is_none());
            assert_eq!(h.count("game_guild_members").await?, 5);
            run_guild_world_pass(&h.root, &authority, &h.node, world, &mut cursor)
                .await
                .map_err(debug)?;
            assert_eq!(h.count("game_guild_members").await?, 1);
            let restarted = DurabilityRoot::connect_test_runtime(&h.database.url)?;
            assert!(restarted.maintain_ready_once().await?);
            let current = restarted
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            // A new process loses its delivery cursor, not the durable work checkpoint.
            cursor = None;
            run_guild_world_pass(&restarted, &current, &h.node, world, &mut cursor)
                .await
                .map_err(debug)?;
            let snapshot: (i16, String) = sqlx::query_as("SELECT state,name FROM game_guilds")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(snapshot, (4, "Guild Origin".into()));
            for table in [
                "game_guild_members",
                "game_guild_ranks",
                "game_guild_invitations",
                "game_account_guild_leadership",
            ] {
                assert_eq!(h.count(table).await?, 0, "orphan rows in {table}");
            }
            assert_eq!(h.count("game_guild_disband_steps").await?, 2);
            run_guild_world_pass(&restarted, &current, &h.node, world, &mut cursor)
                .await
                .map_err(debug)?;
            assert_eq!(h.count("game_guild_disband_steps").await?, 2);
            assert_eq!(text_character_revision(&h).await?, "1");
            Ok(())
        }
        .await;
        h.cleanup().await?;
        outcome
    })
}

#[test]
fn guild_world_job_cleanup_is_bounded_and_event_failure_rolls_back() -> TestResult {
    text_run("guild_world_bounded", async |h| {
        initial_guild(h, 1, 0).await?;
        for n in 91..=191 {
            historical_character(h, n, id(WORLD)).await?;
            initial_member(h, 1, n, 3).await?;
        }
        guard(h).await?;
        let confirmation = name(1);
        let request =
            GuildDisbandRequest::new(request(700, 1, GuildCommand::Disband), &confirmation)
                .map_err(debug)?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            result(
                h.root
                    .commit_guild_disband(&authority, &h.node, fence()?, request)
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        let job = GuildDisbandJobRequest {
            world_id: fence()?.runtime_scope.world_id(),
            guild_id: gid(1),
        };
        event_failure::install(h, true).await?;
        assert!(
            h.root
                .advance_guild_disband(&authority, &h.node, job.clone())
                .await
                .is_err()
        );
        assert_eq!(h.count("game_guild_members").await?, 102);
        assert_eq!(h.count("game_guild_disband_steps").await?, 0);
        event_failure::remove(h, true).await?;
        assert_eq!(
            h.root
                .advance_guild_disband(&authority, &h.node, job.clone())
                .await
                .map_err(debug)?,
            GuildDisbandOutcome::Advanced {
                step: 2,
                removed: 100
            }
        );
        assert_eq!(h.count("game_guild_members").await?, 2);
        assert_eq!(
            h.root
                .advance_guild_disband(&authority, &h.node, job.clone())
                .await
                .map_err(debug)?,
            GuildDisbandOutcome::Advanced {
                step: 3,
                removed: 1
            }
        );
        assert!(matches!(
            h.root
                .advance_guild_disband(&authority, &h.node, job)
                .await
                .map_err(debug)?,
            GuildDisbandOutcome::Disbanded { .. }
        ));
        assert_eq!(h.count("game_guild_ranks").await?, 0);
        Ok(())
    })
}
