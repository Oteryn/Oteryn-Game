use super::*;
#[test]
fn guild_deadline_jobs_strict_bounds_and_closed_states() {
    use oteryn_gameplay_social_rules::guild::{GuildDeadlineJob, VICE_DEFICIT_GRACE_MS};
    let formation = GuildDeadlineJob::Formation;
    let vice = GuildDeadlineJob::ViceDeficit;
    assert!(!formation.due(GuildState::Forming, 100, None, 100));
    assert!(formation.due(GuildState::Forming, 100, None, 101));
    assert!(!vice.due(
        GuildState::Active,
        0,
        Some(100),
        100 + VICE_DEFICIT_GRACE_MS
    ));
    assert!(vice.due(
        GuildState::Active,
        0,
        Some(100),
        101 + VICE_DEFICIT_GRACE_MS
    ));
    assert!(!vice.due(GuildState::Active, 0, None, i64::MAX));
    assert!(vice.due(GuildState::Active, 0, Some(i64::MIN), i64::MAX));
    assert!(!vice.due(GuildState::Active, 0, Some(i64::MAX), i64::MIN));
    for state in [GuildState::Disbanding, GuildState::Disbanded] {
        assert!(!formation.due(state, 0, None, i64::MAX));
        assert!(!vice.due(state, 0, Some(0), i64::MAX));
    }
    assert!(!formation.due(GuildState::Active, 0, None, i64::MAX));
    assert!(!vice.due(GuildState::Forming, 0, Some(0), i64::MAX));
}
#[test]
fn guild_deadline_native_restart_preserves_pending_step_without_terminalization() -> TestResult {
    text_run("guild_job_restart", async |h| {
        initial_guild(h, 1, 0).await?;
        sqlx::query("UPDATE game_guilds SET state=1,formation_deadline=1")
            .execute(&h.pool)
            .await?;
        guard(h).await?;
        let before = footprint(h).await?;
        assert_eq!(guild_job_start(h, 1).await?, GuildDeadlineOutcome::Started);
        assert_eq!(footprint(h).await?, before); // no members/ranks/leadership are removed early.
        assert_eq!(revision(h, 1).await?, "2");
        assert_eq!(h.count("game_guild_job_checkpoints").await?, 1);
        assert_eq!(h.count("game_guild_job_event_outbox").await?, 1);
        let payload: Vec<u8> =
            sqlx::query_scalar("SELECT payload FROM game_guild_job_event_outbox")
                .fetch_one(&h.pool)
                .await?;
        let audit: crate::durability::guild_audit::GuildJobAuditProposal =
            serde_json::from_slice(&payload)?;
        assert_eq!(
            (
                audit.step,
                audit.reason,
                audit.previous_revision,
                audit.revision
            ),
            (1, 1, 1, 2)
        );
        assert_eq!(audit.checked_bytes(), Some(payload.clone()));
        let mut forged = audit.clone();
        forged.revision += 1;
        assert!(forged.checked_bytes().is_none());
        let restarted = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let new_node = fixture::register(&restarted, 2).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = restarted
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = fence()?.runtime_scope.world_id();
        assert_eq!(
            restarted
                .pending_guild_disbands(&authority, &new_node, world)
                .await
                .map_err(debug)?,
            vec![gid(1)]
        );
        assert_eq!(
            restarted
                .start_guild_deadline_disband(&authority, &new_node, world, gid(1))
                .await
                .map_err(debug)?,
            GuildDeadlineOutcome::AlreadyStarted
        );
        assert!(
            restarted
                .guild_deadline_candidates(&authority, &new_node, world)
                .await
                .map_err(debug)?
                .is_empty()
        );
        assert_eq!(revision(h, 1).await?, "2");
        assert_eq!(h.count("game_guild_job_event_outbox").await?, 1);
        let state: i16 = sqlx::query_scalar("SELECT state FROM game_guilds")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(state, 3); // pending Hall/Bank work, never falsely DISBANDED.
        assert_eq!(
            result(apply(h, request(1, 1, GuildCommand::Leave), fence()?).await?).result,
            GuildResult::GuildDisbanding
        );
        Ok(())
    })
}
#[test]
fn guild_deadline_native_vice_and_world_recheck() -> TestResult {
    text_run("guild_job_world", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let foreign = crate::foundation::WorldId::decode(&sid(9, 40))?;
        assert!(
            h.root
                .guild_deadline_candidates(&authority, &h.node, foreign)
                .await
                .map_err(debug)?
                .is_empty()
        );
        assert_eq!(
            h.root
                .start_guild_deadline_disband(&authority, &h.node, foreign, gid(1))
                .await
                .map_err(debug)?,
            GuildDeadlineOutcome::NoLongerDue
        );
        assert_eq!(h.count("game_guild_job_checkpoints").await?, 0);
        assert_eq!(guild_job_start(h, 1).await?, GuildDeadlineOutcome::Started);
        let reason: i16 = sqlx::query_scalar("SELECT reason FROM game_guild_job_checkpoints")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(reason, 2);
        assert_eq!(
            guild_job_start(h, 1).await?,
            GuildDeadlineOutcome::AlreadyStarted
        );
        assert_eq!(
            guild_job_start(h, 999).await?,
            GuildDeadlineOutcome::NoLongerDue
        );
        Ok(())
    })
}
#[test]
fn guild_deadline_native_rollback_on_publication_failure_then_retry() -> TestResult {
    text_run("guild_job_rollback", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        h.pool.execute("CREATE FUNCTION guild_job_fault() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN NEW.payload=convert_to('{}','UTF8');NEW.payload_sha256=sha256(NEW.payload);RETURN NEW;END$$;CREATE TRIGGER aaa_guild_job_fault BEFORE INSERT ON game_guild_job_event_outbox FOR EACH ROW EXECUTE FUNCTION guild_job_fault()").await?;
        assert!(guild_job_start(h, 1).await.is_err());
        assert_eq!(revision(h, 1).await?, "1");
        assert_eq!(h.count("game_guild_job_checkpoints").await?, 0);
        assert_eq!(h.count("game_guild_job_event_outbox").await?, 0);
        let state: i16 = sqlx::query_scalar("SELECT state FROM game_guilds")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(state, 2);
        h.pool
            .execute("DROP TRIGGER aaa_guild_job_fault ON game_guild_job_event_outbox")
            .await?;
        assert_eq!(guild_job_start(h, 1).await?, GuildDeadlineOutcome::Started);
        assert_eq!(h.count("game_guild_job_event_outbox").await?, 1);
        for statement in [
            "DELETE FROM game_guild_job_checkpoints",
            "UPDATE game_guild_job_event_outbox SET payload=payload",
            "TRUNCATE game_guild_job_event_outbox",
            "UPDATE game_guilds SET state=4",
        ] {
            assert!(h.pool.execute(statement).await.is_err());
        }
        Ok(())
    })
}
#[test]
fn guild_deadline_native_concurrent_start_has_one_checkpoint_and_event() -> TestResult {
    text_run("guild_job_race", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let left = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let right = other.open_character_authority(&seal).await.map_err(debug)?;
        let world = fence()?.runtime_scope.world_id();
        let (a, b) = fixture::join_two(
            h.root
                .start_guild_deadline_disband(&left, &h.node, world, gid(1)),
            other.start_guild_deadline_disband(&right, &h.node, world, gid(1)),
        )
        .await;
        let a = a.map_err(debug)?;
        let b = b.map_err(debug)?;
        assert!(
            (a == GuildDeadlineOutcome::Started && b == GuildDeadlineOutcome::AlreadyStarted)
                || (b == GuildDeadlineOutcome::Started
                    && a == GuildDeadlineOutcome::AlreadyStarted)
        );
        assert_eq!(revision(h, 1).await?, "2");
        assert_eq!(h.count("game_guild_job_checkpoints").await?, 1);
        assert_eq!(h.count("game_guild_job_event_outbox").await?, 1);
        Ok(())
    })
}
#[test]
fn guild_deadline_native_current_authority_required_even_after_checkpoint() -> TestResult {
    text_run("guild_job_authority", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = fence()?.runtime_scope.world_id();
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        assert!(matches!(
            other
                .start_guild_deadline_disband(&authority, &h.node, world, gid(1))
                .await,
            Err(GuildError::NotCurrent)
        ));
        let wrong_secret = crate::durability::runtime_scope_assignment::NodeIncarnationProof::new(
            h.node.fact(),
            crate::durability::runtime_scope_assignment::BootstrapSecret::from_bytes([99; 32]),
        );
        assert!(
            h.root
                .start_guild_deadline_disband(&authority, &wrong_secret, world, gid(1))
                .await
                .is_err()
        );
        assert_eq!(h.count("game_guild_job_checkpoints").await?, 0);
        assert_eq!(guild_job_start(h, 1).await?, GuildDeadlineOutcome::Started);
        assert!(
            h.root
                .start_guild_deadline_disband(&authority, &wrong_secret, world, gid(1))
                .await
                .is_err()
        );
        assert_eq!(h.count("game_guild_job_event_outbox").await?, 1);
        h.pool.execute("INSERT INTO game_character_recovery_admissions(authority_scope_id,recovery_generation,recovery_event_id,predecessor_generation,predecessor_digest,issued_at,issuer_identity,reconciled_at) SELECT authority_scope_id,recovery_generation+1,'10000000-0000-7000-8000-000000000001',recovery_generation,decode(repeat('00',32),'hex'),issued_at+1,issuer_identity,reconciled_at FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1").await?;
        assert!(
            h.root
                .start_guild_deadline_disband(&authority, &h.node, world, gid(1))
                .await
                .is_err()
        );
        assert!(
            h.root
                .pending_guild_disbands(&authority, &h.node, world)
                .await
                .is_err()
        );
        assert_eq!(h.count("game_guild_job_event_outbox").await?, 1);
        Ok(())
    })
}
#[test]
fn guild_deadline_native_bounded_pass_and_locked_candidate_recheck() -> TestResult {
    text_run("guild_job_cap", async |h| {
        for n in 1..=101 {
            historical_character(h, n, id(WORLD)).await?;
            initial_guild(h, n, n).await?;
        }
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = fence()?.runtime_scope.world_id();
        let candidates = h
            .root
            .guild_deadline_candidates(&authority, &h.node, world)
            .await
            .map_err(debug)?;
        assert_eq!(candidates.len(), 100);
        assert!(candidates.windows(2).all(|pair| pair[0] < pair[1]));
        let first = oteryn_gameplay_social::guild::run_guild_deadline_pass(
            &h.root, &authority, &h.node, world,
        )
        .await
        .map_err(debug)?;
        assert_eq!(first.started.len(), 100);
        let second = oteryn_gameplay_social::guild::run_guild_deadline_pass(
            &h.root, &authority, &h.node, world,
        )
        .await
        .map_err(debug)?;
        assert_eq!(second.started.len(), 1);
        assert_eq!(h.count("game_guild_job_checkpoints").await?, 101);
        assert_eq!(h.count("game_guild_job_event_outbox").await?, 101);
        // A candidate selected before another worker starts it must recheck under its Guild lock.
        assert_eq!(
            h.root
                .start_guild_deadline_disband(&authority, &h.node, world, candidates[0])
                .await
                .map_err(debug)?,
            GuildDeadlineOutcome::AlreadyStarted
        );
        assert!(
            h.root
                .guild_deadline_candidates(&authority, &h.node, world)
                .await
                .map_err(debug)?
                .is_empty()
        );
        assert_eq!(
            h.root
                .pending_guild_disbands(&authority, &h.node, world)
                .await
                .map_err(debug)?
                .len(),
            100
        );
        Ok(())
    })
}
