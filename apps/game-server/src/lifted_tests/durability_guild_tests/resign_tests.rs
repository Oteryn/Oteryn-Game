use super::*;
#[test]
fn guild_resign_native_request_binding_and_closed_audit() -> TestResult {
    assert!(
        GuildResignRequest::new(request(1, 1, GuildCommand::Resign(character(91))), 0).is_err()
    );
    assert!(
        GuildResignRequest::new(request(1, 1, GuildCommand::SetRank(character(91))), 1).is_err()
    );
    let original = resign_request(1, 91, 1);
    let binding = resign_binding(&fence()?, &original).map_err(debug)?;
    let mut changes = vec![resign_request(1, 92, 1), resign_request(1, 91, 2)];
    let mut changed = original.clone();
    changed.common.guild_id = gid(2);
    changes.push(changed);
    let mut changed = original.clone();
    changed.common.operation = GuildCommand::SetRank(character(91));
    changes.push(changed);
    let mut changed = original.clone();
    changed.common.command = request(2, 1, GuildCommand::Resign(character(91))).command;
    changes.push(changed);
    let mut changed = original.clone();
    changed.common.identity.transaction_id = sid(9, 11);
    changes.push(changed);
    for changed in changes {
        assert_ne!(resign_binding(&fence()?, &changed).map_err(debug)?, binding);
    }
    let mut actor_data = fence()?;
    actor_data.character_id = character(92);
    assert_ne!(
        resign_binding(&actor_data, &original).map_err(debug)?,
        binding
    );
    let audit = GuildResignAuditProposal {
        operation: GuildAuditProposal {
            version: 1,
            occurrence: sid(1, 10),
            transaction_id: sid(1, 11),
            event_id: sid(1, 12),
            guild_id: *gid(1).as_bytes(),
            world_id: id(WORLD),
            actor: id(CHARACTER),
            target: *character(91).as_bytes(),
            action: 10,
            previous_revision: 1,
            revision: 2,
            rank_before: Some(2),
            rank_after: Some(1),
            removed_invitations: 0,
            occurred_at_unix_ms: 1,
        },
        expected_revision: 1,
    };
    let bytes = audit.checked_bytes().ok_or("valid Resign audit refused")?;
    assert_eq!(
        serde_json::from_slice::<GuildResignAuditProposal>(&bytes)?,
        audit
    );
    for field in 0..6 {
        let mut wrong = audit.clone();
        match field {
            0 => wrong.expected_revision = 2,
            1 => wrong.operation.rank_before = Some(1),
            2 => wrong.operation.rank_after = Some(2),
            3 => wrong.operation.actor = wrong.operation.target,
            4 => wrong.operation.action = 9,
            _ => wrong.operation.revision = 3,
        }
        assert!(wrong.checked_bytes().is_none());
    }
    let mut json: serde_json::Value = serde_json::from_slice(&bytes)?;
    json["caller_target_premium"] = true.into();
    assert!(serde_json::from_value::<GuildResignAuditProposal>(json).is_err());
    Ok(())
}
#[test]
fn guild_resign_native_legacy_payloadless_entry_refuses_before_guild_lookup() -> TestResult {
    text_run("guild_resign_legacy", async |h| {
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let before = text_snapshot(h).await?;
        assert!(matches!(
            h.root
                .commit_guild_membership(
                    &authority,
                    &h.node,
                    fence()?,
                    request(1, 999, GuildCommand::Resign(character(91)))
                )
                .await,
            Err(GuildError::InvalidInput)
        ));
        assert_eq!(text_snapshot(h).await?, before);
        assert_eq!(h.count("game_guild_operations").await?, 0);
        assert_eq!(h.count("game_guild_event_outbox").await?, 0);
        Ok(())
    })
}
#[test]
fn guild_resign_native_both_actual_account_premium_matrix() -> TestResult {
    // 0 bothCurrent; 1 onlyactor; 2 onlytarget; 3 bothNotCurrent; 4 noClock;
    // 5 actorPullFailed; 6 targetPullFailed; 7 missinggate; 8 genuineNotActivated.
    for case in 0..9 {
        text_run("guild_resign_premium", async |h| {
            resign_fixture(h).await?;
            guard(h).await?;
            let (premium, consumer) = match case {
                0 | 5 | 6 => resign_current(h).await?,
                1 | 3 => current_coordinator(h, account(0), true).await?,
                2 => current_coordinator(h, account(91), true).await?,
                4 => {
                    let pair = current_coordinator(h, account(0), false).await?;
                    resign_resource_seed_current(h, &pair.1, account(91)).await?;
                    pair
                }
                7 => {
                    let coordinator =
                        SpellPremiumCoordinator::new(Arc::default(), None, None, None);
                    let consumer = coordinator.consumer();
                    (coordinator, consumer)
                }
                _ => {
                    let coordinator = pre_delivery(&h.root);
                    let consumer = coordinator.consumer();
                    (coordinator, consumer)
                }
            };
            if case == 3 || case == 5 {
                consumer.pull_failed(account(0), consumer.ticket());
            }
            if case == 6 {
                consumer.pull_failed(account(91), consumer.ticket());
            }
            let before = rank_mutation_snapshot(h).await?;
            let unchanged = resign_resource_unchanged_fields(h).await?;
            let revisions = resign_resource_character_revisions(h).await?;
            let outcome = result(resign_apply(h, resign_request(1, 91, 1), &premium).await?);
            let success = case == 0 || case == 8;
            assert_eq!(
                outcome.result,
                if success {
                    GuildResult::Ok
                } else {
                    GuildResult::NotPremium
                }
            );
            assert_eq!(h.count("game_guild_operations").await?, 1);
            assert_eq!(
                h.count("game_guild_event_outbox").await?,
                i64::from(success)
            );
            assert_eq!(h.count("game_account_guild_leadership").await?, 2);
            assert_eq!(resign_resource_character_revisions(h).await?, revisions);
            if success {
                assert_eq!(rank_level(h, 0).await?, 2);
                assert_eq!(rank_level(h, 91).await?, 1);
                assert_eq!(revision(h, 1).await?, "2");
                assert_eq!(resign_resource_unchanged_fields(h).await?, unchanged);
                if case == 8 {
                    assert_eq!(h.count("game_premium_activation_latch").await?, 0);
                }
            } else {
                assert_eq!(rank_mutation_snapshot(h).await?, before);
                assert_eq!(revision(h, 1).await?, "1");
            }
            Ok(())
        })?;
    }
    Ok(())
}
#[test]
fn guild_resign_native_current_roles_target_and_world_refusals() -> TestResult {
    for (actor_rank, target_rank, expected_revision) in
        [(2, 2, 1), (3, 2, 1), (1, 3, 1), (2, 2, 99)]
    {
        text_run("guild_resign_roles", async |h| {
            historical_character(h, 90, id(WORLD)).await?;
            historical_character(h, 91, id(WORLD)).await?;
            initial_guild(h, 1, if actor_rank == 1 { 0 } else { 90 }).await?;
            if actor_rank != 1 {
                initial_member(h, 1, 0, actor_rank).await?;
            }
            initial_member(h, 1, 91, target_rank).await?;
            guard(h).await?;
            let before = rank_mutation_snapshot(h).await?;
            assert_eq!(
                result(
                    resign_apply(
                        h,
                        resign_request(1, 91, expected_revision),
                        &pre_delivery(&h.root)
                    )
                    .await?
                )
                .result,
                GuildResult::NotAllowed
            );
            assert_eq!(rank_mutation_snapshot(h).await?, before);
            assert_eq!(h.count("game_guild_operations").await?, 1);
            assert_eq!(h.count("game_guild_event_outbox").await?, 0);
            Ok(())
        })?;
    }
    text_run("guild_resign_target_scope", async |h| {
        historical_character(h, 90, id(WORLD)).await?;
        resign_fixture(h).await?;
        initial_guild(h, 2, 90).await?;
        historical_character(h, 92, id(WORLD)).await?;
        initial_member(h, 2, 92, 2).await?;
        historical_character(h, 93, sid(900, 40)).await?;
        guard(h).await?;
        let before = rank_mutation_snapshot(h).await?;
        for (n, target) in [(1, 0), (2, 92), (3, 93), (4, 999)] {
            assert_eq!(
                result(
                    resign_apply(h, resign_request(n, target, 1), &pre_delivery(&h.root)).await?
                )
                .result,
                GuildResult::NotAllowed
            );
            assert_eq!(rank_mutation_snapshot(h).await?, before);
            assert_eq!(h.count("game_guild_operations").await?, i64::from(n));
            assert_eq!(h.count("game_guild_event_outbox").await?, 0);
        }
        Ok(())
    })
}
#[test]
fn guild_resign_native_keeps_lifecycle_and_refuses_terminal_states() -> TestResult {
    for state in [1_i16, 2, 3, 4] {
        text_run("guild_resign_lifecycle", async |h| {
            resign_fixture(h).await?;
            sqlx::query("UPDATE game_guilds SET state=$1,formation_deadline=1,vice_deficit_since=$2,premium_deficit_since=$3")
                .bind(state).bind(if state == 1 { None } else { Some(1_i64) })
                .bind(if state == 1 { None } else { Some(7_i64) }).execute(&h.pool).await?;
            if state == 4 {
                // A terminal historical fixture has no remaining members/leadership rows.
                h.pool
                    .execute(
                        "DELETE FROM game_account_guild_leadership;DELETE FROM game_guild_members;",
                    )
                    .await?;
            }
            guard(h).await?;
            let before = rank_mutation_snapshot(h).await?;
            let unchanged = resign_resource_unchanged_fields(h).await?;
            let revisions = resign_resource_character_revisions(h).await?;
            let outcome =
                result(resign_apply(h, resign_request(1, 91, 1), &pre_delivery(&h.root)).await?);
            assert_eq!(
                outcome.result,
                if state <= 2 {
                    GuildResult::Ok
                } else {
                    GuildResult::GuildDisbanding
                }
            );
            assert_eq!(h.count("game_guild_operations").await?, 1);
            assert_eq!(
                h.count("game_guild_event_outbox").await?,
                i64::from(state <= 2)
            );
            assert_eq!(resign_resource_character_revisions(h).await?, revisions);
            if state <= 2 {
                assert_eq!(resign_resource_unchanged_fields(h).await?, unchanged);
                assert_eq!(rank_level(h, 0).await?, 2);
                assert_eq!(rank_level(h, 91).await?, 1);
            } else {
                assert_eq!(rank_mutation_snapshot(h).await?, before);
            }
            Ok(())
        })?;
    }
    Ok(())
}
#[test]
fn guild_resign_native_cas_and_overflow_leave_no_partial_swap() -> TestResult {
    text_run("guild_resign_stale", async |h| {
        resign_fixture(h).await?;
        guard(h).await?;
        let before = rank_mutation_snapshot(h).await?;
        assert_eq!(
            result(resign_apply(h, resign_request(1, 91, 2), &pre_delivery(&h.root)).await?).result,
            GuildResult::StaleRevision
        );
        assert_eq!(rank_mutation_snapshot(h).await?, before);
        assert_eq!(h.count("game_guild_operations").await?, 1);
        assert_eq!(h.count("game_guild_event_outbox").await?, 0);
        Ok(())
    })?;
    text_run("guild_resign_overflow", async |h| {
        resign_fixture(h).await?;
        h.pool
            .execute("UPDATE game_guilds SET revision=18446744073709551615")
            .await?;
        guard(h).await?;
        let before = text_snapshot(h).await?;
        assert!(
            resign_apply(h, resign_request(1, 91, u64::MAX), &pre_delivery(&h.root))
                .await
                .is_err()
        );
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}
#[test]
fn guild_resign_native_committed_and_refusal_replay_survive_role_changes() -> TestResult {
    text_run("guild_resign_replay", async |h| {
        let target_fence = resign_fixture(h).await?;
        guard(h).await?;
        let premium = pre_delivery(&h.root);
        let first_req = resign_request(1, 91, 1);
        let first = result(resign_apply(h, first_req.clone(), &premium).await?);
        assert_eq!(first.result, GuildResult::Ok);
        let refused_req = resign_request(2, 91, 2);
        let refused = result(resign_apply(h, refused_req.clone(), &premium).await?);
        assert_eq!(refused.result, GuildResult::NotAllowed);
        let mut reverse =
            GuildResignRequest::new(request(3, 1, GuildCommand::Resign(character(0))), 2)
                .map_err(debug)?;
        reverse.common.command = CommandRef::new(target_fence.game_session_id, CommandId::new(3)?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            result(
                h.root
                    .commit_guild_resign(&authority, &h.node, target_fence, reverse, &premium)
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        drop(authority);
        assert_eq!(rank_level(h, 0).await?, 1);
        assert_eq!(rank_level(h, 91).await?, 2);
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = other.open_character_authority(&seal).await.map_err(debug)?;
        let closed = SpellPremiumCoordinator::new(Arc::default(), None, None, None);
        let mut stale = fence()?;
        stale.connection_generation = ConnectionGeneration::new(999)?;
        let before = text_snapshot(h).await?;
        for (req, receipt) in [(first_req.clone(), first), (refused_req.clone(), refused)] {
            let replay = other
                .commit_guild_resign(&authority, &h.node, stale, req, &closed)
                .await
                .map_err(debug)?;
            assert!(matches!(&replay, GuildOutcome::Replayed(_)));
            assert_eq!(result(replay), receipt);
            assert_eq!(text_snapshot(h).await?, before);
        }
        for field in 0..6 {
            let mut changed = first_req.clone();
            match field {
                0 => changed.expected_revision = 2,
                1 => changed.common.operation = GuildCommand::Resign(character(92)),
                2 => changed.common.guild_id = gid(2),
                3 => changed.common.operation = GuildCommand::SetRank(character(91)),
                4 => changed.common.identity.transaction_id = sid(99, 11),
                _ => {
                    changed.common.command =
                        request(99, 1, GuildCommand::Resign(character(91))).command
                }
            }
            assert!(matches!(
                other
                    .commit_guild_resign(&authority, &h.node, fence()?, changed, &closed)
                    .await,
                Err(GuildError::ConflictingOccurrence)
            ));
            assert_eq!(text_snapshot(h).await?, before);
        }
        let mut reused = resign_request(4, 91, 3);
        reused.common.command = first_req.common.command;
        assert!(matches!(
            other
                .commit_guild_resign(&authority, &h.node, fence()?, reused, &closed)
                .await,
            Err(GuildError::ConflictingOccurrence)
        ));
        assert_eq!(text_snapshot(h).await?, before);
        assert_eq!(h.count("game_guild_operations").await?, 3);
        assert_eq!(h.count("game_guild_event_outbox").await?, 2);
        Ok(())
    })
}
#[test]
fn guild_resign_native_real_formation_and_follow_on_leader_authority() -> TestResult {
    text_run("guild_resign_real_formation", async |h| {
        historical_character(h, 91, id(WORLD)).await?;
        let target_fence = historical_session(h, 91).await?;
        guard(h).await?;
        let pre = pre_delivery(&h.root);
        assert_eq!(
            result(
                found(
                    h,
                    found_request(1, 1, "Resign Native Guild"),
                    fence()?,
                    &pre
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
        let (premium, _) = resign_current(h).await?;
        assert_eq!(
            result(rank_apply(h, rank_request(4, 91, 3, 2), &premium).await?).result,
            GuildResult::Ok
        );
        assert_eq!(
            result(resign_apply(h, resign_request(5, 91, 4), &premium).await?).result,
            GuildResult::Ok
        );
        assert_eq!(rank_level(h, 0).await?, 2);
        assert_eq!(rank_level(h, 91).await?, 1);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut title = text_request(6, GuildCommand::SetTitle(character(0)), 5, "Resigned");
        title.common.command = CommandRef::new(target_fence.game_session_id, CommandId::new(6)?);
        assert_eq!(
            result(
                h.root
                    .commit_guild_text(&authority, &h.node, target_fence, title)
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(7, GuildCommand::SetTitle(character(91)), 6, "Blocked")
                )
                .await?
            )
            .result,
            GuildResult::NotAllowed
        );
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(8, GuildCommand::SetMessage, 6, "New leader; retained vice")
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        assert_eq!(revision(h, 1).await?, "7");
        assert_eq!(h.count("game_guild_operations").await?, 8);
        assert_eq!(h.count("game_guild_event_outbox").await?, 7);
        assert_eq!(h.count("game_account_guild_leadership").await?, 2);
        assert_eq!(h.count("game_guild_invitations").await?, 0);
        let leaders: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_guild_members WHERE rank_level=1")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(leaders, 1);
        let unchanged_revisions: bool =
            sqlx::query_scalar("SELECT bool_and(character_revision=1) FROM game_character_roots")
                .fetch_one(&h.pool)
                .await?;
        assert!(unchanged_revisions);
        Ok(())
    })
}
#[test]
fn guild_resign_native_ordered_swap_sql_binding_and_both_transaction_rollbacks() -> TestResult {
    for delivered in [false, true] {
        text_run("guild_resign_rollback", async |h| {
            let _ = resign_fixture(h).await?;
            guard(h).await?;
            let (current, _) = resign_current(h).await?;
            let before = text_snapshot(h).await?;
            // delivered=false still needs a genuinely empty activation latch. Creating Current
            // snapshots alone does not decide the gate until that coordinator is actually used.
            let pre = pre_delivery(&h.root);
            let premium = if delivered { &current } else { &pre };
            let req = resign_request(1, 91, 1);
            for (setup, cleanup) in [
                (
                    "CREATE FUNCTION guild_resign_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN IF OLD.rank_level=1 THEN NEW.title='corrupt';END IF;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_resign_test_fault BEFORE UPDATE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_resign_test_fault();",
                    "DROP TRIGGER aaa_guild_resign_test_fault ON game_guild_members;DROP FUNCTION guild_resign_test_fault();",
                ),
                (
                    "CREATE FUNCTION guild_resign_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN IF OLD.rank_level=2 AND NEW.rank_level=1 THEN RETURN NULL;END IF;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_resign_test_fault BEFORE UPDATE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_resign_test_fault();",
                    "DROP TRIGGER aaa_guild_resign_test_fault ON game_guild_members;DROP FUNCTION guild_resign_test_fault();",
                ),
                (
                    "CREATE FUNCTION guild_resign_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.vice_deficit_since=NULL;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_resign_test_fault BEFORE UPDATE ON game_guilds FOR EACH ROW EXECUTE FUNCTION guild_resign_test_fault();",
                    "DROP TRIGGER aaa_guild_resign_test_fault ON game_guilds;DROP FUNCTION guild_resign_test_fault();",
                ),
            ] {
                h.pool.execute(setup).await?;
                assert!(resign_apply(h, req.clone(), premium).await.is_err());
                assert_eq!(text_snapshot(h).await?, before);
                assert_eq!(rank_level(h, 0).await?, 1);
                assert_eq!(rank_level(h, 91).await?, 2);
                h.pool.execute(cleanup).await?;
            }
            if !delivered {
                // BEFORE INSERT fixtures corrupt NEW rows before retention. They NEVER mutate
                // immutable receipts/events. Both hashes agree, and all roles/positions swap
                // correctly, so only exact closed action10 payload binding can refuse these.
                event_failure::install_payload(h).await?;
                let valid = resign_test_audit(&req)?;
                let keys = valid
                    .as_object()
                    .ok_or("audit not object")?
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>();
                assert_eq!(keys.len(), 16);
                for key in &keys {
                    for remove in [false, true] {
                        let mut altered = valid.clone();
                        if remove {
                            altered
                                .as_object_mut()
                                .ok_or("audit not object")?
                                .remove(key);
                        } else {
                            // Change exactly one field; all IDs retain genuine UUIDv7 byte shape.
                            altered[key] = match key.as_str() {
                                "actor" | "target" => {
                                    serde_json::to_value(character(999).as_bytes())?
                                }
                                "guild_id" => serde_json::to_value(gid(999).as_bytes())?,
                                "world_id" | "occurrence" | "transaction_id" | "event_id" => {
                                    serde_json::to_value(sid(999, 90))?
                                }
                                _ => serde_json::json!(99),
                            };
                        }
                        event_failure::set_payload(h, &altered).await?;
                        assert!(
                            resign_apply(h, req.clone(), premium).await.is_err(),
                            "unbound audit field {key}, remove={remove}"
                        );
                        assert_eq!(text_snapshot(h).await?, before);
                    }
                }
                let mut extra = valid;
                extra["caller_premium"] = true.into();
                event_failure::set_payload(h, &extra).await?;
                assert!(resign_apply(h, req.clone(), premium).await.is_err());
                assert_eq!(text_snapshot(h).await?, before);
                event_failure::remove_payload(h).await?;
            }
            let outcome = resign_apply(h, req, premium).await?;
            assert!(matches!(&outcome, GuildOutcome::Committed(_)));
            assert_eq!(result(outcome).result, GuildResult::Ok);
            assert_eq!(rank_level(h, 0).await?, 2);
            assert_eq!(rank_level(h, 91).await?, 1);
            assert_eq!(h.count("game_guild_operations").await?, 1);
            assert_eq!(h.count("game_guild_event_outbox").await?, 1);
            let before = text_snapshot(h).await?;
            for sql in [
                "UPDATE game_guild_members SET rank_level=1 WHERE rank_level=2",
                "DELETE FROM game_account_guild_leadership",
                "UPDATE game_guild_operations SET edit_rank=2",
                "UPDATE game_guild_event_outbox SET occurred_at=occurred_at+1",
            ] {
                assert!(h.pool.execute(sql).await.is_err());
                assert_eq!(text_snapshot(h).await?, before);
            }
            Ok(())
        })?;
    }
    Ok(())
}
#[test]
fn guild_resign_native_two_root_occurrence_successor_and_demotion_races() -> TestResult {
    for case in 0..3 {
        text_run("guild_resign_race", async |h| {
            let _ = resign_fixture(h).await?;
            historical_character(h, 92, id(WORLD)).await?;
            initial_member(h, 1, 92, 2).await?;
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
            let pa = pre_delivery(&h.root);
            let pb = pre_delivery(&other);
            let req = resign_request(1, 91, 1);
            let (x, y) = if case < 2 {
                fixture::join_two(
                    h.root
                        .commit_guild_resign(&a, &h.node, fence()?, req.clone(), &pa),
                    other.commit_guild_resign(
                        &b,
                        &h.node,
                        fence()?,
                        if case == 0 {
                            req
                        } else {
                            resign_request(2, 92, 1)
                        },
                        &pb,
                    ),
                )
                .await
            } else {
                fixture::join_two(
                    h.root.commit_guild_resign(&a, &h.node, fence()?, req, &pa),
                    other.commit_guild_rank(&b, &h.node, fence()?, rank_request(2, 91, 1, 3), &pb),
                )
                .await
            };
            let x = x.map_err(debug)?;
            let y = y.map_err(debug)?;
            if case == 0 {
                assert_eq!(
                    usize::from(matches!(&x, GuildOutcome::Committed(_)))
                        + usize::from(matches!(&y, GuildOutcome::Committed(_))),
                    1
                );
                assert_eq!(result(x), result(y));
                assert_eq!(h.count("game_guild_operations").await?, 1);
            } else {
                let outcomes = [result(x).result, result(y).result];
                assert_eq!(
                    outcomes.iter().filter(|x| **x == GuildResult::Ok).count(),
                    1
                );
                assert_eq!(
                    outcomes
                        .iter()
                        .filter(|x| **x == GuildResult::NotAllowed)
                        .count(),
                    1
                );
                assert_eq!(h.count("game_guild_operations").await?, 2);
            }
            let leaders: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_guild_members WHERE rank_level=1")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(leaders, 1);
            assert_eq!(revision(h, 1).await?, "2");
            assert_eq!(h.count("game_guild_event_outbox").await?, 1);
            let incoherent:i64=sqlx::query_scalar("SELECT count(*) FROM game_guild_members m JOIN game_character_roots c USING(character_id) WHERE (m.rank_level<=2) <> EXISTS(SELECT 1 FROM game_account_guild_leadership l WHERE l.account_id=c.account_id AND l.character_id=m.character_id AND l.guild_id=m.guild_id)").fetch_one(&h.pool).await?;
            assert_eq!(incoherent, 0);
            assert_eq!(text_character_revision(h).await?, "1");
            // Native admission locking also serializes whole passes; no isolated throughput claim.
            Ok(())
        })?;
    }
    Ok(())
}
#[test]
fn guild_resign_native_both_fence_boundaries_actual_revoke_and_recovery_first() -> TestResult {
    text_run("guild_resign_fences", async |h| {
        let _ = resign_fixture(h).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let recovery = authority.record_for(&h.root).map_err(debug)?;
        let req = resign_request(1, 91, 1);
        let premium = pre_delivery(&h.root);
        let before = text_snapshot(h).await?;
        let wrong = fixture::register(&h.root, 2).await?;
        for field in 0..5 {
            let mut f = fence()?;
            match field {
                0 => f.connection_generation = ConnectionGeneration::new(2)?,
                1 => f.character_lease_generation = 2,
                2 => f.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
                3 => f.character_id = character(999),
                _ => {}
            }
            let node = if field == 4 { &wrong } else { &h.node };
            assert!(matches!(
                h.root
                    .commit_guild_resign(&authority, node, f, req.clone(), &premium)
                    .await,
                Err(GuildError::NotCurrent)
            ));
            let write = ResignWrite {
                node: node.clone(),
                fence: f,
                resign: req.clone(),
                binding: resign_binding(&f, &req).map_err(debug)?,
                accounts: Some([account(0), account(91)]),
            };
            let mut tx = h.pool.begin().await?;
            assert!(matches!(
                write.execute(&mut tx, &recovery, None).await,
                Err(GuildError::NotCurrent)
            ));
            tx.rollback().await?;
            assert_eq!(text_snapshot(h).await?, before);
        }
        let write = ResignWrite {
            node: h.node.clone(),
            fence: fence()?,
            resign: req.clone(),
            binding: resign_binding(&fence()?, &req).map_err(debug)?,
            accounts: Some([account(0), account(92)]),
        };
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            write.execute(&mut tx, &recovery, None).await,
            Err(GuildError::NotCurrent)
        ));
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        assert!(matches!(
            other
                .commit_guild_resign(&authority, &h.node, fence()?, req.clone(), &premium)
                .await,
            Err(GuildError::NotCurrent)
        ));
        drop(other);
        use crate::durability::runtime_scope_assignment::{
            AssignmentCommand, AssignmentOutcome, AssignmentRequest, AssignmentState, ControlActor,
            OperationKey, RuntimeScopeAssignmentWriter,
        };
        let actor: String = sqlx::query_scalar("SELECT session_user")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(actor, "oteryn_test_admin");
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,3)").bind(id(WORLD).as_slice()).bind(id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
        let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "item-transfer-writer")
            .await
            .map_err(debug)?;
        let predecessor = h
            .root
            .read_runtime_scope_predecessor(fence()?.runtime_scope)
            .await
            .map_err(debug)?
            .ok_or("missing assignment")?;
        assert!(matches!(
            writer
                .submit(&AssignmentRequest {
                    operation_key: OperationKey::from_bytes([89; 32]),
                    actor: ControlActor::new(&actor).map_err(debug)?,
                    command: AssignmentCommand::Revoke {
                        scope: fence()?.runtime_scope,
                        predecessor
                    }
                })
                .await
                .map_err(debug)?,
            AssignmentOutcome::Committed(_)
        ));
        let actual = h
            .root
            .read_runtime_scope_assignment(fence()?.runtime_scope)
            .await
            .map_err(debug)?
            .ok_or("missing revoked assignment")?;
        assert_eq!(actual.state, AssignmentState::Revoked);
        assert_eq!(actual.ownership_generation, 2);
        assert!(matches!(
            h.root
                .commit_guild_resign(&authority, &h.node, fence()?, req.clone(), &premium)
                .await,
            Err(GuildError::NotCurrent)
        ));
        let write = ResignWrite {
            node: h.node.clone(),
            fence: fence()?,
            resign: req.clone(),
            binding: resign_binding(&fence()?, &req).map_err(debug)?,
            accounts: Some([account(0), account(91)]),
        };
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            write.execute(&mut tx, &recovery, None).await,
            Err(GuildError::NotCurrent)
        ));
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        assert_eq!(rank_level(h, 0).await?, 1);
        assert_eq!(rank_level(h, 91).await?, 2);
        Ok(())
    })?;
    text_run("guild_resign_recovery", async |h| {
        let _ = resign_fixture(h).await?;
        guard(h).await?;
        let req = resign_request(1, 91, 1);
        assert_eq!(
            result(resign_apply(h, req.clone(), &pre_delivery(&h.root)).await?).result,
            GuildResult::Ok
        );
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut recovery = authority.record_for(&h.root).map_err(debug)?;
        recovery.recovery_generation += 1;
        let write = ResignWrite {
            node: h.node.clone(),
            fence: fence()?,
            resign: req.clone(),
            binding: resign_binding(&fence()?, &req).map_err(debug)?,
            accounts: None,
        };
        let before = text_snapshot(h).await?;
        let mut tx = h.pool.begin().await?;
        assert!(write.execute(&mut tx, &recovery, None).await.is_err());
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}
#[test]
fn guild_resign_native_batch_gate_commit_precedes_activation_latch() -> TestResult {
    text_run("guild_resign_batch_latch", async |h| {
        let target_fence = resign_fixture(h).await?;
        guard(h).await?;
        h.pool.execute("CREATE FUNCTION guild_resign_test_barrier() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN IF OLD.rank_level=1 THEN PERFORM pg_advisory_xact_lock(901124);END IF;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_resign_test_barrier BEFORE UPDATE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_resign_test_barrier();").await?;
        let mut barrier = h.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(901124)")
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
        let req = resign_request(1, 91, 1);
        let (swapped, latched) = fixture::join_two(
            h.root
                .commit_guild_resign(&authority, &h.node, fence()?, req.clone(), &premium),
            async {
                wait_advisory(&h.pool, 901124, false).await?;
                let latch_key =
                    crate::durability::premium_activation_latch::PREMIUM_ACTIVATION_LOCK_KEY;
                wait_advisory(&h.pool, latch_key, true).await?;
                let (latched, released) = fixture::join_two(
                    other.latch_premium_activation("guild-resign-racing-activation", 1, 2),
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
        assert_eq!(result(swapped.map_err(debug)?).result, GuildResult::Ok);
        assert_eq!(latched?.activation_id, "guild-resign-racing-activation");
        assert_eq!(rank_level(h, 0).await?, 2);
        assert_eq!(rank_level(h, 91).await?, 1);
        h.pool.execute("DROP TRIGGER aaa_guild_resign_test_barrier ON game_guild_members;DROP FUNCTION guild_resign_test_barrier();").await?;
        // Current newly elected leader attempts the reverse swap after delivery, with no Current
        // snapshot for either Account. Both real decisions are required; persisted positions grant none.
        let mut reverse =
            GuildResignRequest::new(request(2, 1, GuildCommand::Resign(character(0))), 2)
                .map_err(debug)?;
        reverse.common.command = CommandRef::new(target_fence.game_session_id, CommandId::new(2)?);
        let before = rank_mutation_snapshot(h).await?;
        assert_eq!(
            result(
                h.root
                    .commit_guild_resign(
                        &authority,
                        &h.node,
                        target_fence,
                        reverse,
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
                .commit_guild_resign(&authority, &h.node, fence()?, req, &pre_delivery(&h.root))
                .await
                .map_err(debug)?,
            GuildOutcome::Replayed(_)
        ));
        assert_eq!(h.count("game_guild_operations").await?, 2);
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        Ok(())
    })
}
