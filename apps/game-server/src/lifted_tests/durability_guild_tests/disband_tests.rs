use super::*;
#[test]
fn guild_disband_native_confirmation_data_is_bounded_and_codec_closed() -> TestResult {
    let common = request(1, 1, GuildCommand::Disband);
    assert!(GuildDisbandRequest::new(common.clone(), &"a".repeat(30)).is_err());
    assert!(GuildDisbandRequest::new(common.clone(), "Guild\0Name").is_err());
    assert!(GuildDisbandRequest::new(request(1, 1, GuildCommand::Leave), "Guild").is_err());
    // Empty/wrong but bounded confirmation is DATA for a typed NotAllowed, never a name normalizer.
    assert!(GuildDisbandRequest::new(common, "").is_ok());
    Ok(())
}
#[test]
fn guild_disband_native_leader_start_replay_and_exact_confirmation_binding() -> TestResult {
    for forming in [false, true] {
        text_run("guild_disband_start", async |h| {
            initial_guild(h, 1, 0).await?;
            sqlx::query("UPDATE game_guilds SET state=$1")
                .bind(if forming { 1_i16 } else { 2 })
                .execute(&h.pool)
                .await?;
            guard(h).await?;
            let f = fence()?;
            let wrong = disband_request(1, f, &name(1).to_lowercase());
            assert_eq!(
                result(disband_apply(h, wrong, f).await?).result,
                GuildResult::NotAllowed
            );
            assert_eq!(revision(h, 1).await?, "1");
            assert_eq!(h.count("game_guild_job_checkpoints").await?, 0);
            let req = disband_request(2, f, &name(1));
            let outcome = disband_apply(h, req.clone(), f).await?;
            assert_eq!(result(outcome).result, GuildResult::Ok);
            assert_eq!(revision(h, 1).await?, "2");
            assert_eq!(h.count("game_guild_members").await?, 1);
            assert_eq!(h.count("game_guild_ranks").await?, 4);
            assert_eq!(h.count("game_account_guild_leadership").await?, 1);
            assert_eq!(h.count("game_guild_job_event_outbox").await?, 0);
            assert_eq!(h.count("game_guild_event_outbox").await?, 1);
            let reason: i16 = sqlx::query_scalar("SELECT reason FROM game_guild_job_checkpoints")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(reason, 3);
            let payload: Vec<u8> =
                sqlx::query_scalar("SELECT payload FROM game_guild_event_outbox")
                    .fetch_one(&h.pool)
                    .await?;
            let audit: crate::durability::guild_audit::GuildDisbandAuditProposal =
                serde_json::from_slice(&payload)?;
            assert_eq!(audit.confirmation, name(1));
            assert_eq!(audit.checked_bytes(), Some(payload));
            let mut changed = audit.clone();
            changed.operation.action = 12;
            assert!(changed.checked_bytes().is_none());
            let mut stale = f;
            stale.connection_generation = ConnectionGeneration::new(99)?;
            assert!(matches!(
                disband_apply(h, req.clone(), stale).await?,
                GuildOutcome::Replayed(_)
            ));
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut changed = req;
            changed.confirmation.push(' ');
            assert!(matches!(
                h.root
                    .commit_guild_disband(&authority, &h.node, f, changed)
                    .await,
                Err(GuildError::ConflictingOccurrence)
            ));
            assert!(matches!(
                h.root
                    .commit_guild_membership(
                        &authority,
                        &h.node,
                        f,
                        request(3, 1, GuildCommand::Disband)
                    )
                    .await,
                Err(GuildError::InvalidInput)
            ));
            assert_eq!(
                result(disband_apply(h, disband_request(4, f, &name(1)), f).await?).result,
                GuildResult::GuildDisbanding
            );
            assert_eq!(h.count("game_guild_event_outbox").await?, 1);
            assert_eq!(text_character_revision(h).await?, "1");
            Ok(())
        })?;
    }
    Ok(())
}
#[test]
fn guild_disband_native_current_leader_required_not_vice_or_other_guild() -> TestResult {
    for rank in [None, Some(2_i16), Some(3)] {
        text_run("guild_disband_role", async |h| {
            historical_character(h, 90, id(WORLD)).await?;
            initial_guild(h, 1, 0).await?;
            if let Some(rank) = rank {
                initial_member(h, 1, 90, rank).await?;
            } else {
                initial_guild(h, 2, 90).await?;
            }
            let f = historical_session(h, 90).await?;
            guard(h).await?;
            assert_eq!(
                result(disband_apply(h, disband_request(1, f, &name(1)), f).await?).result,
                GuildResult::NotAllowed
            );
            assert_eq!(revision(h, 1).await?, "1");
            assert_eq!(h.count("game_guild_job_checkpoints").await?, 0);
            assert_eq!(h.count("game_guild_event_outbox").await?, 0);
            Ok(())
        })?;
    }
    Ok(())
}
#[test]
fn guild_disband_native_current_authority_rejections_write_nothing() -> TestResult {
    text_run("guild_disband_authority", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let f = fence()?;
        for field in 0..3 {
            let mut stale = f;
            match field {
                0 => stale.connection_generation = ConnectionGeneration::new(2)?,
                1 => stale.character_lease_generation = 2,
                _ => stale.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
            }
            assert!(matches!(
                h.root
                    .commit_guild_disband(
                        &authority,
                        &h.node,
                        stale,
                        disband_request(field + 1, stale, &name(1))
                    )
                    .await,
                Err(GuildError::NotCurrent)
            ));
        }
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        assert!(matches!(
            other
                .commit_guild_disband(&authority, &h.node, f, disband_request(4, f, &name(1)))
                .await,
            Err(GuildError::NotCurrent)
        ));
        let wrong = crate::durability::runtime_scope_assignment::NodeIncarnationProof::new(
            h.node.fact(),
            crate::durability::runtime_scope_assignment::BootstrapSecret::from_bytes([99; 32]),
        );
        assert!(matches!(
            h.root
                .commit_guild_disband(&authority, &wrong, f, disband_request(5, f, &name(1)))
                .await,
            Err(GuildError::NotCurrent)
        ));
        assert_eq!(h.count("game_guild_operations").await?, 0);
        assert_eq!(h.count("game_guild_job_checkpoints").await?, 0);
        assert_eq!(revision(h, 1).await?, "1");
        Ok(())
    })
}
#[test]
fn guild_disband_native_event_forgery_rolls_back_command_checkpoint_and_state() -> TestResult {
    text_run("guild_disband_rollback", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        h.pool.execute("CREATE FUNCTION guild_disband_fault() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN NEW.payload=convert_to(replace(convert_from(NEW.payload,'UTF8'),'Guild Fixture B','Wrong Guild'),'UTF8');NEW.payload_sha256=sha256(NEW.payload);RETURN NEW;END$$;CREATE TRIGGER aaa_guild_disband_fault BEFORE INSERT ON game_guild_event_outbox FOR EACH ROW EXECUTE FUNCTION guild_disband_fault()").await?;
        assert!(
            disband_apply(h, disband_request(1, fence()?, &name(1)), fence()?)
                .await
                .is_err()
        );
        assert_eq!(h.count("game_guild_operations").await?, 0);
        assert_eq!(h.count("game_guild_job_checkpoints").await?, 0);
        assert_eq!(h.count("game_guild_event_outbox").await?, 0);
        assert_eq!(revision(h, 1).await?, "1");
        h.pool
            .execute("DROP TRIGGER aaa_guild_disband_fault ON game_guild_event_outbox")
            .await?;
        assert_eq!(
            result(disband_apply(h, disband_request(1, fence()?, &name(1)), fence()?).await?)
                .result,
            GuildResult::Ok
        );
        assert!(
            h.pool
                .execute("UPDATE game_guilds SET state=4")
                .await
                .is_err()
        );
        Ok(())
    })
}
#[test]
fn guild_disband_native_deadline_job_race_commits_one_start_only() -> TestResult {
    text_run("guild_disband_job_race", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let b = other.open_character_authority(&seal).await.map_err(debug)?;
        let f = fence()?;
        let world = f.runtime_scope.world_id();
        let (command, job) = fixture::join_two(
            h.root
                .commit_guild_disband(&a, &h.node, f, disband_request(1, f, &name(1))),
            other.start_guild_deadline_disband(&b, &h.node, world, gid(1)),
        )
        .await;
        let command = result(command.map_err(debug)?).result;
        let job = job.map_err(debug)?;
        assert!(
            (command == GuildResult::Ok && job == GuildDeadlineOutcome::AlreadyStarted)
                || (command == GuildResult::GuildDisbanding
                    && job == GuildDeadlineOutcome::Started)
        );
        assert_eq!(h.count("game_guild_job_checkpoints").await?, 1);
        assert_eq!(
            h.count("game_guild_job_event_outbox").await?
                + h.count("game_guild_event_outbox").await?,
            1
        );
        assert_eq!(revision(h, 1).await?, "2");
        assert_eq!(h.count("game_guild_members").await?, 1);
        Ok(())
    })
}
#[test]
fn guild_disband_native_rechecks_real_leader_after_resign() -> TestResult {
    text_run("guild_disband_resign", async |h| {
        let successor = resign_fixture(h).await?;
        guard(h).await?;
        let premium = pre_delivery(&h.root);
        assert_eq!(
            result(resign_apply(h, resign_request(1, 91, 1), &premium).await?).result,
            GuildResult::Ok
        );
        assert_eq!(
            result(disband_apply(h, disband_request(2, fence()?, &name(1)), fence()?).await?)
                .result,
            GuildResult::NotAllowed
        );
        assert_eq!(h.count("game_guild_job_checkpoints").await?, 0);
        assert_eq!(
            result(disband_apply(h, disband_request(3, successor, &name(1)), successor).await?)
                .result,
            GuildResult::Ok
        );
        assert_eq!(revision(h, 1).await?, "3");
        assert_eq!(h.count("game_guild_job_checkpoints").await?, 1);
        Ok(())
    })
}
