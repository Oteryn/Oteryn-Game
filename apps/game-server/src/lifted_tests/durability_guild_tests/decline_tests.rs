use super::*;
#[test]
fn guild_decline_native_actor_target_binding_and_closed_codec() -> TestResult {
    let req = request(1, 1, GuildCommand::Decline);
    assert_eq!(req.operation.target(character(0)), character(0));
    for op in [
        GuildCommand::Accept,
        GuildCommand::Revoke(character(0)),
        GuildCommand::Leave,
    ] {
        assert_ne!(
            binding(&fence()?, &req).map_err(debug)?,
            binding(&fence()?, &request(1, 1, op)).map_err(debug)?
        );
    }
    let valid = decline_test_audit(&req);
    let bytes = valid
        .checked_bytes()
        .ok_or("valid Decline audit rejected")?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(value["removed_invitations"], 1);
    assert_eq!(value["action"], 12);
    assert!(
        valid.operation.checked_bytes().is_none(),
        "old codec must remain closed"
    );
    for field in 0..5 {
        let mut changed = valid.clone();
        match field {
            0 => changed.operation.target = *character(92).as_bytes(),
            1 => changed.operation.removed_invitations = 0,
            2 => changed.operation.rank_before = Some(1),
            3 => changed.operation.revision = 3,
            _ => changed.operation.action = 2,
        }
        assert!(changed.checked_bytes().is_none());
    }
    Ok(())
}
#[test]
fn guild_decline_native_only_own_selected_invitation_even_expired_and_other_guild_member()
-> TestResult {
    for expired in [false, true] {
        text_run("guild_decline_own", async |h| {
            decline_fixture(h, expired).await?;
            guard(h).await?;
            let original = decline_non_invitation_fields(h).await?;
            let characters = resign_resource_character_revisions(h).await?;
            let mut expected = decline_invitation_rows(h).await?;
            expected.retain(|(g, c, _, _)| {
                *g != uuid(*gid(1).as_bytes()) || *c != uuid(*character(0).as_bytes())
            });
            // Actor leads a DIFFERENT Guild: that never grants this Guild's Revoke permission.
            assert_eq!(
                result(
                    apply(
                        h,
                        request(1, 1, GuildCommand::Revoke(character(0))),
                        fence()?
                    )
                    .await?
                )
                .result,
                GuildResult::NotAllowed
            );
            assert_eq!(
                result(apply(h, request(2, 1, GuildCommand::Decline), fence()?).await?).result,
                GuildResult::Ok
            );
            assert_eq!(decline_invitation_rows(h).await?, expected);
            assert_eq!(decline_non_invitation_fields(h).await?, original);
            assert_eq!(resign_resource_character_revisions(h).await?, characters);
            assert_eq!(revision(h, 1).await?, "2");
            for guild in [2, 3, 4] {
                assert_eq!(revision(h, guild).await?, "1");
            }
            let unchanged = rank_mutation_snapshot(h).await?;
            assert_eq!(
                result(apply(h, request(3, 1, GuildCommand::Decline), fence()?).await?).result,
                GuildResult::NotInvited
            );
            assert_eq!(rank_mutation_snapshot(h).await?, unchanged);
            assert_eq!(h.count("game_guild_operations").await?, 3);
            assert_eq!(h.count("game_guild_event_outbox").await?, 1);
            Ok(())
        })?;
    }
    Ok(())
}
#[test]
fn guild_decline_native_replay_binding_and_two_root_single_effect() -> TestResult {
    text_run("guild_decline_history", async |h| {
        decline_fixture(h, false).await?;
        guard(h).await?;
        let req = request(1, 1, GuildCommand::Decline);
        let first = result(apply(h, req.clone(), fence()?).await?);
        assert_eq!(first.result, GuildResult::Ok);
        let before = text_snapshot(h).await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let b = other.open_character_authority(&seal).await.map_err(debug)?;
        let mut stale_fence = fence()?;
        stale_fence.connection_generation = ConnectionGeneration::new(999)?;
        let replay = other
            .commit_guild_membership(&b, &h.node, stale_fence, req.clone())
            .await
            .map_err(debug)?;
        assert!(matches!(&replay, GuildOutcome::Replayed(_)));
        assert_eq!(result(replay), first);
        for field in 0..3 {
            let mut changed = req.clone();
            match field {
                0 => changed.guild_id = gid(3),
                1 => changed.operation = GuildCommand::Revoke(character(0)),
                _ => changed.command = request(10, 1, GuildCommand::Decline).command,
            }
            assert!(matches!(
                other
                    .commit_guild_membership(&b, &h.node, fence()?, changed)
                    .await,
                Err(GuildError::ConflictingOccurrence)
            ));
            assert_eq!(text_snapshot(h).await?, before);
        }
        let mut recovery = a.record_for(&h.root).map_err(debug)?;
        recovery.recovery_generation += 1;
        let mut tx = h.pool.begin().await?;
        assert!(
            lock_fenced_guild_operation(&mut tx, &recovery, &req)
                .await
                .is_err()
        );
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        let same = request(2, 3, GuildCommand::Decline);
        let (x, y) = fixture::join_two(
            h.root
                .commit_guild_membership(&a, &h.node, fence()?, same.clone()),
            other.commit_guild_membership(&b, &h.node, fence()?, same),
        )
        .await;
        let (x, y) = (x.map_err(debug)?, y.map_err(debug)?);
        assert_eq!(
            usize::from(matches!(&x, GuildOutcome::Committed(_)))
                + usize::from(matches!(&y, GuildOutcome::Committed(_))),
            1
        );
        assert_eq!(result(x), result(y));
        let (x, y) = fixture::join_two(
            h.root.commit_guild_membership(
                &a,
                &h.node,
                fence()?,
                request(3, 4, GuildCommand::Decline),
            ),
            other.commit_guild_membership(
                &b,
                &h.node,
                fence()?,
                request(4, 4, GuildCommand::Decline),
            ),
        )
        .await;
        let answers = [
            result(x.map_err(debug)?).result,
            result(y.map_err(debug)?).result,
        ];
        assert_eq!(
            answers
                .iter()
                .filter(|value| **value == GuildResult::Ok)
                .count(),
            1
        );
        assert_eq!(
            answers
                .iter()
                .filter(|value| **value == GuildResult::NotInvited)
                .count(),
            1
        );
        assert_eq!(h.count("game_guild_operations").await?, 4);
        assert_eq!(h.count("game_guild_event_outbox").await?, 3);
        Ok(())
    })
}
#[test]
fn guild_decline_native_current_session_node_and_physical_tx_guards() -> TestResult {
    text_run("guild_decline_fences", async |h| {
        decline_fixture(h, false).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let recovery = authority.record_for(&h.root).map_err(debug)?;
        let req = request(1, 1, GuildCommand::Decline);
        let before = text_snapshot(h).await?;
        let wrong = fixture::register(&h.root, 2).await?;
        for field in 0..8 {
            let mut stale = fence()?;
            match field {
                0 => stale.connection_generation = ConnectionGeneration::new(2)?,
                1 => stale.character_lease_generation = 2,
                2 => stale.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
                3 => stale.character_id = character(999),
                4 => stale.game_session_id = GameSessionId::decode(&sid(999, 20))?,
                5 => {
                    if let crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } =
                        stale.runtime_scope
                    {
                        stale.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
                            crate::foundation::WorldId::decode(&sid(999, 70))?,
                            channel_id,
                        );
                    }
                }
                6 => {
                    if let crate::foundation::RuntimeScopeRefV1::Channel { world_id, .. } =
                        stale.runtime_scope
                    {
                        stale.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
                            world_id,
                            crate::foundation::ChannelId::decode(&sid(999, 71))?,
                        );
                    }
                }
                _ => {}
            }
            let node = if field == 7 { &wrong } else { &h.node };
            let mut case_req = req.clone();
            if field == 4 {
                // Keep DATA binding internally consistent to reach the real current-session guard.
                case_req.command = CommandRef::new(stale.game_session_id, req.command.command_id());
            }
            assert!(matches!(
                h.root
                    .commit_guild_membership(&authority, node, stale, case_req.clone())
                    .await,
                Err(GuildError::NotCurrent)
            ));
            let mut tx = h.pool.begin().await?;
            lock_fenced_guild_operation(&mut tx, &recovery, &case_req)
                .await
                .map_err(debug)?;
            assert!(matches!(
                execute(
                    &mut tx,
                    node,
                    &stale,
                    &case_req,
                    &binding(&stale, &case_req).map_err(debug)?,
                    None,
                    None
                )
                .await,
                Err(GuildError::NotCurrent)
            ));
            tx.rollback().await?;
            assert_eq!(text_snapshot(h).await?, before);
        }
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        assert!(matches!(
            other
                .commit_guild_membership(&authority, &h.node, fence()?, req.clone())
                .await,
            Err(GuildError::NotCurrent)
        ));
        assert_eq!(text_snapshot(h).await?, before);
        assert_eq!(
            result(apply(h, req, fence()?).await?).result,
            GuildResult::Ok
        );
        let before = text_snapshot(h).await?;
        let fresh = request(2, 3, GuildCommand::Decline);
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
                    operation_key: OperationKey::from_bytes([91; 32]),
                    actor: ControlActor::new(&actor).map_err(debug)?,
                    command: AssignmentCommand::Revoke {
                        scope: fence()?.runtime_scope,
                        predecessor
                    },
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
                .commit_guild_membership(&authority, &h.node, fence()?, fresh.clone())
                .await,
            Err(GuildError::NotCurrent)
        ));
        let mut tx = h.pool.begin().await?;
        lock_fenced_guild_operation(&mut tx, &recovery, &fresh)
            .await
            .map_err(debug)?;
        assert!(matches!(
            execute(
                &mut tx,
                &h.node,
                &fence()?,
                &fresh,
                &binding(&fence()?, &fresh).map_err(debug)?,
                None,
                None
            )
            .await,
            Err(GuildError::NotCurrent)
        ));
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}
#[test]
fn guild_decline_native_terminal_guild_state_never_cleans_invitation() -> TestResult {
    for state in [3_i16, 4] {
        text_run("guild_decline_state", async |h| {
            decline_fixture(h, false).await?;
            sqlx::query("UPDATE game_guilds SET state=$1 WHERE guild_id=encode($2,'hex')::uuid")
                .bind(state)
                .bind(gid(1).as_bytes().as_slice())
                .execute(&h.pool)
                .await?;
            if state == 4 {
                // Valid terminal Guild has no membership or invitation left; no fake Hall/Bank fact.
                sqlx::query("DELETE FROM game_account_guild_leadership WHERE guild_id=encode($1,'hex')::uuid").bind(gid(1).as_bytes().as_slice()).execute(&h.pool).await?;
                sqlx::query("DELETE FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid")
                    .bind(gid(1).as_bytes().as_slice())
                    .execute(&h.pool)
                    .await?;
                sqlx::query(
                    "DELETE FROM game_guild_invitations WHERE guild_id=encode($1,'hex')::uuid",
                )
                .bind(gid(1).as_bytes().as_slice())
                .execute(&h.pool)
                .await?;
            }
            guard(h).await?;
            let before = rank_mutation_snapshot(h).await?;
            assert_eq!(
                result(apply(h, request(1, 1, GuildCommand::Decline), fence()?).await?).result,
                GuildResult::GuildDisbanding
            );
            assert_eq!(rank_mutation_snapshot(h).await?, before);
            assert_eq!(h.count("game_guild_event_outbox").await?, 0);
            Ok(())
        })?;
    }
    Ok(())
}
#[test]
fn guild_decline_native_sql_delete_and_event_failure_are_atomic() -> TestResult {
    text_run("guild_decline_atomic", async |h| {
        decline_fixture(h, false).await?;
        guard(h).await?;
        let before = text_snapshot(h).await?;
        let raw = sqlx::query("DELETE FROM game_guild_invitations WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid")
            .bind(gid(1).as_bytes().as_slice()).bind(character(0).as_bytes().as_slice()).execute(&h.pool).await.expect_err("unbound invitation delete accepted");
        assert_eq!(
            raw.as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some("23514")
        );
        for (setup, cleanup) in [
            (
                "CREATE FUNCTION guild_decline_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN RETURN NULL;END$$;CREATE TRIGGER aaa_guild_decline_test_fault BEFORE DELETE ON game_guild_invitations FOR EACH ROW EXECUTE FUNCTION guild_decline_test_fault();",
                "DROP TRIGGER aaa_guild_decline_test_fault ON game_guild_invitations;DROP FUNCTION guild_decline_test_fault();",
            ),
            (
                "CREATE FUNCTION guild_decline_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'native deferred event failure' USING ERRCODE='23514';END$$;CREATE CONSTRAINT TRIGGER guild_decline_test_fault AFTER INSERT ON game_guild_event_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION guild_decline_test_fault();",
                "DROP TRIGGER guild_decline_test_fault ON game_guild_event_outbox;DROP FUNCTION guild_decline_test_fault();",
            ),
            (
                "CREATE FUNCTION guild_decline_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.vice_deficit_since=99;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_decline_test_fault BEFORE UPDATE ON game_guilds FOR EACH ROW EXECUTE FUNCTION guild_decline_test_fault();",
                "DROP TRIGGER aaa_guild_decline_test_fault ON game_guilds;DROP FUNCTION guild_decline_test_fault();",
            ),
        ] {
            h.pool.execute(setup).await?;
            assert!(
                apply(h, request(1, 1, GuildCommand::Decline), fence()?)
                    .await
                    .is_err()
            );
            assert_eq!(text_snapshot(h).await?, before);
            h.pool.execute(cleanup).await?;
        }
        h.pool.execute("CREATE TABLE guild_decline_test_payload(payload BYTEA NOT NULL);CREATE FUNCTION guild_decline_test_op() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN SELECT sha256(payload) INTO NEW.audit_sha256 FROM guild_decline_test_payload;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_decline_test_op BEFORE INSERT ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION guild_decline_test_op();CREATE FUNCTION guild_decline_test_event() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN SELECT payload INTO NEW.payload FROM guild_decline_test_payload;NEW.payload_sha256=sha256(NEW.payload);RETURN NEW;END$$;CREATE TRIGGER aaa_guild_decline_test_event BEFORE INSERT ON game_guild_event_outbox FOR EACH ROW EXECUTE FUNCTION guild_decline_test_event();").await?;
        let valid =
            serde_json::to_value(decline_test_audit(&request(1, 1, GuildCommand::Decline)))?;
        for field in 0..5 {
            let mut changed = valid.clone();
            match field {
                0 => changed["target"] = serde_json::to_value(character(92).as_bytes())?,
                1 => changed["removed_invitations"] = serde_json::json!(0),
                2 => changed["action"] = serde_json::json!(2),
                3 => changed["occurred_at_unix_ms"] = serde_json::json!(1),
                _ => changed["caller_is_invited"] = serde_json::json!(true),
            }
            sqlx::query("DELETE FROM guild_decline_test_payload")
                .execute(&h.pool)
                .await?;
            sqlx::query("INSERT INTO guild_decline_test_payload VALUES($1)")
                .bind(serde_json::to_vec(&changed)?)
                .execute(&h.pool)
                .await?;
            assert!(
                apply(h, request(1, 1, GuildCommand::Decline), fence()?)
                    .await
                    .is_err()
            );
            assert_eq!(text_snapshot(h).await?, before);
        }
        h.pool.execute("DROP TRIGGER aaa_guild_decline_test_op ON game_guild_operations;DROP TRIGGER aaa_guild_decline_test_event ON game_guild_event_outbox;DROP FUNCTION guild_decline_test_op();DROP FUNCTION guild_decline_test_event();DROP TABLE guild_decline_test_payload;").await?;
        assert_eq!(
            result(apply(h, request(1, 1, GuildCommand::Decline), fence()?).await?).result,
            GuildResult::Ok
        );
        let committed = text_snapshot(h).await?;
        assert!(
            sqlx::query("UPDATE game_guild_operations SET target=actor WHERE action=12")
                .execute(&h.pool)
                .await
                .is_err()
        );
        assert!(sqlx::query("UPDATE game_guild_event_outbox SET payload=convert_to('{}','UTF8'),payload_sha256=sha256(convert_to('{}','UTF8'))").execute(&h.pool).await.is_err());
        assert_eq!(text_snapshot(h).await?, committed);
        assert_eq!(h.count("game_guild_operations").await?, 1);
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        Ok(())
    })
}
