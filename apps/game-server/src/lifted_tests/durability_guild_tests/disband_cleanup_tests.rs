//! Bounded cleanup retains no departed vice leadership and resumes through the World owner.
use super::*;
use oteryn_gameplay_social::guild::run_guild_world_pass;

#[test]
fn guild_disband_cleanup_batches_members_leadership_invitations_and_restart() -> TestResult {
    text_run("guild_cleanup_batches", async |h| {
        initial_guild(h, 1, 0).await?;
        for n in 91..=191 {
            historical_character(h, n, id(WORLD)).await?;
            initial_member(h, 1, n, 2).await?;
        }
        for n in 192..=292 {
            historical_character(h, n, id(WORLD)).await?;
            initial_invitation(h, 1, n, 0, false).await?;
        }
        guard(h).await?;
        // Player start remains valid with 101 vices; no Premium lapse is invented by cleanup.
        let common = request(700, 1, GuildCommand::Disband);
        let disband = GuildDisbandRequest::new(common, &name(1)).map_err(debug)?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            result(
                h.root
                    .commit_guild_disband(&authority, &h.node, fence()?, disband)
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        let world = fence()?.runtime_scope.world_id();
        let job = GuildDisbandJobRequest {
            world_id: world,
            guild_id: gid(1),
        };
        event_failure::install(h, true).await?;
        assert!(
            h.root
                .advance_guild_disband(&authority, &h.node, job)
                .await
                .is_err()
        );
        assert_eq!(h.count("game_guild_members").await?, 102);
        assert_eq!(h.count("game_account_guild_leadership").await?, 102);
        assert_eq!(h.count("game_guild_disband_steps").await?, 0);
        event_failure::remove(h, true).await?;
        run_guild_world_pass(&h.root, &authority, &h.node, world, &mut None)
            .await
            .map_err(debug)?;
        assert_eq!(h.count("game_guild_members").await?, 2);
        assert_eq!(h.count("game_account_guild_leadership").await?, 2);
        assert_eq!(h.count("game_guild_invitations").await?, 101);
        let restarted = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let current = restarted
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        for (members, invitations, leadership) in [(1, 101, 1), (1, 1, 1), (1, 0, 1), (0, 0, 0)] {
            run_guild_world_pass(&restarted, &current, &h.node, world, &mut None)
                .await
                .map_err(debug)?;
            assert_eq!(h.count("game_guild_members").await?, members);
            assert_eq!(h.count("game_guild_invitations").await?, invitations);
            assert_eq!(h.count("game_account_guild_leadership").await?, leadership);
        }
        let steps: Vec<(i16, i32, i32, i32)> = sqlx::query_as("SELECT phase,cardinality(member_ids),cardinality(invitation_ids),cardinality(leadership_accounts) FROM game_guild_disband_steps ORDER BY step")
            .fetch_all(&h.pool).await?;
        assert_eq!(
            steps,
            [
                (4, 100, 0, 100),
                (4, 1, 0, 1),
                (5, 0, 100, 0),
                (5, 0, 1, 0),
                (6, 1, 0, 1)
            ]
        );
        assert_eq!(h.count("game_guild_ranks").await?, 0);
        assert_eq!(h.count("game_guild_disband_step_events").await?, 5);
        let state: i16 = sqlx::query_scalar("SELECT state FROM game_guilds")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(state, 4);
        run_guild_world_pass(&restarted, &current, &h.node, world, &mut None)
            .await
            .map_err(debug)?;
        assert_eq!(h.count("game_guild_disband_steps").await?, 5);
        Ok(())
    })
}

#[test]
fn guild_disband_cleanup_guard_requires_exact_batch_accounts_and_removal() -> TestResult {
    text_run("guild_cleanup_guards", async |h| {
        initial_guild(h, 1, 0).await?;
        historical_character(h, 91, id(WORLD)).await?;
        initial_member(h, 1, 91, 2).await?;
        guard(h).await?;
        let unassigned = fixture::register(&h.root, 3).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(matches!(
            h.root
                .start_guild_deadline_disband(
                    &authority,
                    &unassigned,
                    fence()?.runtime_scope.world_id(),
                    gid(1)
                )
                .await,
            Err(GuildError::NotCurrent)
        ));
        assert_eq!(h.count("game_guild_job_checkpoints").await?, 0);
        assert_eq!(guild_job_start(h, 1).await?, GuildDeadlineOutcome::Started);
        for accounts in [
            Vec::new(),
            vec![uuid(account(0))],
            vec![uuid(account(91)), uuid(account(0))],
        ] {
            let mut tx = h.pool.begin().await?;
            let error = sqlx::query("INSERT INTO game_guild_disband_steps(guild_id,step,phase,world_id,previous_revision,revision,member_ids,invitation_ids,leadership_accounts,occurred_at) SELECT guild_id,2,4,world_id,revision,revision+1,ARRAY[encode($1,'hex')::uuid],'{}',ARRAY(SELECT x::uuid FROM unnest($2::text[]) x),floor(extract(epoch FROM transaction_timestamp())*1000)::bigint FROM game_guilds")
                .bind(character(91).as_bytes().as_slice()).bind(accounts).execute(&mut *tx).await.expect_err("unbound batch accounts accepted");
            assert_eq!(
                error.as_database_error().and_then(|e| e.code()).as_deref(),
                Some("23514")
            );
            tx.rollback().await?;
        }
        let job = GuildDisbandJobRequest {
            world_id: fence()?.runtime_scope.world_id(),
            guild_id: gid(1),
        };
        // A trigger that suppresses the account deletion must fail the deferred snapshot proof.
        h.pool.execute("CREATE FUNCTION guild_cleanup_keep_account() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RETURN NULL;END$$;CREATE TRIGGER aaa_guild_cleanup_keep_account BEFORE DELETE ON game_account_guild_leadership FOR EACH ROW EXECUTE FUNCTION guild_cleanup_keep_account()").await?;
        assert!(
            h.root
                .advance_guild_disband(&authority, &h.node, job.clone())
                .await
                .is_err()
        );
        assert_eq!(h.count("game_guild_members").await?, 2);
        assert_eq!(h.count("game_account_guild_leadership").await?, 2);
        assert_eq!(h.count("game_guild_disband_steps").await?, 0);
        h.pool.execute("DROP TRIGGER aaa_guild_cleanup_keep_account ON game_account_guild_leadership;DROP FUNCTION guild_cleanup_keep_account()").await?;
        assert_eq!(
            h.root
                .advance_guild_disband(&authority, &h.node, job)
                .await
                .map_err(debug)?,
            GuildDisbandOutcome::Advanced {
                step: 2,
                removed: 1
            }
        );
        assert_eq!(h.count("game_account_guild_leadership").await?, 1);
        Ok(())
    })
}
