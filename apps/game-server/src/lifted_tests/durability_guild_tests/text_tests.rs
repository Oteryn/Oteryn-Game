use super::*;
#[test]
fn guild_text_native_parser_bounds_empty_title_and_closed_audit() -> TestResult {
    let title = "🦀".repeat(29);
    assert_eq!(
        text_request(1, GuildCommand::SetTitle(character(0)), 1, &title).text,
        title
    );
    assert_eq!(
        text_request(1, GuildCommand::SetTitle(character(0)), 1, "").text,
        ""
    );
    assert!(
        GuildTextRequest::new(
            request(1, 1, GuildCommand::SetTitle(character(0))),
            1,
            &"x".repeat(30)
        )
        .is_err()
    );
    assert!(
        GuildTextRequest::new(
            request(1, 1, GuildCommand::SetTitle(character(0))),
            1,
            "a\0b"
        )
        .is_err()
    );
    for input in ["", "   ", "control\nline", "\u{0085}"] {
        assert!(GuildTextRequest::new(request(1, 1, GuildCommand::SetMessage), 1, input).is_err());
    }
    assert!(
        GuildTextRequest::new(request(1, 1, GuildCommand::SetMessage), 1, &"x".repeat(256))
            .is_err()
    );
    assert!(
        GuildTextRequest::new(
            request(1, 1, GuildCommand::SetMessage),
            1,
            &format!("{}x", " ".repeat(1020))
        )
        .is_err()
    );
    assert!(GuildTextRequest::new(request(1, 1, GuildCommand::SetMessage), 0, "hello").is_err());
    assert!(GuildTextRequest::new(request(1, 1, GuildCommand::Leave), 1, "hello").is_err());
    assert_eq!(
        text_request(1, GuildCommand::SetMessage, 1, "  hello  ").text,
        "hello"
    );
    let text = "🦀".repeat(255);
    let req = text_request(1, GuildCommand::SetMessage, 1, &text);
    let mut audit = GuildTextAuditProposal {
        operation: GuildAuditProposal {
            version: 1,
            occurrence: req.common.identity.occurrence,
            transaction_id: req.common.identity.transaction_id,
            event_id: req.common.identity.event_id,
            guild_id: *gid(1).as_bytes(),
            world_id: id(WORLD),
            actor: *character(0).as_bytes(),
            target: *character(0).as_bytes(),
            action: 8,
            previous_revision: 1,
            revision: 2,
            rank_before: None,
            rank_after: None,
            removed_invitations: 0,
            occurred_at_unix_ms: 1,
        },
        expected_revision: 1,
        text,
    };
    let bytes = audit
        .checked_bytes()
        .ok_or("maximal valid message audit rejected")?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(value["text"].as_str(), Some(req.text.as_str()));
    assert_eq!(value["expected_revision"], 1);
    assert!(
        audit.operation.checked_bytes().is_none(),
        "old codec still refuses text-less new arms"
    );
    audit.expected_revision = 2;
    assert!(audit.checked_bytes().is_none());
    audit.expected_revision = 1;
    audit.operation.target = *character(90).as_bytes();
    assert!(audit.checked_bytes().is_none());
    audit.operation.target = *character(0).as_bytes();
    audit.text = " padded ".into();
    assert!(audit.checked_bytes().is_none());
    Ok(())
}
#[test]
fn guild_text_native_found_source_exact_title_message_and_clear_title() -> TestResult {
    text_run("guild_text_found", async |h| {
        guard(h).await?;
        assert_eq!(
            result(
                found(
                    h,
                    found_request(100, 1, "Text Guild"),
                    fence()?,
                    &pre_delivery(&h.root)
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        let title = "🦀".repeat(29);
        let message = "🦀".repeat(255);
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(101, GuildCommand::SetTitle(character(0)), 1, &title)
                )
                .await?
            )
            .revision,
            Some(2)
        );
        assert_eq!(
            result(text_apply(h, text_request(102, GuildCommand::SetMessage, 2, &message)).await?)
                .revision,
            Some(3)
        );
        let actual:(String,String,String,Option<i64>)=sqlx::query_as("SELECT motd,revision::text,title,vice_deficit_since FROM game_guilds JOIN game_guild_members USING(guild_id)").fetch_one(&h.pool).await?;
        assert_eq!(actual, (message, "3".into(), title, None));
        let op:(String,String,String)=sqlx::query_as("SELECT expected_revision::text,edit_text,revision::text FROM game_guild_operations WHERE action=8").fetch_one(&h.pool).await?;
        assert_eq!(op, ("2".into(), "🦀".repeat(255), "3".into()));
        let payload:Vec<u8>=sqlx::query_scalar("SELECT payload FROM game_guild_event_outbox e JOIN game_guild_operations o USING(occurrence) WHERE o.action=8").fetch_one(&h.pool).await?;
        let audit: serde_json::Value = serde_json::from_slice(&payload)?;
        assert_eq!(audit["text"].as_str(), Some("🦀".repeat(255).as_str()));
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(103, GuildCommand::SetTitle(character(0)), 3, "")
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        let title: String = sqlx::query_scalar("SELECT title FROM game_guild_members")
            .fetch_one(&h.pool)
            .await?;
        assert!(title.is_empty());
        assert_eq!(h.count("game_guild_operations").await?, 4);
        assert_eq!(h.count("game_guild_event_outbox").await?, 4);
        assert_eq!(text_character_revision(h).await?, "1");
        Ok(())
    })
}
#[test]
fn guild_text_native_current_role_target_and_world_are_read_not_supplied() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        for role in 0..5 {
            let h=Harness::create(admin.clone(),"guild_text_roles").await?;
            let outcome: TestResult<()> = async {
                prepare(&h).await?;historical_character(&h,90,id(WORLD)).await?;historical_character(&h,91,id(WORLD)).await?;
                initial_guild(&h,1,if role==0 {0}else{90}).await?;
                initial_member(&h,1,91,3).await?;
                if role==1 || role==2 {initial_member(&h,1,0,if role==1 {2}else{3}).await?;}
                if role==4 {initial_guild(&h,2,0).await?;}
                guard(&h).await?;
                let title=result(text_apply(&h,text_request(1,GuildCommand::SetTitle(character(91)),1,"Target Title")).await?);
                assert_eq!(title.result,if role==0{GuildResult::Ok}else{GuildResult::NotAllowed});
                let revision=if role==0{2}else{1};
                let message=result(text_apply(&h,text_request(2,GuildCommand::SetMessage,revision,"Guild Message")).await?);
                assert_eq!(message.result,if role<=1{GuildResult::Ok}else{GuildResult::NotAllowed});
                let before=text_snapshot(&h).await?;
                let seal=h.recovery.seal_current().map_err(debug)?;let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
                let mut missing=text_request(3,GuildCommand::SetTitle(character(999)),revision,"Unknown Member");
                if role==0 {missing.expected_revision=3;}
                assert_eq!(result(h.root.commit_guild_text(&authority,&h.node,fence()?,missing).await.map_err(debug)?).result,GuildResult::NotAllowed);
                let title:String=sqlx::query_scalar("SELECT title FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid").bind(character(91).as_bytes().as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(title,if role==0{"Target Title"}else{""});
                // Same immutable requested Guild/actor under a foreign expected World never grants current authority.
                let mut foreign=fence()?;foreign.runtime_scope=crate::foundation::RuntimeScopeRefV1::channel(crate::foundation::WorldId::decode(&sid(7,91))?,crate::foundation::ChannelId::decode(&id(fixture::CHANNEL))?);
                assert!(matches!(h.root.commit_guild_text(&authority,&h.node,foreign,text_request(4,GuildCommand::SetMessage,revision,"No Foreign World")).await,Err(GuildError::NotCurrent)));
                assert_eq!(text_character_revision(&h).await?,"1");
                // Ordinary refusal receipts are expected; no foreign/current-authority error writes another receipt.
                assert_eq!(h.count("game_guild_event_outbox").await?,if role==0{2}else if role==1{1}else{0});
                assert_ne!(text_snapshot(&h).await?,before,"typed missing-target refusal is receipted");
                Ok(())
            }.await;
            h.cleanup().await?;outcome?;
        }
        for state in [3_i16,4] {
            let h=Harness::create(admin.clone(),"guild_text_closed_state").await?;
            let outcome: TestResult<()> = async {
                prepare(&h).await?;initial_guild(&h,1,0).await?;
                // A historical DISBANDED snapshot has no remaining members/positions.
                if state==4 {h.pool.execute("DELETE FROM game_account_guild_leadership;DELETE FROM game_guild_members").await?;}
                sqlx::query("UPDATE game_guilds SET state=$1").bind(state).execute(&h.pool).await?;
                guard(&h).await?;
                let before:String=sqlx::query_scalar("SELECT to_jsonb(g)::text FROM game_guilds g").fetch_one(&h.pool).await?;
                for (n,op) in [(1,GuildCommand::SetTitle(character(0))),(2,GuildCommand::SetMessage)] {
                    assert_eq!(result(text_apply(&h,text_request(n,op,1,"Closed Guild")).await?).result,GuildResult::GuildDisbanding);
                    let after:String=sqlx::query_scalar("SELECT to_jsonb(g)::text FROM game_guilds g").fetch_one(&h.pool).await?;
                    assert_eq!(after,before);
                }
                assert_eq!(h.count("game_guild_operations").await?,2);assert_eq!(h.count("game_guild_event_outbox").await?,0);assert_eq!(text_character_revision(&h).await?,"1");
                Ok(())
            }.await;
            h.cleanup().await?;outcome?;
        }
        Ok(())
    })
}
#[test]
fn guild_text_native_revision_cas_receipt_binding_and_restart_replay() -> TestResult {
    text_run("guild_text_receipt", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        let original = text_request(1, GuildCommand::SetMessage, 1, "Original Message");
        let committed = result(text_apply(h, original.clone()).await?);
        assert_eq!(committed.result, GuildResult::Ok);
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(2, GuildCommand::SetTitle(character(0)), 1, "Stale")
                )
                .await?
            )
            .result,
            GuildResult::StaleRevision
        );
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(3, GuildCommand::SetMessage, 2, "Newer Message")
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        let title = text_request(4, GuildCommand::SetTitle(character(0)), 3, "Bound Target");
        assert_eq!(
            result(text_apply(h, title.clone()).await?).result,
            GuildResult::Ok
        );
        let before = text_snapshot(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut changed_target = title.clone();
        changed_target.common.operation = GuildCommand::SetTitle(character(90));
        assert!(matches!(
            h.root
                .commit_guild_text(&authority, &h.node, fence()?, changed_target)
                .await,
            Err(GuildError::ConflictingOccurrence)
        ));
        let mut changed_command = title.clone();
        changed_command.common.identity =
            request(5, 1, GuildCommand::SetTitle(character(0))).identity;
        assert!(matches!(
            h.root
                .commit_guild_text(&authority, &h.node, fence()?, changed_command)
                .await,
            Err(GuildError::ConflictingOccurrence)
        ));
        assert_eq!(text_snapshot(h).await?, before);
        for mutation in 0..4 {
            let mut changed = original.clone();
            match mutation {
                0 => changed.text = "Changed".into(),
                1 => changed.expected_revision = 2,
                2 => changed.common.operation = GuildCommand::SetTitle(character(0)),
                _ => changed.common.guild_id = gid(2),
            }
            assert!(matches!(
                h.root
                    .commit_guild_text(&authority, &h.node, fence()?, changed)
                    .await,
                Err(GuildError::ConflictingOccurrence)
            ));
            assert_eq!(text_snapshot(h).await?, before);
        }
        assert!(matches!(
            h.root
                .commit_guild_membership(
                    &authority,
                    &h.node,
                    fence()?,
                    request(4, 1, GuildCommand::SetMessage)
                )
                .await,
            Err(GuildError::InvalidInput)
        ));
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let recovered = other.open_character_authority(&seal).await.map_err(debug)?;
        let replay = other
            .commit_guild_text(&recovered, &h.node, fence()?, original)
            .await
            .map_err(debug)?;
        assert!(matches!(&replay, GuildOutcome::Replayed(_)));
        assert_eq!(result(replay), committed);
        assert_eq!(text_snapshot(h).await?, before);
        drop(recovered);
        drop(other);
        Ok(())
    })
}
#[test]
fn guild_text_native_both_boundaries_current_fences_revoke_and_recovery_first() -> TestResult {
    text_run("guild_text_fences", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        let first = text_request(1, GuildCommand::SetTitle(character(0)), 1, "Current Title");
        text_apply(h, first.clone()).await?;
        let second = text_request(2, GuildCommand::SetMessage, 2, "Current Message");
        text_apply(h, second.clone()).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let before = text_snapshot(h).await?;
        // Bootstrap authorization is one-launch/one-use. Register the independently
        // current foreign Node once, then reuse that genuine proof for both boundaries.
        let wrong = fixture::register(&h.root, 2).await?;
        for op in [
            GuildCommand::SetTitle(character(0)),
            GuildCommand::SetMessage,
        ] {
            let req = text_request(3, op, 3, "Fresh Edit");
            for mutation in 0..4 {
                let mut f = fence()?;
                match mutation {
                    0 => f.connection_generation = ConnectionGeneration::new(2)?,
                    1 => f.character_lease_generation = 2,
                    2 => f.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
                    _ => f.character_id = character(999),
                }
                assert!(matches!(
                    h.root
                        .commit_guild_text(&authority, &h.node, f, req.clone())
                        .await,
                    Err(GuildError::NotCurrent)
                ));
                assert_eq!(text_snapshot(h).await?, before);
            }
            assert!(matches!(
                h.root
                    .commit_guild_text(&authority, &wrong, fence()?, req.clone())
                    .await,
                Err(GuildError::NotCurrent)
            ));
            assert_eq!(text_snapshot(h).await?, before);
        }
        use crate::durability::runtime_scope_assignment::{
            AssignmentCommand, AssignmentOutcome, AssignmentRequest, AssignmentState, ControlActor,
            OperationKey, RuntimeScopeAssignmentWriter,
        };
        // The native fixture grants only Assign (operation1). Revoke must have its
        // own exact-scope operation3 grant for the same authenticated DB principal.
        let actor: String = sqlx::query_scalar("SELECT session_user")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(
            actor, "oteryn_test_admin",
            "native fixture's writer session principal"
        );
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,3)")
            .bind(id(WORLD).as_slice()).bind(id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
        // Root lifetime is already bound to this native fixture writer registration.
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
                operation_key: OperationKey::from_bytes([87; 32]),
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
        for op in [
            GuildCommand::SetTitle(character(0)),
            GuildCommand::SetMessage,
        ] {
            assert!(matches!(
                h.root
                    .commit_guild_text(
                        &authority,
                        &h.node,
                        fence()?,
                        text_request(3, op, 3, "Revoked")
                    )
                    .await,
                Err(GuildError::NotCurrent)
            ));
        }
        for req in [first, second] {
            assert!(matches!(
                h.root
                    .commit_guild_text(&authority, &h.node, fence()?, req)
                    .await
                    .map_err(debug)?,
                GuildOutcome::Replayed(_)
            ));
        }
        assert_eq!(text_snapshot(h).await?, before);
        let wrong_root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(wrong_root.maintain_ready_once().await?);
        assert!(matches!(
            wrong_root
                .commit_guild_text(
                    &authority,
                    &h.node,
                    fence()?,
                    text_request(1, GuildCommand::SetTitle(character(0)), 1, "Current Title")
                )
                .await,
            Err(GuildError::NotCurrent)
        ));
        // Sealed-store generation locks deliberately prevent manufacturing a stale opaque
        // Root capability. Negative expected recovery DATA exercises the SAME production
        // transaction prelude before its historical receipt, without changing native rows.
        let mut wrong_recovery = authority.record_for(&h.root).map_err(debug)?;
        wrong_recovery.recovery_generation += 1;
        let req = text_request(1, GuildCommand::SetTitle(character(0)), 1, "Current Title");
        let mut digest = Sha256::new();
        digest.update(b"oteryn:guild-text-proposal:v1");
        digest.update(binding(&fence()?, &req.common).map_err(debug)?);
        digest.update(req.expected_revision.to_be_bytes());
        digest.update(req.text.as_bytes());
        let binding = digest.finalize().to_vec();
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            execute_fenced_operation(
                &mut tx,
                &wrong_recovery,
                &h.node,
                &fence()?,
                &req.common,
                &binding,
                Some(&req)
            )
            .await,
            Err(GuildError::Unavailable(_))
        ));
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        drop(wrong_root);
        Ok(())
    })
}
#[test]
fn guild_text_native_closed_field_proofs_deferred_failure_and_overflow_rollback() -> TestResult {
    text_run("guild_text_rollback", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        h.pool.execute("CREATE FUNCTION guild_text_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'injected text event failure' USING ERRCODE='23514';END$$;CREATE CONSTRAINT TRIGGER guild_text_fault AFTER INSERT ON game_guild_event_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION guild_text_fault()").await?;
        for op in [
            GuildCommand::SetTitle(character(0)),
            GuildCommand::SetMessage,
        ] {
            let before = text_snapshot(h).await?;
            assert!(
                text_apply(h, text_request(1, op, 1, "Atomic Text"))
                    .await
                    .is_err()
            );
            assert_eq!(text_snapshot(h).await?, before);
        }
        h.pool.execute("DROP TRIGGER guild_text_fault ON game_guild_event_outbox;DROP FUNCTION guild_text_fault()").await?;
        h.pool.execute("CREATE FUNCTION guild_text_wrong_field() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN NEW.vice_deficit_since=999;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_text_wrong_field BEFORE UPDATE ON game_guilds FOR EACH ROW EXECUTE FUNCTION guild_text_wrong_field()").await?;
        for op in [
            GuildCommand::SetTitle(character(0)),
            GuildCommand::SetMessage,
        ] {
            let before = text_snapshot(h).await?;
            assert!(
                text_apply(h, text_request(1, op, 1, "No Unrelated Field"))
                    .await
                    .is_err()
            );
            assert_eq!(text_snapshot(h).await?, before);
        }
        h.pool.execute("DROP TRIGGER aaa_guild_text_wrong_field ON game_guilds;DROP FUNCTION guild_text_wrong_field()").await?;
        let denied = sqlx::query("UPDATE game_guild_members SET title='unproved'")
            .execute(&h.pool)
            .await
            .expect_err("no current-xid operation");
        assert_eq!(
            denied.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(1, GuildCommand::SetTitle(character(0)), 1, "Atomic Title")
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(2, GuildCommand::SetMessage, 2, "Atomic Message")
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        assert_eq!(h.count("game_guild_operations").await?, 2);
        assert_eq!(h.count("game_guild_event_outbox").await?, 2);
        assert_eq!(text_character_revision(h).await?, "1");
        Ok(())
    })?;
    text_run("guild_text_overflow", async |h| {
        initial_guild(h, 1, 0).await?;
        sqlx::query("UPDATE game_guilds SET revision=18446744073709551615")
            .execute(&h.pool)
            .await?;
        guard(h).await?;
        for op in [
            GuildCommand::SetTitle(character(0)),
            GuildCommand::SetMessage,
        ] {
            let before = text_snapshot(h).await?;
            assert!(
                text_apply(h, text_request(1, op, u64::MAX, "Overflow"))
                    .await
                    .is_err()
            );
            assert_eq!(text_snapshot(h).await?, before);
        }
        Ok(())
    })
}
#[test]
fn guild_text_native_two_root_revision_and_occurrence_serialization() -> TestResult {
    text_run("guild_text_race", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let first = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let second = other.open_character_authority(&seal).await.map_err(debug)?;
        let a = text_request(1, GuildCommand::SetTitle(character(0)), 1, "Racing Title");
        let b = text_request(2, GuildCommand::SetMessage, 1, "Racing Message");
        let (a, b) = fixture::join_two(
            h.root.commit_guild_text(&first, &h.node, fence()?, a),
            other.commit_guild_text(&second, &h.node, fence()?, b),
        )
        .await;
        let results = [result(a.map_err(debug)?), result(b.map_err(debug)?)];
        assert_eq!(
            results
                .iter()
                .filter(|r| r.result == GuildResult::Ok)
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|r| r.result == GuildResult::StaleRevision)
                .count(),
            1
        );
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        assert_eq!(h.count("game_guild_operations").await?, 2);
        let same = text_request(3, GuildCommand::SetMessage, 2, "One Exact Edit");
        let (a, b) = fixture::join_two(
            h.root
                .commit_guild_text(&first, &h.node, fence()?, same.clone()),
            other.commit_guild_text(&second, &h.node, fence()?, same),
        )
        .await;
        let a = a.map_err(debug)?;
        let b = b.map_err(debug)?;
        assert_eq!(
            usize::from(matches!(&a, GuildOutcome::Committed(_)))
                + usize::from(matches!(&b, GuildOutcome::Committed(_))),
            1
        );
        assert_eq!(result(a), result(b));
        assert_eq!(h.count("game_guild_event_outbox").await?, 2);
        assert_eq!(h.count("game_guild_operations").await?, 3);
        assert_eq!(text_character_revision(h).await?, "1");
        // The unchanged native admission relation locks serialize full passes too; this
        // proves two genuine Root results, not isolated row-lock/load performance.
        drop(second);
        drop(first);
        drop(other);
        Ok(())
    })
}
