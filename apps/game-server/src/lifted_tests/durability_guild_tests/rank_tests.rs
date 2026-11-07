use super::*;
#[test]
fn guild_rank_native_request_binding_and_closed_audit() -> TestResult {
    for level in [0, 21, u8::MAX] {
        assert!(
            GuildRankRequest::new(
                request(1, 1, GuildCommand::SetRank(character(91))),
                1,
                level
            )
            .is_err()
        );
    }
    for level in [1, 20] {
        assert_eq!(rank_request(1, 91, 1, level).level.level(), level);
    }
    assert!(
        GuildRankRequest::new(request(1, 1, GuildCommand::SetRank(character(91))), 0, 2).is_err()
    );
    assert!(
        GuildRankRequest::new(request(1, 1, GuildCommand::SetTitle(character(91))), 1, 2).is_err()
    );
    let original = rank_request(1, 91, 1, 2);
    let binding = rank_binding(&fence()?, &original).map_err(debug)?;
    let mut alternatives = vec![
        rank_request(1, 92, 1, 2),
        rank_request(1, 91, 2, 2),
        rank_request(1, 91, 1, 3),
    ];
    let mut other = original.clone();
    other.common.guild_id = gid(2);
    alternatives.push(other);
    let mut other = original.clone();
    other.common.operation = GuildCommand::SetTitle(character(91));
    alternatives.push(other);
    let mut other = original.clone();
    other.common.command = request(2, 1, GuildCommand::SetRank(character(91))).command;
    alternatives.push(other);
    for changed in alternatives {
        assert_ne!(rank_binding(&fence()?, &changed).map_err(debug)?, binding);
    }
    let audit = GuildRankAuditProposal {
        operation: GuildAuditProposal {
            version: 1,
            occurrence: sid(1, 10),
            transaction_id: sid(1, 11),
            event_id: sid(1, 12),
            guild_id: *gid(1).as_bytes(),
            world_id: id(WORLD),
            actor: id(CHARACTER),
            target: *character(91).as_bytes(),
            action: 9,
            previous_revision: 1,
            revision: 2,
            rank_before: Some(3),
            rank_after: Some(2),
            removed_invitations: 0,
            occurred_at_unix_ms: 1,
        },
        expected_revision: 1,
    };
    let bytes = audit.checked_bytes().ok_or("valid rank audit rejected")?;
    assert_eq!(
        serde_json::from_slice::<GuildRankAuditProposal>(&bytes)?,
        audit
    );
    let mut wrong = audit.clone();
    wrong.expected_revision = 2;
    assert!(wrong.checked_bytes().is_none());
    let mut wrong = audit.clone();
    wrong.operation.action = 7;
    assert!(wrong.checked_bytes().is_none());
    let mut wrong = audit.clone();
    wrong.operation.rank_after = Some(1);
    assert!(wrong.checked_bytes().is_none());
    let mut wrong = audit.clone();
    wrong.operation.rank_before = None;
    assert!(wrong.checked_bytes().is_none());
    let mut wrong = audit.clone();
    wrong.operation.actor = wrong.operation.target;
    assert!(wrong.checked_bytes().is_none());
    let mut json: serde_json::Value = serde_json::from_slice(&bytes)?;
    json["caller_premium"] = true.into();
    assert!(serde_json::from_value::<GuildRankAuditProposal>(json).is_err());
    Ok(())
}
#[test]
fn guild_rank_native_target_premium_and_position_transitions() -> TestResult {
    // GUILD0 §3.2: only the newly promoted TARGET Account consumes Current/NotActivated.
    for case in 0..5 {
        text_run("guild_rank_premium", async |h| {
            rank_fixture(h, 0).await?;
            guard(h).await?;
            let who = if case == 1 { account(0) } else { account(91) };
            let (coordinator, consumer) = current_coordinator(h, who, case != 2).await?;
            if case == 3 {
                consumer.pull_failed(account(91), consumer.ticket());
            }
            let pre = pre_delivery(&h.root);
            let premium = if case == 4 { &pre } else { &coordinator };
            let before = rank_mutation_snapshot(h).await?;
            let outcome = result(rank_apply(h, rank_request(1, 91, 1, 2), premium).await?);
            let success = case == 0 || case == 4;
            assert_eq!(
                outcome.result,
                if success {
                    GuildResult::Ok
                } else {
                    GuildResult::NotPremium
                }
            );
            if !success {
                assert_eq!(rank_mutation_snapshot(h).await?, before);
                assert_eq!(h.count("game_guild_operations").await?, 1);
            } else {
                assert_eq!(rank_level(h, 91).await?, 2);
                assert_eq!(h.count("game_account_guild_leadership").await?, 2);
                // Once the role exists, unavailable Premium cannot revoke it or block demotion.
                let closed =
                    SpellPremiumCoordinator::new(Arc::default(), None, None, Some(h.root.clone()));
                assert_eq!(
                    result(rank_apply(h, rank_request(2, 91, 2, 2), &closed).await?).result,
                    GuildResult::Ok
                );
                assert_eq!(h.count("game_account_guild_leadership").await?, 2);
                assert_eq!(
                    result(rank_apply(h, rank_request(3, 91, 3, 3), &closed).await?).result,
                    GuildResult::Ok
                );
                assert_eq!(rank_level(h, 91).await?, 3);
                assert_eq!(h.count("game_account_guild_leadership").await?, 1);
                assert_eq!(revision(h, 1).await?, "4");
                assert_eq!(h.count("game_guild_event_outbox").await?, 3);
                let audit:Vec<u8>=sqlx::query_scalar("SELECT payload FROM game_guild_event_outbox ORDER BY occurred_at,event_id LIMIT 1").fetch_one(&h.pool).await?;
                let audit: GuildRankAuditProposal = serde_json::from_slice(&audit)?;
                assert_eq!(audit.operation.action, 9);
                assert_eq!(audit.operation.rank_before, Some(3));
                assert_eq!(audit.operation.rank_after, Some(2));
                assert_eq!(audit.expected_revision, 1);
                let unchanged:bool=sqlx::query_scalar("SELECT title='' AND joined_at=1 FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid")
                    .bind(character(91).as_bytes().as_slice()).fetch_one(&h.pool).await?;
                assert!(unchanged);
            }
            assert_eq!(text_character_revision(h).await?, "1");
            let deficit: Option<i64> =
                sqlx::query_scalar("SELECT premium_deficit_since FROM game_guilds")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(deficit, Some(7));
            Ok(())
        })?;
    }
    Ok(())
}
#[test]
fn guild_rank_native_both_strict_role_inequalities_and_existing_rank() -> TestResult {
    // Numeric role authority is resolved from current locked rows, including ordinary rank3.
    for (actor, target, desired, expected) in [
        (1, 3, 2, GuildResult::Ok),
        (2, 3, 2, GuildResult::NotAllowed),
        (2, 3, 4, GuildResult::Ok),
        (3, 4, 4, GuildResult::Ok),
        (3, 3, 4, GuildResult::NotAllowed),
        (3, 4, 3, GuildResult::NotAllowed),
        (1, 3, 1, GuildResult::NotAllowed),
        (1, 3, 20, GuildResult::NotAllowed),
    ] {
        text_run("guild_rank_roles", async |h| {
            historical_character(h, 90, id(WORLD)).await?;
            historical_character(h, 91, id(WORLD)).await?;
            // Actor0 is the leader only in the actor1 case; all other ranks use a real other leader.
            initial_guild(h, 1, if actor == 1 { 0 } else { 90 }).await?;
            if actor != 1 {
                initial_member(h, 1, 0, actor).await?;
            }
            initial_member(h, 1, 91, target).await?;
            guard(h).await?;
            let before = rank_mutation_snapshot(h).await?;
            assert_eq!(
                result(
                    rank_apply(h, rank_request(1, 91, 1, desired), &pre_delivery(&h.root)).await?
                )
                .result,
                expected
            );
            if expected == GuildResult::Ok {
                assert_eq!(rank_level(h, 91).await?, i16::from(desired));
                assert_eq!(revision(h, 1).await?, "2");
            } else {
                assert_eq!(rank_mutation_snapshot(h).await?, before);
            }
            assert_eq!(text_character_revision(h).await?, "1");
            Ok(())
        })?;
    }
    text_run("guild_rank_scope", async |h| {
        initial_guild(h, 1, 0).await?;
        historical_character(h, 90, id(WORLD)).await?;
        initial_guild(h, 2, 90).await?;
        historical_character(h, 91, id(WORLD)).await?;
        initial_member(h, 2, 91, 3).await?;
        historical_character(h, 92, sid(900, 40)).await?;
        guard(h).await?;
        for (n, target) in [(1, 0), (2, 91), (3, 92), (4, 999)] {
            let before = rank_mutation_snapshot(h).await?;
            assert_eq!(
                result(rank_apply(h, rank_request(n, target, 1, 2), &pre_delivery(&h.root)).await?)
                    .result,
                GuildResult::NotAllowed
            );
            assert_eq!(rank_mutation_snapshot(h).await?, before);
        }
        Ok(())
    })
}
#[test]
fn guild_rank_native_formation_and_first_deficit_cure() -> TestResult {
    for forming in [false, true] {
        text_run("guild_rank_lifecycle", async |h| {
            rank_fixture(h, 3).await?;
            sqlx::query("UPDATE game_guilds SET state=$1")
                .bind(if forming { 1_i16 } else { 2 })
                .execute(&h.pool)
                .await?;
            guard(h).await?;
            // The three retained vices have no Current Premium snapshot; all three still count.
            let (premium, _) = current_coordinator(h, account(91), true).await?;
            assert_eq!(
                result(rank_apply(h, rank_request(1, 91, 1, 2), &premium).await?).result,
                GuildResult::Ok
            );
            let row: (i16, Option<i64>, Option<i64>) = sqlx::query_as(
                "SELECT state,vice_deficit_since,premium_deficit_since FROM game_guilds",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(row, (2, None, Some(7)));
            assert_eq!(
                result(rank_apply(h, rank_request(2, 91, 2, 3), &premium).await?).result,
                GuildResult::Ok
            );
            let first: Option<i64> =
                sqlx::query_scalar("SELECT vice_deficit_since FROM game_guilds")
                    .fetch_one(&h.pool)
                    .await?;
            assert!(first.is_some_and(|t| t > 0));
            assert_eq!(
                result(rank_apply(h, rank_request(3, 100, 3, 3), &premium).await?).result,
                GuildResult::Ok
            );
            let second: Option<i64> =
                sqlx::query_scalar("SELECT vice_deficit_since FROM game_guilds")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(second, first);
            assert_eq!(
                result(rank_apply(h, rank_request(4, 91, 4, 2), &premium).await?).result,
                GuildResult::Ok
            );
            let still: Option<i64> =
                sqlx::query_scalar("SELECT vice_deficit_since FROM game_guilds")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(still, first); // three is still a deficit.
            let (last, _) = current_coordinator(h, account(100), true).await?;
            assert_eq!(
                result(rank_apply(h, rank_request(5, 100, 5, 2), &last).await?).result,
                GuildResult::Ok
            );
            let cleared: Option<i64> =
                sqlx::query_scalar("SELECT vice_deficit_since FROM game_guilds")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(cleared, None);
            assert_eq!(revision(h, 1).await?, "6");
            assert_eq!(text_character_revision(h).await?, "1");
            Ok(())
        })?;
    }
    // Curative commands cannot outrun an expired formation/14-day deadline; refusals retain receipts only.
    for state in [1_i16, 2] {
        text_run("guild_rank_late_cure", async |h| {
            rank_fixture(h, 3).await?;
            sqlx::query(
                "UPDATE game_guilds SET state=$1,formation_deadline=1,vice_deficit_since=1",
            )
            .bind(state)
            .execute(&h.pool)
            .await?;
            guard(h).await?;
            let before = rank_mutation_snapshot(h).await?;
            assert_eq!(
                result(rank_apply(h, rank_request(1, 91, 1, 2), &pre_delivery(&h.root)).await?)
                    .result,
                GuildResult::GuildDeadlinePassed
            );
            assert_eq!(rank_mutation_snapshot(h).await?, before);
            assert_eq!(
                result(rank_apply(h, rank_request(2, 100, 1, 3), &pre_delivery(&h.root)).await?)
                    .result,
                GuildResult::Ok
            );
            let deficit: Option<i64> =
                sqlx::query_scalar("SELECT vice_deficit_since FROM game_guilds")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(deficit, Some(1));
            Ok(())
        })?;
    }
    Ok(())
}
#[test]
fn guild_rank_native_replay_conflicts_cas_and_restart() -> TestResult {
    text_run("guild_rank_replay", async |h| {
        rank_fixture(h, 0).await?;
        guard(h).await?;
        let req = rank_request(1, 91, 1, 2);
        let first = result(rank_apply(h, req.clone(), &pre_delivery(&h.root)).await?);
        assert_eq!(first.result, GuildResult::Ok);
        // A later demotion changes target rank/current Premium and revision before historical replay.
        assert_eq!(
            result(rank_apply(h, rank_request(2, 91, 2, 3), &pre_delivery(&h.root)).await?).result,
            GuildResult::Ok
        );
        let before = text_snapshot(h).await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = other.open_character_authority(&seal).await.map_err(debug)?;
        let closed = SpellPremiumCoordinator::new(Arc::default(), None, None, None);
        let mut stale = fence()?;
        stale.connection_generation = ConnectionGeneration::new(999)?;
        let replay = other
            .commit_guild_rank(&authority, &h.node, stale, req.clone(), &closed)
            .await
            .map_err(debug)?;
        assert!(matches!(&replay, GuildOutcome::Replayed(_)));
        assert_eq!(result(replay), first);
        assert_eq!(text_snapshot(h).await?, before);
        for field in 0..6 {
            let mut changed = req.clone();
            match field {
                0 => changed.level = GuildRank::new(3).unwrap(),
                1 => changed.expected_revision = 2,
                2 => changed.common.operation = GuildCommand::SetRank(character(100)),
                3 => changed.common.guild_id = gid(2),
                4 => changed.common.operation = GuildCommand::SetTitle(character(91)),
                _ => {
                    changed.common.command =
                        request(10, 1, GuildCommand::SetRank(character(91))).command
                }
            }
            assert!(matches!(
                other
                    .commit_guild_rank(&authority, &h.node, fence()?, changed, &closed)
                    .await,
                Err(GuildError::ConflictingOccurrence)
            ));
            assert_eq!(text_snapshot(h).await?, before);
        }
        let mut reused = rank_request(3, 91, 3, 2);
        reused.common.command = req.common.command;
        assert!(matches!(
            other
                .commit_guild_rank(&authority, &h.node, fence()?, reused, &closed)
                .await,
            Err(GuildError::ConflictingOccurrence)
        ));
        drop(authority);
        drop(other);
        let before = rank_mutation_snapshot(h).await?;
        assert_eq!(
            result(rank_apply(h, rank_request(4, 91, 1, 2), &pre_delivery(&h.root)).await?).result,
            GuildResult::StaleRevision
        );
        assert_eq!(rank_mutation_snapshot(h).await?, before);
        assert_eq!(h.count("game_guild_operations").await?, 3);
        Ok(())
    })?;
    text_run("guild_rank_overflow", async |h| {
        rank_fixture(h, 0).await?;
        sqlx::query("UPDATE game_guilds SET revision=18446744073709551615")
            .execute(&h.pool)
            .await?;
        guard(h).await?;
        let before = text_snapshot(h).await?;
        assert!(
            rank_apply(h, rank_request(1, 91, u64::MAX, 2), &pre_delivery(&h.root))
                .await
                .is_err()
        );
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}
#[test]
fn guild_rank_native_two_roots_revision_and_occurrence_serialization() -> TestResult {
    text_run("guild_rank_revision_race", async |h| {
        rank_fixture(h, 0).await?;
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
        let premium = pre_delivery(&h.root);
        let second = pre_delivery(&other);
        let (x, y) = fixture::join_two(
            h.root
                .commit_guild_rank(&a, &h.node, fence()?, rank_request(1, 91, 1, 4), &premium),
            other.commit_guild_rank(&b, &h.node, fence()?, rank_request(2, 91, 1, 4), &second),
        )
        .await;
        let results = [
            result(x.map_err(debug)?).result,
            result(y.map_err(debug)?).result,
        ];
        assert_eq!(results.iter().filter(|r| **r == GuildResult::Ok).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|r| **r == GuildResult::StaleRevision)
                .count(),
            1
        );
        let same = rank_request(3, 91, 2, 3);
        let (x, y) = fixture::join_two(
            h.root
                .commit_guild_rank(&a, &h.node, fence()?, same.clone(), &premium),
            other.commit_guild_rank(&b, &h.node, fence()?, same, &second),
        )
        .await;
        let x = x.map_err(debug)?;
        let y = y.map_err(debug)?;
        assert_eq!(
            usize::from(matches!(&x, GuildOutcome::Committed(_)))
                + usize::from(matches!(&y, GuildOutcome::Committed(_))),
            1
        );
        assert_eq!(result(x), result(y));
        assert_eq!(revision(h, 1).await?, "3");
        assert_eq!(h.count("game_guild_operations").await?, 3);
        assert_eq!(h.count("game_guild_event_outbox").await?, 2);
        // Native admission relation locking also participates; no isolated throughput claim.
        Ok(())
    })
}
#[test]
fn guild_rank_native_global_target_account_serialization() -> TestResult {
    text_run("guild_rank_account_race", async |h| {
        historical_character(h, 90, id(WORLD)).await?;
        for c in [91, 92] {
            historical_character_account(h, c, id(WORLD), account(777)).await?;
        }
        initial_guild(h, 1, 0).await?;
        initial_guild(h, 2, 90).await?;
        initial_member(h, 1, 91, 3).await?;
        initial_member(h, 2, 92, 3).await?;
        let actor_b = historical_session(h, 90).await?;
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
        let first = rank_request(1, 91, 1, 2);
        let mut next =
            GuildRankRequest::new(request(2, 2, GuildCommand::SetRank(character(92))), 1, 2)
                .map_err(debug)?;
        next.common.command = CommandRef::new(actor_b.game_session_id, CommandId::new(2)?);
        let pa = pre_delivery(&h.root);
        let pb = pre_delivery(&other);
        let (x, y) = fixture::join_two(
            h.root.commit_guild_rank(&a, &h.node, fence()?, first, &pa),
            other.commit_guild_rank(&b, &h.node, actor_b, next, &pb),
        )
        .await;
        let results = [
            result(x.map_err(debug)?).result,
            result(y.map_err(debug)?).result,
        ];
        assert_eq!(results.iter().filter(|r| **r == GuildResult::Ok).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|r| **r == GuildResult::AccountHasPosition)
                .count(),
            1
        );
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_guild_leadership WHERE account_id=encode($1,'hex')::uuid")
            .bind(account(777).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(count, 1);
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        assert_eq!(h.count("game_guild_operations").await?, 2);
        Ok(())
    })
}
#[test]
fn guild_rank_native_savepoint_atomicity_and_closed_sql() -> TestResult {
    text_run("guild_rank_rollback", async |h| {
        rank_fixture(h, 0).await?;
        guard(h).await?;
        let before = text_snapshot(h).await?;
        let premium = pre_delivery(&h.root);
        let req = rank_request(1, 91, 1, 2);
        // Faults happen after receipt/event creation and must roll back that entire savepoint,
        // even though the genuine NotActivated gate commits its enclosing physical transaction.
        for (trigger, cleanup) in [
            (
                "CREATE FUNCTION guild_rank_test_corrupt() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.title='tampered';RETURN NEW;END$$;CREATE TRIGGER aaa_guild_rank_test_corrupt BEFORE UPDATE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_rank_test_corrupt();",
                "DROP TRIGGER aaa_guild_rank_test_corrupt ON game_guild_members;DROP FUNCTION guild_rank_test_corrupt();",
            ),
            (
                "CREATE FUNCTION guild_rank_test_corrupt() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.premium_deficit_since=99;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_rank_test_corrupt BEFORE UPDATE ON game_guilds FOR EACH ROW EXECUTE FUNCTION guild_rank_test_corrupt();",
                "DROP TRIGGER aaa_guild_rank_test_corrupt ON game_guilds;DROP FUNCTION guild_rank_test_corrupt();",
            ),
            (
                "CREATE FUNCTION guild_rank_test_corrupt() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN RETURN NULL;END$$;CREATE TRIGGER aaa_guild_rank_test_corrupt BEFORE INSERT ON game_account_guild_leadership FOR EACH ROW EXECUTE FUNCTION guild_rank_test_corrupt();",
                "DROP TRIGGER aaa_guild_rank_test_corrupt ON game_account_guild_leadership;DROP FUNCTION guild_rank_test_corrupt();",
            ),
        ] {
            h.pool.execute(trigger).await?;
            assert!(rank_apply(h, req.clone(), &premium).await.is_err());
            assert_eq!(text_snapshot(h).await?, before);
            assert_eq!(h.count("game_premium_activation_latch").await?, 0);
            h.pool.execute(cleanup).await?;
        }
        // No receipt survived; the exact command now commits rather than replaying.
        let outcome = rank_apply(h, req.clone(), &premium).await?;
        assert!(matches!(&outcome, GuildOutcome::Committed(_)));
        assert_eq!(result(outcome).result, GuildResult::Ok);
        let before = text_snapshot(h).await?;
        assert!(sqlx::query("UPDATE game_guild_members SET rank_level=4 WHERE character_id=encode($1,'hex')::uuid")
            .bind(character(91).as_bytes().as_slice()).execute(&h.pool).await.is_err());
        assert!(
            sqlx::query(
                "DELETE FROM game_account_guild_leadership WHERE account_id=encode($1,'hex')::uuid"
            )
            .bind(account(91).as_slice())
            .execute(&h.pool)
            .await
            .is_err()
        );
        assert!(
            sqlx::query("UPDATE game_guild_operations SET edit_rank=4")
                .execute(&h.pool)
                .await
                .is_err()
        );
        assert!(sqlx::query("UPDATE game_guild_event_outbox SET payload=convert_to('{}','UTF8'),payload_sha256=sha256(convert_to('{}','UTF8'))")
            .execute(&h.pool).await.is_err());
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })?;
    text_run("guild_rank_current_rollback", async |h| {
        rank_fixture(h, 0).await?;
        guard(h).await?;
        let (current, _) = current_coordinator(h, account(91), true).await?;
        let before = text_snapshot(h).await?;
        h.pool.execute("CREATE FUNCTION guild_rank_test_current_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.title='tampered';RETURN NEW;END$$;CREATE TRIGGER aaa_guild_rank_test_current_fault BEFORE UPDATE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_rank_test_current_fault();").await?;
        let req = rank_request(1, 91, 1, 2);
        assert!(rank_apply(h, req.clone(), &current).await.is_err());
        assert_eq!(text_snapshot(h).await?, before);
        h.pool.execute("DROP TRIGGER aaa_guild_rank_test_current_fault ON game_guild_members;DROP FUNCTION guild_rank_test_current_fault();").await?;
        assert!(matches!(
            rank_apply(h, req, &current).await?,
            GuildOutcome::Committed(_)
        ));
        assert_eq!(rank_level(h, 91).await?, 2);
        assert_eq!(h.count("game_guild_operations").await?, 1);
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        Ok(())
    })
}
#[test]
fn guild_rank_native_fresh_session_node_recovery_and_writer_recheck() -> TestResult {
    text_run("guild_rank_currentness", async |h| {
        rank_fixture(h, 0).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let premium = pre_delivery(&h.root);
        let req = rank_request(1, 91, 1, 2);
        let before = text_snapshot(h).await?;
        let wrong_node = fixture::register(&h.root, 2).await?;
        for field in 0..5 {
            let mut stale = fence()?;
            match field {
                0 => stale.connection_generation = ConnectionGeneration::new(2)?,
                1 => stale.character_lease_generation = 2,
                2 => stale.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
                3 => stale.character_id = character(999),
                _ => {}
            }
            let node = if field == 4 { &wrong_node } else { &h.node };
            assert!(matches!(
                h.root
                    .commit_guild_rank(&authority, node, stale, req.clone(), &premium)
                    .await,
                Err(GuildError::NotCurrent)
            ));
            assert_eq!(text_snapshot(h).await?, before);
        }
        let recovery = authority.record_for(&h.root).map_err(debug)?;
        let mut stale_recovery = recovery.clone();
        stale_recovery.recovery_generation += 1;
        let write = RankWrite {
            node: h.node.clone(),
            fence: fence()?,
            binding: rank_binding(&fence()?, &req).map_err(debug)?,
            rank: req.clone(),
            account: Some(account(91)),
        };
        let mut tx = h.pool.begin().await?;
        assert!(write.execute(&mut tx, &stale_recovery, None).await.is_err());
        tx.rollback().await?;
        // Revalidation refuses a changed native target Account selector in the real writer too.
        let wrong_selector = RankWrite {
            account: Some(account(92)),
            ..write
        };
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            wrong_selector.execute(&mut tx, &recovery, None).await,
            Err(GuildError::NotCurrent)
        ));
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        assert_eq!(
            result(
                h.root
                    .commit_guild_rank(&authority, &h.node, fence()?, req.clone(), &premium)
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        // Recovery is checked before even a historical exact replay.
        let replay_write = RankWrite {
            node: h.node.clone(),
            fence: fence()?,
            binding: rank_binding(&fence()?, &req).map_err(debug)?,
            rank: req.clone(),
            account: None,
        };
        let before = text_snapshot(h).await?;
        let mut tx = h.pool.begin().await?;
        assert!(
            replay_write
                .execute(&mut tx, &stale_recovery, None)
                .await
                .is_err()
        );
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        use crate::durability::runtime_scope_assignment::{
            AssignmentCommand, AssignmentOutcome, AssignmentRequest, AssignmentState, ControlActor,
            OperationKey, RuntimeScopeAssignmentWriter,
        };
        let actor: String = sqlx::query_scalar("SELECT session_user")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(actor, "oteryn_test_admin");
        // Exact operation3 grant belongs only to this native test principal/scope; no fake caller.
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,3)")
            .bind(id(WORLD).as_slice()).bind(id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
        let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "item-transfer-writer")
            .await
            .map_err(debug)?;
        let predecessor = h
            .root
            .read_runtime_scope_predecessor(fence()?.runtime_scope)
            .await
            .map_err(debug)?
            .ok_or("missing assignment")?;
        let revoked = writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([88; 32]),
                actor: ControlActor::new(&actor).map_err(debug)?,
                command: AssignmentCommand::Revoke {
                    scope: fence()?.runtime_scope,
                    predecessor,
                },
            })
            .await
            .map_err(debug)?;
        assert!(matches!(revoked, AssignmentOutcome::Committed(_)));
        let current = h
            .root
            .read_runtime_scope_assignment(fence()?.runtime_scope)
            .await
            .map_err(debug)?
            .ok_or("missing revoked assignment")?;
        assert_eq!(current.state, AssignmentState::Revoked);
        assert_eq!(current.ownership_generation, 2);
        let fresh = rank_request(2, 91, 2, 3);
        assert!(matches!(
            h.root
                .commit_guild_rank(&authority, &h.node, fence()?, fresh.clone(), &premium)
                .await,
            Err(GuildError::NotCurrent)
        ));
        let revoked_write = RankWrite {
            node: h.node.clone(),
            fence: fence()?,
            binding: rank_binding(&fence()?, &fresh).map_err(debug)?,
            rank: fresh,
            account: Some(account(91)),
        };
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            revoked_write.execute(&mut tx, &recovery, None).await,
            Err(GuildError::NotCurrent)
        ));
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        assert!(matches!(
            h.root
                .commit_guild_rank(&authority, &h.node, fence()?, req, &premium)
                .await
                .map_err(debug)?,
            GuildOutcome::Replayed(_)
        ));
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}
#[test]
fn guild_rank_native_real_found_invite_accept_feed_through() -> TestResult {
    text_run("guild_rank_real_formation", async |h| {
        historical_character(h, 91, id(WORLD)).await?;
        let target_fence = historical_session(h, 91).await?;
        guard(h).await?;
        let premium = pre_delivery(&h.root);
        assert_eq!(
            result(
                found(
                    h,
                    found_request(1, 1, "Rank Native Guild"),
                    fence()?,
                    &premium
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        assert_eq!(
            result(
                apply(
                    h,
                    request(2, 1, GuildCommand::Invite(character(91))),
                    fence()?
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        let mut accept = request(3, 1, GuildCommand::Accept);
        accept.command = CommandRef::new(target_fence.game_session_id, CommandId::new(3)?);
        assert_eq!(
            result(apply(h, accept, target_fence).await?).result,
            GuildResult::Ok
        );
        let (target_premium, _) = current_coordinator(h, account(91), true).await?;
        assert_eq!(
            result(rank_apply(h, rank_request(4, 91, 3, 2), &target_premium).await?).result,
            GuildResult::Ok
        );
        assert_eq!(rank_level(h, 91).await?, 2);
        assert_eq!(revision(h, 1).await?, "4");
        assert_eq!(h.count("game_guild_event_outbox").await?, 4);
        assert_eq!(h.count("game_account_guild_leadership").await?, 2);
        assert_eq!(h.count("game_guild_invitations").await?, 0);
        let state: i16 = sqlx::query_scalar("SELECT state FROM game_guilds")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(state, 1);
        assert_eq!(text_character_revision(h).await?, "1");
        Ok(())
    })
}
#[test]
fn guild_rank_native_exact_database_transaction_deadline_boundaries() -> TestResult {
    // The only exact clock here is PostgreSQL's physical transaction clock. Historical deadline
    // setup happens before installing the private guards, inside the real NotActivated gate TX.
    // RankWrite then reuses that genuine borrowed TX and genuine recovery/node/session records.
    // The public entry's real selector/Premium binding is separately covered above.
    for state in [1_i16, 2] {
        for delta in [0_i64, -1] {
            text_run("guild_rank_exact_deadline", async |h| {
                rank_fixture(h, 3).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let recovery = authority.record_for(&h.root).map_err(debug)?;
                let req = rank_request(1, 91, 1, 2);
                let write = RankWrite {
                    node: h.node.clone(),
                    fence: fence()?,
                    binding: rank_binding(&fence()?, &req).map_err(debug)?,
                    rank: req,
                    account: Some(account(91)),
                };
                let applied=pre_delivery(&h.root).with_premium_gate(account(91),move |status,tx| {
                    Box::pin(async move {
                        if !matches!(&status,PremiumStatus::NotActivated(_)) {return Err(invalid());}
                        let tx=tx.ok_or_else(invalid)?;
                        let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM transaction_timestamp())*1000)::bigint").fetch_one(&mut **tx).await?;
                        sqlx::query("UPDATE game_guilds SET state=$1,formation_deadline=$2,vice_deficit_since=$3")
                            .bind(state).bind(now+delta).bind(if state==2 {Some(now-1_209_600_000+delta)}else{None}).execute(&mut **tx).await?;
                        (&mut **tx).execute(*GUILD_GUARDS_PROPOSAL).await?;
                        let outcome=write.execute(tx,&recovery,Some(&status)).await?;
                        Ok::<_,GuildError>((outcome,now))
                    })
                }).await.map_err(debug)?.map_err(debug)?;
                assert_eq!(
                    result(applied.0).result,
                    if delta == 0 {
                        GuildResult::Ok
                    } else {
                        GuildResult::GuildDeadlinePassed
                    }
                );
                assert_eq!(revision(h, 1).await?, if delta == 0 { "2" } else { "1" });
                assert_eq!(rank_level(h, 91).await?, if delta == 0 { 2 } else { 3 });
                assert_eq!(h.count("game_guild_operations").await?, 1);
                assert_eq!(
                    h.count("game_guild_event_outbox").await?,
                    i64::from(delta == 0)
                );
                let row: (i16, Option<i64>, Option<i64>) = sqlx::query_as(
                    "SELECT state,vice_deficit_since,premium_deficit_since FROM game_guilds",
                )
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(row.0, if delta == 0 { 2 } else { state });
                assert_eq!(
                    row.1,
                    if delta == 0 || state == 1 {
                        None
                    } else {
                        Some(applied.1 - 1_209_600_000 - 1)
                    }
                );
                assert_eq!(row.2, Some(7));
                assert_eq!(text_character_revision(h).await?, "1");
                Ok(())
            })?;
        }
    }
    Ok(())
}
#[test]
fn guild_rank_native_shared_gate_commits_before_activation_latch() -> TestResult {
    text_run("guild_rank_gate_latch", async |h| {
        rank_fixture(h, 0).await?;
        guard(h).await?;
        h.pool.execute("CREATE FUNCTION guild_rank_test_barrier() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN PERFORM pg_advisory_xact_lock(901123);RETURN NEW;END$$;CREATE TRIGGER aaa_guild_rank_test_barrier BEFORE UPDATE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_rank_test_barrier();").await?;
        let mut barrier = h.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(901123)")
            .execute(&mut *barrier)
            .await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let premium = pre_delivery(&h.root);
        let req = rank_request(1, 91, 1, 2);
        let (promoted, latched) = fixture::join_two(
            h.root
                .commit_guild_rank(&authority, &h.node, fence()?, req.clone(), &premium),
            async {
                wait_advisory(&h.pool, 901123, false).await?;
                let latch_key =
                    crate::durability::premium_activation_latch::PREMIUM_ACTIVATION_LOCK_KEY;
                wait_advisory(&h.pool, latch_key, true).await?;
                let (latched, released) = fixture::join_two(
                    other.latch_premium_activation("guild-rank-racing-activation", 1, 2),
                    async {
                        wait_advisory(&h.pool, latch_key, false).await?;
                        barrier.rollback().await?;
                        Ok::<_, Box<dyn std::error::Error>>(())
                    },
                )
                .await;
                released?;
                latched.map_err(|e| -> Box<dyn std::error::Error> { debug(e).into() })
            },
        )
        .await;
        assert_eq!(result(promoted.map_err(debug)?).result, GuildResult::Ok);
        assert_eq!(latched?.activation_id, "guild-rank-racing-activation");
        h.pool.execute("DROP TRIGGER aaa_guild_rank_test_barrier ON game_guild_members;DROP FUNCTION guild_rank_test_barrier();").await?;
        // After delivery, demotion still requires no fresh Premium grant. Re-promotion does.
        assert_eq!(
            result(
                h.root
                    .commit_guild_rank(
                        &authority,
                        &h.node,
                        fence()?,
                        rank_request(2, 91, 2, 3),
                        &pre_delivery(&h.root)
                    )
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        let before = rank_mutation_snapshot(h).await?;
        assert_eq!(
            result(
                h.root
                    .commit_guild_rank(
                        &authority,
                        &h.node,
                        fence()?,
                        rank_request(3, 91, 3, 2),
                        &pre_delivery(&h.root)
                    )
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::NotPremium
        );
        assert_eq!(rank_mutation_snapshot(h).await?, before);
        assert!(matches!(
            h.root
                .commit_guild_rank(&authority, &h.node, fence()?, req, &pre_delivery(&h.root))
                .await
                .map_err(debug)?,
            GuildOutcome::Replayed(_)
        ));
        assert_eq!(h.count("game_guild_event_outbox").await?, 2);
        assert_eq!(h.count("game_guild_operations").await?, 3);
        Ok(())
    })
}
